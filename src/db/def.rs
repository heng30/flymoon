use crate::slint_generatedAppWindow::{
    ChatEntry as UIChatEntry, ChatHistory, ChatSession as UIChatSession,
    PromptEntry as UIPromptEntry,
};
use serde::{Deserialize, Serialize};
use slint::Model;

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

impl From<UIChatSession> for ChatHistory {
    fn from(entry: UIChatSession) -> Self {
        ChatHistory {
            uuid: entry.uuid,
            time: entry.time,
            summary: entry.histories.row_data(0).unwrap_or_default().user,
            ..Default::default()
        }
    }
}
