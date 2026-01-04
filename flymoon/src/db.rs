use pmacro::SlintFromConvert;
use serde::{Deserialize, Serialize};
use slint::Model;

use crate::slint_generatedAppWindow::{
    ChatEntry as UIChatEntry, ChatHistory, ChatSession as UIChatSession,
};

pub const CHAT_SESSION_TABLE: &str = "chat_session";

pub async fn init(db_path: &str) {
    sqldb::create_db(db_path).await.expect("create db");

    sqldb::entry::new(CHAT_SESSION_TABLE)
        .await
        .expect("chat session table failed");
}

#[macro_export]
macro_rules! db_add {
    ($table:expr, $ty:ident) => {
        fn db_add(ui: slint::Weak<crate::slint_generatedAppWindow::AppWindow>, entry: $ty) {
            tokio::spawn(async move {
                let data = serde_json::to_string(&entry).expect("Not implement `Serialize` trait");
                if let Err(e) = sqldb::entry::insert($table, entry.uuid.as_str(), &data).await {
                    crate::logic::toast::async_toast_warn(
                        ui,
                        format!("{}. {e}", crate::logic::tr::tr("insert entry failed")),
                    );
                }
            });
        }
    };
}

#[macro_export]
macro_rules! db_update {
    ($table:expr, $ty:ident) => {
        fn db_update(ui: slint::Weak<crate::slint_generatedAppWindow::AppWindow>, entry: $ty) {
            tokio::spawn(async move {
                let data = serde_json::to_string(&entry).expect("Not implement `Serialize` trait");
                if let Err(e) = sqldb::entry::update($table, entry.uuid.as_str(), &data).await {
                    crate::logic::toast::async_toast_warn(
                        ui,
                        format!("{}. {e}", crate::logic::tr::tr("update entry failed")),
                    );
                }
            });
        }
    };
}

#[macro_export]
macro_rules! db_select_all {
    ($table:expr, $ty:ident) => {{
        match sqldb::entry::select_all($table).await {
            Ok(items) => items
                .into_iter()
                .filter_map(|item| serde_json::from_str::<$ty>(&item.data).ok())
                .collect(),
            Err(e) => {
                log::warn!("{:?}", e);
                vec![]
            }
        }
    }};
}

#[macro_export]
macro_rules! db_remove {
    ($table:expr) => {
        fn db_remove(
            ui: slint::Weak<crate::slint_generatedAppWindow::AppWindow>,
            uuid: slint::SharedString,
        ) {
            let uuid = uuid.to_string();
            tokio::spawn(async move {
                if let Err(e) = sqldb::entry::delete($table, uuid.as_str()).await {
                    crate::logic::toast::async_toast_warn(
                        ui,
                        format!("{}. {e}", crate::logic::tr::tr("remove entry failed")),
                    );
                }
            });
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, SlintFromConvert)]
#[from("UIChatEntry")]
#[vec_ui("md_elems")]
#[vec_ui("link_urls")]
pub struct ChatEntry {
    pub user: String,
    pub bot: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, SlintFromConvert)]
#[from("UIChatSession")]
pub struct ChatSession {
    pub uuid: String,
    pub time: String,
    #[vec(from = "histories")]
    pub histories: Vec<ChatEntry>,
}

impl From<UIChatSession> for ChatHistory {
    fn from(entry: UIChatSession) -> Self {
        ChatHistory {
            uuid: entry.uuid,
            time: entry.time,
            summary: entry
                .histories
                .row_data(0)
                .unwrap_or_default()
                .user
                .replace(['\r', '\n'], "")
                .into(),
            ..Default::default()
        }
    }
}

pub use sqldb::entry;
