use crate::{
    config::{Model as SettingModel, model as setting_model},
    db::{CHAT_SESSION_TABLE as DB_TABLE, ChatSession},
    db_add, db_remove, db_select, db_update, global_logic, global_store,
    logic::{md, toast},
    logic_cb,
    slint_generatedAppWindow::{
        AppWindow, ChatEntry as UIChatEntry, ChatPhase, ChatSession as UIChatSession,
    },
    toast_warn,
};
use bot::openai::{
    Chat, ChatConfig,
    request::{APIConfig as ChatAPIConfig, HistoryChat},
    response::StreamTextItem,
};
use once_cell::sync::Lazy;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::sync::Mutex;
use tokio::sync::mpsc::{Sender, channel};
use uuid::Uuid;

static CHAT_ABORT_HANDLE: Lazy<Mutex<Option<tokio::task::AbortHandle>>> =
    Lazy::new(|| Mutex::new(None));

db_add!(DB_TABLE, ChatSession);
db_select!(DB_TABLE, ChatSession);
db_update!(DB_TABLE, ChatSession);
db_remove!(DB_TABLE);

#[macro_export]
macro_rules! store_current_chat_session {
    ($ui:expr) => {
        $crate::global_store!($ui).get_current_chat_session()
    };
}

#[macro_export]
macro_rules! store_current_chat_session_histories {
    ($ui:expr) => {
        $crate::global_store!($ui)
            .get_current_chat_session()
            .histories
            .as_any()
            .downcast_ref::<VecModel<UIChatEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

pub fn init(ui: &AppWindow) {
    inner_init(ui);

    logic_cb!(new_chat_session, ui);
    logic_cb!(load_chat_session, ui, uuid);
    logic_cb!(send_question, ui, question);
    logic_cb!(stop_question, ui);
    logic_cb!(retry_question, ui, index, question);
    logic_cb!(remove_question, ui, index);
    logic_cb!(copy_last_bot_text, ui);
    logic_cb!(toggle_edit_question, ui, index);
    logic_cb!(toggle_hide_bot_reasoner, ui, index);
}

fn inner_init(ui: &AppWindow) {
    let mut session = UIChatSession::default();
    session.histories = ModelRc::new(VecModel::from(vec![]));
    global_store!(ui).set_current_chat_session(session);
}

fn new_chat_session(ui: &AppWindow) {
    inner_init(ui);
}

fn load_chat_session(ui: &AppWindow, uuid: slint::SharedString) {
    load_db_entry(ui, uuid);
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let histories = chat_histories(ui, question.clone());
    let enabled_reasoner_model = global_store!(ui).get_enabled_reasoner_model();

    let ui_weak = ui.as_weak();
    tokio::spawn(async move {
        log::info!("start sending question to model...");

        let (tx, mut rx) = channel(100);
        let chat = prepare_chat(
            ui_weak.clone(),
            question,
            histories,
            enabled_reasoner_model,
            tx,
        );

        let ui_weak_clone = ui_weak.clone();
        let recv_task = tokio::spawn(async move {
            while let Some(item) = rx.recv().await {
                stream_text(ui_weak_clone.clone(), item);
            }
        });

        let chat_task = tokio::spawn(async move {
            start_chat(ui_weak, chat).await;
        });

        *CHAT_ABORT_HANDLE.lock().unwrap() = Some(chat_task.abort_handle());

        _ = chat_task.await;
        recv_task.abort();
    });
}

fn stop_question(_ui: &AppWindow) {
    if let Some(handle) = CHAT_ABORT_HANDLE.lock().unwrap().take() {
        handle.abort();
    }
}

fn retry_question(ui: &AppWindow, index: i32, mut question: slint::SharedString) {
    let index = index as usize;

    if question.is_empty()
        && let Some(entry) = store_current_chat_session_histories!(ui).row_data(index)
    {
        question = entry.user;
    }

    // remove entries from [index, rows)
    let rows = store_current_chat_session_histories!(ui).row_count();
    for offset in 0..(rows - index) {
        store_current_chat_session_histories!(ui).remove(rows - 1 - offset);
    }

    global_logic!(ui).invoke_send_question(question);
}

fn remove_question(ui: &AppWindow, index: i32) {
    store_current_chat_session_histories!(ui).remove(index as usize);
    let entry_db: ChatSession = store_current_chat_session!(ui).into();
    db_update(ui.as_weak(), entry_db);
}

fn copy_last_bot_text(ui: &AppWindow) {
    let index = store_current_chat_session_histories!(ui).row_count();
    if let Some(entry) = store_current_chat_session_histories!(ui).row_data(index - 1) {
        global_logic!(ui).invoke_copy_to_clipboard(entry.bot);
    }
}

fn toggle_edit_question(ui: &AppWindow, index: i32) {
    let index = index as usize;
    if let Some(mut entry) = store_current_chat_session_histories!(ui).row_data(index) {
        entry.is_user_edit = !entry.is_user_edit;
        store_current_chat_session_histories!(ui).set_row_data(index, entry);
    }
}

fn toggle_hide_bot_reasoner(ui: &AppWindow, index: i32) {
    let index = index as usize;
    if let Some(mut entry) = store_current_chat_session_histories!(ui).row_data(index) {
        entry.is_hide_bot_reasoner = !entry.is_hide_bot_reasoner;
        store_current_chat_session_histories!(ui).set_row_data(index, entry);
    }
}

fn prepare_chat(
    ui: Weak<AppWindow>,
    question: SharedString,
    histories: Vec<HistoryChat>,
    enabled_reasoner_model: bool,
    tx: Sender<StreamTextItem>,
) -> Chat {
    async_update_chat_phase(ui.clone(), ChatPhase::Thinking);

    let mut request_config: ChatAPIConfig = setting_model().into();
    if enabled_reasoner_model {
        request_config.api_model = setting_model().chat.reasoner_model_name;
    }

    let prompt = SharedString::from("You are a helpful AI assistant.");
    let chat_config = ChatConfig { tx };
    let chat = Chat::new(prompt, question, chat_config, request_config, histories);

    chat
}

async fn start_chat(ui: Weak<AppWindow>, chat: Chat) {
    if let Err(e) = chat.start().await {
        toast::async_toast_warn(ui.clone(), format!("Chat failed: {e:?}"));
    }

    async_update_chat_phase(ui, ChatPhase::None);
}

fn stream_text(ui: Weak<AppWindow>, item: StreamTextItem) {
    _ = ui.upgrade_in_event_loop(move |ui| {
        if let Some(err) = item.etext {
            toast_warn!(ui, format!("Chat failed: {err}"));
            return;
        }

        if item.finished {
            md::parse_last_history_bot_text(&ui);
            let entry_db: ChatSession = store_current_chat_session!(ui).into();
            db_update(ui.as_weak(), entry_db);
            return;
        }

        let chat_phase = global_store!(ui).get_chat_phase();

        if let Some(ref rtext) = item.reasoning_text {
            if chat_phase != ChatPhase::Chatting {
                global_store!(ui).set_chat_phase(ChatPhase::Chatting);
            }

            let rows = store_current_chat_session_histories!(ui).row_count();
            if rows == 0 {
                return;
            }

            let last_index = rows - 1;
            let mut entry = store_current_chat_session_histories!(ui)
                .row_data(last_index)
                .unwrap();

            entry.bot_reasoner.push_str(rtext);
            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
        }

        if let Some(text) = item.text {
            let rows = store_current_chat_session_histories!(ui).row_count();
            if rows == 0 {
                return;
            }

            let last_index = rows - 1;
            let mut entry = store_current_chat_session_histories!(ui)
                .row_data(last_index)
                .unwrap();
            entry.bot.push_str(&text);

            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
            global_store!(ui).set_chat_phase(ChatPhase::Chatting);
            md::parse_stream_bot_text(&ui);
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
        let entry_db: ChatSession = store_current_chat_session!(ui).into();
        db_add(ui.as_weak(), entry_db);
    }

    histories
}

fn async_update_chat_phase(ui: Weak<AppWindow>, phase: ChatPhase) {
    _ = slint::invoke_from_event_loop(move || {
        global_store!(ui.unwrap()).set_chat_phase(phase);
    });
}

fn load_db_entry(ui: &AppWindow, uuid: SharedString) {
    db_select(
        ui.as_weak(),
        uuid,
        |ui: &AppWindow, session: ChatSession| {
            global_store!(ui).set_current_chat_session(session.into());
            md::parse_histories_bot_text(ui);
        },
    );
}

pub async fn get_all_db_entries() -> Vec<UIChatSession> {
    use crate::db_select_all;
    db_select_all!(DB_TABLE, ChatSession)
        .into_iter()
        .map(|item: ChatSession| item.into())
        .collect()
}

pub fn delete_db_entry(ui: &AppWindow, uuid: SharedString) {
    db_remove(ui.as_weak(), uuid);
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
