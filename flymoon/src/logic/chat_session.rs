use super::{md, toast, tr::tr};
use crate::{
    config::{Model as SettingModel, model as setting_model},
    db::{CHAT_SESSION_TABLE as DB_TABLE, ChatSession, entry},
    global_logic, global_store, logic_cb,
    slint_generatedAppWindow::{
        AppWindow, ChatEntry as UIChatEntry, ChatPhase, ChatSession as UIChatSession,
    },
    toast_warn,
};
use bot::openai::{
    Chat,
    request::{APIConfig as ChatAPIConfig, HistoryChat},
    response::StreamTextItem,
};
use cutil::time::chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use uuid::Uuid;

struct ChatCache {
    id: u64,
    ui: Weak<AppWindow>,
    stop_tx: Arc<mpsc::Sender<()>>,
    reasoner_start: Option<DateTime<Utc>>,
    bot_text: String,
}

static INC_CHAT_ID: AtomicU64 = AtomicU64::new(0);
static CHAT_CACHE: Lazy<Mutex<Option<ChatCache>>> = Lazy::new(|| Mutex::new(None));

#[macro_export]
macro_rules! store_current_chat_session {
    ($ui:expr) => {
        crate::global_store!($ui).get_current_chat_session()
    };
}

#[macro_export]
macro_rules! store_current_chat_session_histories {
    ($ui:expr) => {
        crate::global_store!($ui)
            .get_current_chat_session()
            .histories
            .as_any()
            .downcast_ref::<VecModel<UIChatEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

pub fn init(ui: &AppWindow) {
    chat_session_init(ui);

    logic_cb!(new_chat_session, ui);
    logic_cb!(load_chat_session, ui, uuid);
    logic_cb!(send_question, ui, question);
    logic_cb!(stop_question, ui);
    logic_cb!(retry_question, ui, index, question);
    logic_cb!(copy_last_bot_text, ui);
    logic_cb!(remove_question, ui, index);
    logic_cb!(toggle_edit_question, ui, index);
    logic_cb!(toggle_hide_bot_reasoner, ui, index);
}

fn stop_question(_ui: &AppWindow) {
    tokio::spawn(async move {
        let mut cc = CHAT_CACHE.lock().unwrap();
        if let Some(cc) = cc.take() {
            _ = cc.stop_tx.send(());
        }
    });
}

fn parse_prompt(
    _ui: &AppWindow,
    question: SharedString,
) -> (SharedString, SharedString, Option<f32>) {
    // Use default system prompt
    let prompt = SharedString::from("You are a helpful AI assistant.");
    (prompt, question, None)
}

fn stream_text(id: u64, item: StreamTextItem) {
    if id != item.id {
        return;
    }

    let (cc_id, ui, reasoner_start) = {
        let cc = CHAT_CACHE.lock().unwrap();
        if cc.is_none() {
            return;
        }

        let cc = cc.as_ref().unwrap();
        (cc.id, cc.ui.clone(), cc.reasoner_start.clone())
    };

    if id != cc_id {
        return;
    }

    if item.text.is_some() {
        let mut cc = CHAT_CACHE.lock().unwrap();
        if cc.is_some() {
            cc.as_mut()
                .unwrap()
                .bot_text
                .push_str(&item.text.as_ref().unwrap());
        }
    }

    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        if item.etext.is_some() {
            toast_warn!(
                ui,
                format!(
                    "{}. {}: {}",
                    tr("Chat failed"),
                    tr("Reason"),
                    item.etext.unwrap()
                )
            );
            return;
        }

        if item.finished {
            md::parse_last_history_bot_text(&ui);
            update_db_entry(&ui);
            return;
        }

        let chat_phase = global_store!(ui).get_chat_phase();

        if item.reasoning_text.is_some() {
            if chat_phase != ChatPhase::Chatting {
                global_store!(ui).set_chat_phase(ChatPhase::Chatting);
            }

            let rows = store_current_chat_session_histories!(ui).row_count();
            if rows <= 0 {
                return;
            }

            let last_index = rows - 1;
            let mut entry = store_current_chat_session_histories!(ui)
                .row_data(last_index)
                .unwrap();

            if reasoner_start.is_some() {
                entry.reasoner_spending_seconds =
                    (Utc::now() - reasoner_start.unwrap()).num_seconds() as i32;
            }

            entry.bot_reasoner.push_str(&item.reasoning_text.unwrap());
            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
        }

        if item.text.is_some() {
            let rows = store_current_chat_session_histories!(ui).row_count();
            if rows <= 0 {
                return;
            }

            let last_index = rows - 1;
            let mut entry = store_current_chat_session_histories!(ui)
                .row_data(last_index)
                .unwrap();

            let text = item.text.unwrap();
            entry.bot.push_str(&text);
            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);

            if text.contains("\n") {
                if chat_phase != ChatPhase::Chatting {
                    global_store!(ui).set_chat_phase(ChatPhase::Chatting);
                }

                md::parse_stream_bot_text(&ui);
            }
        }
    });
}

fn chat_histories(ui: &AppWindow, question: SharedString) -> Vec<HistoryChat> {
    let mut session = store_current_chat_session!(ui);
    let (is_new_chat, histories) = if session.uuid.is_empty() {
        session.uuid = Uuid::new_v4().to_string().into();
        session.time = cutil::time::local_now("%m-%d %H:%M").into();
        global_store!(ui).set_current_chat_session(session);

        (true, vec![])
    } else {
        let histories = session
            .histories
            .iter()
            .map(|entry| entry.into())
            .collect::<Vec<HistoryChat>>();

        (false, histories)
    };

    store_current_chat_session_histories!(ui).push(UIChatEntry {
        user: question,
        md_elems: ModelRc::new(VecModel::from(vec![])),
        link_urls: ModelRc::new(VecModel::from(vec![])),
        ..Default::default()
    });

    if is_new_chat {
        add_db_entry(ui);
    }

    histories
}

fn prepare_chat(
    ui: Weak<AppWindow>,
    prompt: SharedString,
    question: SharedString,
    histories: Vec<HistoryChat>,
    temperature: Option<f32>,
    enabled_reasoner_model: bool,
) -> (Chat, u64) {
    async_update_chat_phase(ui.clone(), ChatPhase::Thinking);

    let mut config: ChatAPIConfig = setting_model().into();
    config.temperature = temperature;
    if enabled_reasoner_model {
        config.api_model = setting_model().chat.reasoner_model_name.into();
    }

    let (chat, stop_tx) = Chat::new(prompt, question, config, histories);
    let id = INC_CHAT_ID.fetch_add(1, Ordering::Relaxed);

    {
        let mut cc = CHAT_CACHE.lock().unwrap();
        *cc = Some(ChatCache {
            id,
            ui: ui.clone(),
            bot_text: String::default(),
            stop_tx: Arc::new(stop_tx),
            reasoner_start: if enabled_reasoner_model {
                Some(Utc::now())
            } else {
                None
            },
        });
    }

    (chat, id)
}

async fn start_chat(ui: Weak<AppWindow>, chat: Chat, id: u64) {
    match chat
        .start(id, |item| {
            stream_text(id, item);
        })
        .await
    {
        Err(e) => {
            toast::async_toast_warn(
                ui.clone(),
                format!("{}. {}: {e:?}", tr("Chat failed"), tr("Reason")),
            );
        }
        _ => {}
    }

    async_update_chat_phase(ui, ChatPhase::None);
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let (prompt, question, temperature) = parse_prompt(ui, question);
    let histories = chat_histories(ui, question.clone());

    let enabled_reasoner_model = global_store!(ui).get_enabled_reasoner_model();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        log::info!("start sending question to model...");
        let (chat, id) = prepare_chat(
            ui.clone(),
            prompt,
            question,
            histories,
            temperature,
            enabled_reasoner_model,
        );

        start_chat(ui, chat, id).await;
    });
}

fn load_entry_db(ui: &AppWindow, uuid: SharedString) {
    let ui = ui.as_weak();

    tokio::spawn(async move {
        match entry::select(DB_TABLE, &uuid).await {
            Ok(item) => match serde_json::from_str::<ChatSession>(&item.data) {
                Ok(session) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        let ui = ui.unwrap();

                        global_store!(ui).set_current_chat_session(session.into());

                        md::parse_histories_bot_text(&ui);
                    });
                }
                Err(e) => toast::async_toast_warn(
                    ui,
                    format!("{}. {}: {e:?}", tr("Load entry failed"), tr("Reason")),
                ),
            },
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Load entry failed"), tr("Reason")),
            ),
        };
    });
}

fn add_db_entry(ui: &AppWindow) {
    let entry_db: ChatSession = store_current_chat_session!(ui).into();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match entry::insert(DB_TABLE, &entry_db.uuid, &data).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Add entry failed"), tr("Reason")),
            ),
            _ => (),
        }
    });
}

fn update_db_entry(ui: &AppWindow) {
    let entry_db: ChatSession = store_current_chat_session!(ui).into();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let data = serde_json::to_string(&entry_db).unwrap();
        match entry::update(DB_TABLE, &entry_db.uuid, &data).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Update entry failed"), tr("Reason")),
            ),
            _ => (),
        }
    });
}

pub fn delete_db_entry(ui: &AppWindow, uuid: SharedString) {
    let ui = ui.as_weak();
    tokio::spawn(async move {
        match entry::delete(DB_TABLE, uuid.as_str()).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Remove entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Remove entry successfully")),
        }
    });
}

fn async_update_chat_phase(ui: Weak<AppWindow>, phase: ChatPhase) {
    _ = slint::invoke_from_event_loop(move || {
        global_store!(ui.unwrap()).set_chat_phase(phase);
    });
}

impl From<SettingModel> for ChatAPIConfig {
    fn from(setting: SettingModel) -> Self {
        ChatAPIConfig {
            api_base_url: setting.chat.api_base_url,
            api_model: setting.chat.model_name,
            api_key: setting.chat.api_key,
            temperature: None,
        }
    }
}

impl From<UIChatEntry> for HistoryChat {
    fn from(entry: UIChatEntry) -> Self {
        HistoryChat {
            utext: entry.user.into(),
            btext: entry.bot.into(),
        }
    }
}

pub async fn get_from_db() -> Vec<UIChatSession> {
    let entries = match entry::select_all(DB_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_str::<ChatSession>(&item.data).ok())
            .map(|item| item.into())
            .collect(),

        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    };

    entries
}

fn chat_session_init(ui: &AppWindow) {
    let mut session = UIChatSession::default();
    session.histories = ModelRc::new(VecModel::from(vec![]));
    global_store!(ui).set_current_chat_session(session);
}

fn new_chat_session(ui: &AppWindow) {
    chat_session_init(ui);
}

fn load_chat_session(ui: &AppWindow, uuid: slint::SharedString) {
    load_entry_db(&ui, uuid);
}

fn retry_question(ui: &AppWindow, index: i32, mut question: slint::SharedString) {
    let index = index as usize;

    if question.is_empty() {
        let entry = store_current_chat_session_histories!(ui)
            .row_data(index)
            .unwrap();
        question = entry.user;
    }

    // remove entries from [index, rows)
    let rows = store_current_chat_session_histories!(ui).row_count();
    for offset in 0..(rows - index) {
        store_current_chat_session_histories!(ui).remove(rows - 1 - offset);
    }

    global_logic!(ui).invoke_send_question(question);
}

fn copy_last_bot_text(ui: &AppWindow) {
    let index = store_current_chat_session_histories!(ui).row_count();
    if index <= 0 {
        return;
    }

    let entry = store_current_chat_session_histories!(ui)
        .row_data(index - 1)
        .unwrap();

    global_logic!(ui).invoke_copy_to_clipboard(entry.bot);
}

fn remove_question(ui: &AppWindow, index: i32) {
    store_current_chat_session_histories!(ui).remove(index as usize);
    update_db_entry(&ui);
}

fn toggle_edit_question(ui: &AppWindow, index: i32) {
    let index = index as usize;

    let mut entry = store_current_chat_session_histories!(ui)
        .row_data(index)
        .unwrap();
    entry.is_user_edit = !entry.is_user_edit;
    store_current_chat_session_histories!(ui).set_row_data(index, entry);
}

fn toggle_hide_bot_reasoner(ui: &AppWindow, index: i32) {
    let index = index as usize;

    let mut entry = store_current_chat_session_histories!(ui)
        .row_data(index)
        .unwrap();
    entry.is_hide_bot_reasoner = !entry.is_hide_bot_reasoner;
    store_current_chat_session_histories!(ui).set_row_data(index, entry);
}
