use crate::slint_generatedAppWindow::{
    ChatEntry as UIChatEntry, ChatHistory, ChatSession as UIChatSession, MCPEntry as UIMCPEntry,
    PromptEntry as UIPromptEntry, PromptType, SearchLink as UISearchLink,
};
use search::SearchLink;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use slint::{Model, ModelRc, VecModel};
use std::fmt;

pub const PROMPT_TABLE: &str = "prompt";
pub const MCP_TABLE: &str = "mcp";
pub const CHAT_SESSION_TABLE: &str = "chat_session";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PromptEntry {
    pub uuid: String,
    pub name: String,
    pub shortcut: String,
    pub detail: String,
    pub temperature: f32,
}

impl From<UIPromptEntry> for PromptEntry {
    fn from(entry: UIPromptEntry) -> Self {
        PromptEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            detail: entry.detail.into(),
            temperature: entry.temperature,
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
            temperature: entry.temperature,
        }
    }
}

impl From<UIMCPEntry> for UIPromptEntry {
    fn from(entry: UIMCPEntry) -> Self {
        UIPromptEntry {
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MCPEntry {
    pub uuid: String,
    pub name: String,
    pub shortcut: String,
    pub config: String,

    #[serde(default = "mcp_default_temperature")]
    pub temperature: f32,
}

impl From<UIMCPEntry> for MCPEntry {
    fn from(entry: UIMCPEntry) -> Self {
        MCPEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            config: entry.config.into(),
            temperature: entry.temperature,
        }
    }
}

impl From<MCPEntry> for UIMCPEntry {
    fn from(entry: MCPEntry) -> Self {
        UIMCPEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            shortcut: entry.shortcut.into(),
            config: entry.config.into(),
            temperature: entry.temperature,
            ..Default::default()
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
    pub mcp_config: String,
    pub prompt_type: PromptType,
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

impl Serialize for PromptType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PromptType::Normal => serializer.serialize_str("Normal"),
            PromptType::MCP => serializer.serialize_str("MCP"),
        }
    }
}

impl<'de> Deserialize<'de> for PromptType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PromptTypeVisitor;

        impl<'de> Visitor<'de> for PromptTypeVisitor {
            type Value = PromptType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing PromptType ('Normal' or 'MCP')")
            }

            fn visit_str<E>(self, value: &str) -> Result<PromptType, E>
            where
                E: de::Error,
            {
                match value {
                    "Normal" => Ok(PromptType::Normal),
                    "MCP" => Ok(PromptType::MCP),
                    _ => Err(E::custom(format!("unknown PromptType variant: {}", value))),
                }
            }
        }

        deserializer.deserialize_str(PromptTypeVisitor)
    }
}

fn mcp_default_temperature() -> f32 {
    return 0.7;
}
