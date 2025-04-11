use super::{toast, tr::tr};
use crate::{
    config::{data::Model as SettingModel, model as setting_chat_model},
    db::{
        self,
        def::{CHAT_SESSION_TABLE as DB_TABLE, ChatEntry, ChatSession},
    },
    slint_generatedAppWindow::{
        AppWindow, ChatEntry as UIChatEntry, ChatSession as UIChatSession, Logic,
        PromptEntry as UIPromptEntry, Store,
    },
    store_prompt_entries, toast_warn,
};
use bot::openai::{
    Chat,
    request::{APIConfig as ChatAPIConfig, HistoryChat},
    response::StreamTextItem,
};
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
}

static INC_CHAT_ID: AtomicU64 = AtomicU64::new(0);
static CHAT_CACHE: Lazy<Mutex<Option<ChatCache>>> = Lazy::new(|| Mutex::new(None));

#[macro_export]
macro_rules! store_current_chat_session {
    ($ui:expr) => {
        $ui.global::<Store>().get_current_chat_session()
    };
}

#[macro_export]
macro_rules! store_current_chat_session_histories {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_current_chat_session()
            .histories
            .as_any()
            .downcast_ref::<VecModel<UIChatEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

impl From<SettingModel> for ChatAPIConfig {
    fn from(setting: SettingModel) -> Self {
        ChatAPIConfig {
            api_base_url: setting.chat.api_base_url,
            api_model: setting.chat.model_name,
            api_key: setting.chat.api_key,
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

impl From<UIChatSession> for ChatSession {
    fn from(entry: UIChatSession) -> Self {
        let histories = entry
            .histories
            .iter()
            .map(|entry| entry.into())
            .collect::<Vec<ChatEntry>>();

        ChatSession {
            uuid: entry.uuid.into(),
            time: entry.time.into(),
            prompt: entry.prompt.into(),
            histories,
        }
    }
}

impl From<ChatSession> for UIChatSession {
    fn from(entry: ChatSession) -> Self {
        let histories = ModelRc::new(
            entry
                .histories
                .into_iter()
                .map(|entry| entry.into())
                .collect::<VecModel<UIChatEntry>>(),
        );

        UIChatSession {
            uuid: entry.uuid.into(),
            time: entry.time.into(),
            prompt: entry.prompt.into(),
            histories,
        }
    }
}

pub async fn get_from_db() -> Vec<UIChatSession> {
    let entries = match db::entry::select_all(DB_TABLE).await {
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
    session.histories = store_current_chat_session!(ui).histories;
    ui.global::<Store>().set_current_chat_session(session);
    store_current_chat_session_histories!(ui).set_vec(vec![]);
}

pub fn init(ui: &AppWindow) {
    // chat_session_init(ui);

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_new_chat_session(move || {
        let ui = ui_handle.unwrap();
        chat_session_init(&ui);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_chat_session(move |uuid| {
        let ui = ui_handle.unwrap();
        load_entry_db(&ui, uuid);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_send_question(move |question| {
        let ui = ui_handle.unwrap();
        send_question(&ui, question);
    });

    ui.global::<Logic>().on_stop_question(move || {
        tokio::spawn(async move {
            let mut cc = CHAT_CACHE.lock().unwrap();
            if let Some(cc) = cc.take() {
                _ = cc.stop_tx.send(());
            }
        });
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_retry_question(move |index, mut question| {
            let ui = ui_handle.unwrap();
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

            ui.global::<Logic>().invoke_send_question(question);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_question(move |index| {
        let ui = ui_handle.unwrap();
        store_current_chat_session_histories!(ui).remove(index as usize);
        update_db_entry(&ui);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_toggle_edit_question(move |index| {
        let ui = ui_handle.unwrap();
        let index = index as usize;

        let mut entry = store_current_chat_session_histories!(ui)
            .row_data(index)
            .unwrap();
        entry.is_user_edit = !entry.is_user_edit;
        store_current_chat_session_histories!(ui).set_row_data(index, entry);
    });
}

fn stream_text(id: u64, item: StreamTextItem) {
    // log::debug!("{item:?}");

    if id != item.id {
        return;
    }

    let (cc_id, ui) = {
        let cc = CHAT_CACHE.lock().unwrap();
        if cc.is_none() {
            return;
        }

        let cc = cc.as_ref().unwrap();
        (cc.id, cc.ui.clone())
    };

    if id != cc_id {
        return;
    }

    let _ = slint::invoke_from_event_loop(move || {
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
            update_db_entry(&ui);
            return;
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

            entry.bot.push_str(&item.text.unwrap());
            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
            // ui.global::<Logic>()
            //     .invoke_notify_chat_session_auto_scroll();
        }
    });
}

fn parse_prompt(ui: &AppWindow, question: SharedString) -> (SharedString, SharedString) {
    let mut session = store_current_chat_session!(ui);

    if question.is_empty() || !question.starts_with("/") {
        return (session.prompt, question);
    }

    if let Some(shortcut) = question.split_whitespace().next() {
        if let Some(entry) = store_prompt_entries!(ui)
            .iter()
            .find(|item| item.shortcut.as_str().eq(&shortcut[1..]))
        {
            let question = question
                .trim_start_matches(&format!("/{}", entry.shortcut))
                .trim_start()
                .into();

            session.prompt = entry.detail.clone();
            ui.global::<Store>().set_current_chat_session(session);
            return (entry.detail, question);
        }
    }

    (session.prompt, question)
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let (prompt, question) = parse_prompt(ui, question);

    let mut session = store_current_chat_session!(ui);
    let (is_new_chat, histories) = if session.uuid.is_empty() {
        session.uuid = Uuid::new_v4().to_string().into();
        session.time = cutil::time::local_now("%m-%d %H:%M").into();
        ui.global::<Store>().set_current_chat_session(session);

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
        user: question.clone(),
        ..Default::default()
    });

    if is_new_chat {
        add_db_entry(ui);
    }

    ui.global::<Store>().set_is_chatting(true);

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let config = setting_chat_model().into();
        let (chat, stop_tx) = Chat::new(prompt, question, config, histories);
        let id = INC_CHAT_ID.fetch_add(1, Ordering::Relaxed);

        {
            let mut cc = CHAT_CACHE.lock().unwrap();
            *cc = Some(ChatCache {
                id,
                ui: ui.clone(),
                stop_tx: Arc::new(stop_tx),
            });
        }

        match chat
            .start(id, |item| {
                stream_text(id, item);
            })
            .await
        {
            Err(e) => toast::async_toast_warn(
                ui.clone(),
                format!("{}. {}: {e:?}", tr("Chat failed"), tr("Reason")),
            ),
            _ => (),
        }

        let _ = slint::invoke_from_event_loop(move || {
            ui.unwrap().global::<Store>().set_is_chatting(false);
        });
    });
}

fn load_entry_db(ui: &AppWindow, uuid: SharedString) {
    let ui = ui.as_weak();

    tokio::spawn(async move {
        match db::entry::select(DB_TABLE, &uuid).await {
            Ok(item) => match serde_json::from_str::<ChatSession>(&item.data) {
                Ok(session) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        ui.unwrap()
                            .global::<Store>()
                            .set_current_chat_session(session.into());
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
        match db::entry::insert(DB_TABLE, &entry_db.uuid, &data).await {
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
        match db::entry::update(DB_TABLE, &entry_db.uuid, &data).await {
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
        match db::entry::delete(DB_TABLE, uuid.as_str()).await {
            Err(e) => toast::async_toast_warn(
                ui,
                format!("{}. {}: {e:?}", tr("Remove entry failed"), tr("Reason")),
            ),
            _ => toast::async_toast_success(ui, tr("Remove entry successfully")),
        }
    });
}
