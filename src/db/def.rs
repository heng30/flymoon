use serde::{Deserialize, Serialize};

use crate::slint_generatedAppWindow::{ChatEntry as UIChatEntry, PromptEntry as UIPromptEntry};
pub const PROMPT_TABLE: &str = "prompt";
pub const CHAT_SESSION_TABLE: &str = "chat_session";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PromptEntry {
    pub uuid: String,
    pub name: String,
    pub shortcut: String,
    pub detail: String,
}

impl From<UIPromptEntry> for PromptEntry {
    fn from(entry: UIPromptEntry) -> Self {
        PromptEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            detail: entry.detail.into(),
        }
    }
}

impl From<PromptEntry> for UIPromptEntry {
    fn from(entry: PromptEntry) -> Self {
        UIPromptEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            detail: entry.detail.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChatEntry {
    user: String,
    bot: String,
}

impl From<UIChatEntry> for ChatEntry {
    fn from(entry: UIChatEntry) -> Self {
        ChatEntry {
            user: entry.user.into(),
            bot: entry.bot.into(),
        }
    }
}

impl From<ChatEntry> for UIChatEntry {
    fn from(entry: ChatEntry) -> Self {
        UIChatEntry {
            user: entry.user.into(),
            bot: entry.bot.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChatSession {
    pub uuid: String,
    pub time: String,
    pub prompt: String,
    pub histories: Vec<ChatEntry>,
}

// use serde::{Deserialize, Deserializer, Serialize, Serializer};
// use serde_with::{serde_as, DeserializeAs, SerializeAs};

// #[serde_as]
// #[derive(Serialize, Deserialize, Debug, Clone, Default)]
// pub struct HistoryEntry {
//     pub uuid: String,
//     pub network: String,
//     pub hash: String,
//     pub balance: String,
//     pub time: String,

//     #[serde_as(as = "TranStatus")]
//     pub status: TransactionTileStatus,
// }

// struct TranStatus;
// impl SerializeAs<TransactionTileStatus> for TranStatus {
//     fn serialize_as<S>(source: &TransactionTileStatus, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let status = match source {
//             TransactionTileStatus::Success => "Success",
//             TransactionTileStatus::Pending => "Pending",
//             _ => "Error",
//         };

//         serializer.serialize_str(status)
//     }
// }

// impl<'de> DeserializeAs<'de, TransactionTileStatus> for TranStatus {
//     fn deserialize_as<D>(deserializer: D) -> Result<TransactionTileStatus, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let status = String::deserialize(deserializer)?;
//         let status = match status.as_str() {
//             "Success" => TransactionTileStatus::Success,
//             "Pending" => TransactionTileStatus::Pending,
//             _ => TransactionTileStatus::Error,
//         };
//         Ok(status)
//     }
// }
