use super::{toast, tr::tr};
use crate::db::def::{CHAT_SESSION_TABLE as DB_TABLE, ChatEntry, ChatSession};
use crate::slint_generatedAppWindow::{
    AppWindow, ChatEntry as UIChatEntry, ChatSession as UIChatSession, Logic, Store,
};
use crate::{
    config::{data::Model as SettingModel, model as setting_chat_model},
    db,
};
use bot::openai::{
    Chat,
    request::{APIConfig, HistoryChat},
    response::StreamTextItem,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::sync::{Arc, mpsc};
use uuid::Uuid;

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

impl From<SettingModel> for APIConfig {
    fn from(setting: SettingModel) -> Self {
        APIConfig {
            api_base_url: setting.api_base_url,
            api_model: setting.model_name,
            api_key: setting.api_key,
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
    chat_session_init(ui);

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

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_stop_question(move || {
        let ui = ui_handle.unwrap();
        todo!();
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_retry_question(move |index| {
        let ui = ui_handle.unwrap();
        let index = index as usize;

        let rows = store_current_chat_session_histories!(ui).row_count();

        // remove entries from [index + 1, rows)
        for offset in 0..(rows - index - 1) {
            store_current_chat_session_histories!(ui).remove(rows - offset - 1);
        }

        let mut entry = store_current_chat_session_histories!(ui)
            .row_data(index)
            .unwrap();

        let question = entry.user.clone();
        entry.bot = SharedString::default();
        store_current_chat_session_histories!(ui).set_row_data(index, entry);
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

fn stream_text(
    ui: Weak<AppWindow>,
    item: StreamTextItem,
    stop_tx: Arc<mpsc::Sender<()>>,
    is_new_chat: bool,
) {
    println!("{item:?}");
    todo!();
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let session = store_current_chat_session!(ui);
    let prompt = session.prompt;

    let (uuid, is_new_chat) = if session.uuid.is_empty() {
        (Uuid::new_v4().to_string(), true)
    } else {
        (session.uuid.to_string(), false)
    };

    let histories = session
        .histories
        .iter()
        .map(|entry| entry.into())
        .collect::<Vec<HistoryChat>>();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        let ui = ui.clone();
        let config = setting_chat_model().into();
        let (chat, stop_tx) = Chat::new(prompt, question, config, histories);
        let stop_tx = Arc::new(stop_tx);

        // match chat
        //     .start(uuid, |item| {
        //         stream_text(ui, item, stop_tx, is_new_chat);
        //     })
        //     .await
        // {
        //     Err(e) => toast::async_toast_warn(
        //         ui,
        //         format!("{}. {}: {e:?}", tr("Chat failed"), tr("Reason")),
        //     ),
        //     _ => (),
        // }
    });
}

fn load_entry_db(ui: &AppWindow, uuid: SharedString) {
    let ui = ui.as_weak();

    tokio::spawn(async move {
        match db::entry::select(DB_TABLE, &uuid).await {
            Ok(item) => match serde_json::from_str::<ChatSession>(&item.data) {
                Ok(session) => {
                    ui.unwrap()
                        .global::<Store>()
                        .set_current_chat_session(session.into());
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
            _ => toast::async_toast_success(ui, tr("Add entry successfully")),
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
            _ => toast::async_toast_success(ui, tr("Update entry successfully")),
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
