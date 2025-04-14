use crate::slint_generatedAppWindow::{
    ChatEntry as UIChatEntry, ChatHistory, ChatSession as UIChatSession,
    PromptEntry as UIPromptEntry, SearchLink as UISearchLink,
};
use search::SearchLink;
use serde::{Deserialize, Serialize};
use slint::{Model, ModelRc, VecModel};

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
    search_links: Vec<SearchLink>,
}

impl From<UIChatEntry> for ChatEntry {
    fn from(entry: UIChatEntry) -> Self {
        let search_links = entry
            .search_links
            .iter()
            .map(|item| item.into())
            .collect::<Vec<SearchLink>>();

        ChatEntry {
            user: entry.user.into(),
            bot: entry.bot.into(),
            search_links,
        }
    }
}

impl From<ChatEntry> for UIChatEntry {
    fn from(entry: ChatEntry) -> Self {
        let search_links = ModelRc::new(
            entry
                .search_links
                .into_iter()
                .map(|item| item.into())
                .collect::<VecModel<UISearchLink>>(),
        );

        UIChatEntry {
            user: entry.user.into(),
            bot: entry.bot.into(),
            search_links,
            md_elems: ModelRc::new(VecModel::from(vec![])),
            link_urls: ModelRc::new(VecModel::from(vec![])),
            ..Default::default()
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
