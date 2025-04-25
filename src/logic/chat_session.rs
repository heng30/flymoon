use super::{md, toast, tr::tr};
use crate::{
    config::{data::Model as SettingModel, model as setting_model},
    db::{
        self,
        def::{CHAT_SESSION_TABLE as DB_TABLE, ChatEntry, ChatSession},
    },
    slint_generatedAppWindow::{
        AppWindow, ChatEntry as UIChatEntry, ChatPhase, ChatSession as UIChatSession, Logic,
        MCPEntry as UIMCPEntry, PromptEntry as UIPromptEntry, SearchLink as UISearchLink, Store,
    },
    store_mcp_entries, store_prompt_entries, toast_success, toast_warn,
};
use anyhow::Result;
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

#[derive(Copy, Debug, PartialEq, Clone)]
enum PromptType {
    Normal,
    MCP,
}

const MCP_TOOL_START_SEP: &'static str = "TOOL_START";
const MCP_TOOL_END_SEP: &'static str = "TOOL_END";

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

#[macro_export]
macro_rules! store_current_chat_session_histories_search_links {
    ($entry:expr) => {
        $entry
            .search_links
            .as_any()
            .downcast_ref::<VecModel<UISearchLink>>()
            .expect("We know we set a VecModel earlier")
    };
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

impl From<SettingModel> for search::google::Config {
    fn from(setting: SettingModel) -> Self {
        Self {
            cx: setting.google_search.cx,
            api_key: setting.google_search.api_key,
            num: setting.google_search.num as u8,
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

impl From<search::SearchLink> for UISearchLink {
    fn from(entry: search::SearchLink) -> Self {
        Self {
            title: entry.title.into(),
            link: entry.link.into(),
        }
    }
}

impl From<UISearchLink> for search::SearchLink {
    fn from(entry: UISearchLink) -> Self {
        Self {
            title: entry.title.into(),
            link: entry.link.into(),
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
    session.histories = ModelRc::new(VecModel::from(vec![]));
    ui.global::<Store>().set_current_chat_session(session);
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

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_toggle_hide_bot_reasoner(move |index| {
            let ui = ui_handle.unwrap();
            let index = index as usize;

            let mut entry = store_current_chat_session_histories!(ui)
                .row_data(index)
                .unwrap();
            entry.is_hide_bot_reasoner = !entry.is_hide_bot_reasoner;
            store_current_chat_session_histories!(ui).set_row_data(index, entry);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_clear_current_chat_session_prompt(move || {
            let ui = ui_handle.unwrap();
            let mut session = store_current_chat_session!(ui);
            session.prompt = Default::default();
            ui.global::<Store>().set_current_chat_session(session);

            toast_success!(ui, tr("Clear current session prompt successfully"));
        });
}

fn parse_prompt(
    ui: &AppWindow,
    question: SharedString,
) -> (SharedString, SharedString, Option<f32>, PromptType) {
    let mut session = store_current_chat_session!(ui);

    if question.is_empty() || (!question.starts_with("/") && !question.starts_with("@")) {
        return (session.prompt, question, None, PromptType::Normal);
    }

    if let Some(shortcut) = question.split_whitespace().next() {
        if question.starts_with("/") {
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
                return (
                    entry.detail,
                    question,
                    Some(entry.temperature),
                    PromptType::Normal,
                );
            }
        } else if question.starts_with("@") {
            if let Some(entry) = store_mcp_entries!(ui)
                .iter()
                .find(|item| item.shortcut.as_str().eq(&shortcut[1..]))
            {
                let question = question
                    .trim_start_matches(&format!("@{}", entry.shortcut))
                    .trim_start()
                    .into();

                // get the prompt later
                session.prompt = entry.config.clone();
                ui.global::<Store>().set_current_chat_session(session);
                return (entry.config, question, Some(0.7), PromptType::MCP);
            }
        }
    }

    (session.prompt, question, None, PromptType::Normal)
}

fn stream_text(id: u64, item: StreamTextItem) {
    // log::debug!("{item:?}");

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

    // for mcp server
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

        if ui.global::<Store>().get_chat_phase() != ChatPhase::Chatting {
            ui.global::<Store>().set_chat_phase(ChatPhase::Chatting);
        }

        if item.reasoning_text.is_some() {
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

            entry.bot.push_str(&item.text.unwrap());
            store_current_chat_session_histories!(ui).set_row_data(last_index, entry);

            if md::need_parse_stream_bot_text(&ui) {
                md::parse_stream_bot_text(&ui);
            }
        }
    });
}

async fn search_webpages(
    ui: Weak<AppWindow>,
    question: &str,
    histories: &mut Vec<HistoryChat>,
) -> Result<()> {
    let ui_handle = ui.clone();
    _ = slint::invoke_from_event_loop(move || {
        ui_handle
            .unwrap()
            .global::<Store>()
            .set_chat_phase(ChatPhase::Searching);
    });

    let config = setting_model().into();

    if let (Some(text), search_links) = search::google::search(question, config).await? {
        log::info!("webpages content length: {}", text.len());
        log::info!("finished searching webpages");

        let text = format!(
            "The following web content is relevant to the user's question. Please consult these resources when preparing your answer. {text}"
        );

        histories.push(HistoryChat {
            utext: text,
            ..Default::default()
        });

        _ = slint::invoke_from_event_loop(move || {
            let ui = ui.unwrap();

            let rows = store_current_chat_session_histories!(ui).row_count();
            if rows > 0 {
                let last_entry = store_current_chat_session_histories!(ui)
                    .row_data(rows - 1)
                    .unwrap();

                let search_links = search_links
                    .into_iter()
                    .map(|item| item.into())
                    .collect::<Vec<UISearchLink>>();

                store_current_chat_session_histories_search_links!(last_entry)
                    .set_vec(search_links);
            }
        });
    }

    Ok(())
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let (mut prompt, question, temperature, prompt_type) = parse_prompt(ui, question);

    log::info!("{} - {:?}", prompt, prompt_type);

    let mut session = store_current_chat_session!(ui);
    let (is_new_chat, mut histories) = if session.uuid.is_empty() {
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
        md_elems: ModelRc::new(VecModel::from(vec![])),
        link_urls: ModelRc::new(VecModel::from(vec![])),
        search_links: ModelRc::new(VecModel::from(vec![])),
        ..Default::default()
    });

    if is_new_chat {
        add_db_entry(ui);
    }

    let enabled_search_webpages = ui.global::<Store>().get_enabled_search_webpages();
    let enabled_reasoner_model = ui.global::<Store>().get_enabled_reasoner_model();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        if enabled_search_webpages && prompt_type != PromptType::MCP {
            log::info!("start searching wabpages...");

            if let Err(e) = search_webpages(ui.clone(), &question, &mut histories).await {
                toast::async_toast_warn(
                    ui.clone(),
                    format!("{}. {}: {e:?}", tr("Search webpages failed"), tr("Reason")),
                );

                let ui_handle = ui.clone();
                _ = slint::invoke_from_event_loop(move || {
                    ui_handle
                        .unwrap()
                        .global::<Store>()
                        .set_chat_phase(ChatPhase::None);
                });

                return;
            }
        }

        let mut mcp_client = None;
        if prompt_type == PromptType::MCP {
            let ui_handle = ui.clone();
            _ = slint::invoke_from_event_loop(move || {
                ui_handle
                    .unwrap()
                    .global::<Store>()
                    .set_chat_phase(ChatPhase::MCP);
            });

            let mut is_get_mcp_prompt_error = false;
            match mcp::create_mcp_client(&prompt).await {
                Ok(client) => match gen_mcp_prompt(&client) {
                    Some(p) => {
                        prompt = p.into();
                        mcp_client = Some(client);

                        // clear chat session system prompt
                        let ui_handle = ui.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            let mut session = ui.global::<Store>().get_current_chat_session();
                            session.prompt = SharedString::default();
                            ui.global::<Store>().set_current_chat_session(session);
                        });
                    }
                    _ => {
                        is_get_mcp_prompt_error = true;
                        toast::async_toast_warn(
                            ui.clone(),
                            format!("{}", tr("No MCP server tools")),
                        );
                    }
                },
                Err(e) => {
                    is_get_mcp_prompt_error = true;
                    toast::async_toast_warn(
                        ui.clone(),
                        format!(
                            "{}. {}: {e:?}",
                            tr("Get MCP server prompt failed"),
                            tr("Reason")
                        ),
                    );
                }
            }

            if is_get_mcp_prompt_error {
                let ui_handle = ui.clone();
                _ = slint::invoke_from_event_loop(move || {
                    ui_handle
                        .unwrap()
                        .global::<Store>()
                        .set_chat_phase(ChatPhase::None);
                });

                return;
            }
        }

        let ui_handle = ui.clone();
        _ = slint::invoke_from_event_loop(move || {
            let phase = if enabled_reasoner_model {
                ChatPhase::Thinking
            } else {
                ChatPhase::Chatting
            };

            ui_handle.unwrap().global::<Store>().set_chat_phase(phase);
        });

        log::info!("start sending question to model...");

        let mut config: ChatAPIConfig = setting_model().into();
        config.temperature = temperature;
        if enabled_reasoner_model {
            config.api_model = setting_model().chat.reasoner_model_name.into();
        }

        // log::info!("{config:?}");

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

        if let Err(e) = chat
            .start(id, |item| {
                stream_text(id, item);
            })
            .await
        {
            toast::async_toast_warn(
                ui.clone(),
                format!("{}. {}: {e:?}", tr("Chat failed"), tr("Reason")),
            );
        } else {
            if mcp_client.is_some() {
                let ui_handle = ui.clone();
                _ = slint::invoke_from_event_loop(move || {
                    ui_handle
                        .unwrap()
                        .global::<Store>()
                        .set_chat_phase(ChatPhase::MCP);
                });

                let mcp_client = mcp_client.unwrap();
                call_mcp_server_tool(ui.clone(), mcp_client).await;
            }
        }

        _ = slint::invoke_from_event_loop(move || {
            ui.unwrap()
                .global::<Store>()
                .set_chat_phase(ChatPhase::None);
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
                        let ui = ui.unwrap();

                        ui.global::<Store>()
                            .set_current_chat_session(session.into());

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

fn gen_mcp_prompt(client: &mcp::Client) -> Option<String> {
    let tools = client.tool_set.tools();
    if tools.is_empty() {
        return None;
    }

    let mut prompt =
        "you are a assistant, you can help user to complete various tasks. you have the following tools to use:\n".to_string();

    for tool in tools {
        prompt.push_str(&format!(
            "\ntool name: {}\ndescription: {}\nparameters: {}\n",
            tool.name(),
            tool.description(),
            serde_json::to_string_pretty(&tool.parameters()).unwrap_or_default()
        ));
    }

    prompt.push_str(&format!(
        r#"\nif you need to call tools, please use the following format:\n
        \n{}\n
        {{"name": "tool_name", "arguments": "tool_arguments"}}
        \n{}\n"#,
        MCP_TOOL_START_SEP, MCP_TOOL_END_SEP
    ));

    Some(prompt)
}

async fn call_mcp_server_tool(ui: Weak<AppWindow>, client: mcp::Client) {
    let content = {
        let cc = CHAT_CACHE.lock().unwrap();
        if cc.is_some() {
            cc.as_ref().unwrap().bot_text.clone()
        } else {
            String::default()
        }
    };

    if content.is_empty() {
        return;
    }

    let (mut meet_tool, mut tool_list, mut tool_text) = (false, vec![], String::default());
    let lines: Vec<&str> = content.split('\n').collect();

    for line in lines {
        if line.starts_with(MCP_TOOL_START_SEP) {
            meet_tool = true;
        } else if line.starts_with(MCP_TOOL_END_SEP) {
            tool_list.push(tool_text.clone());
            meet_tool = false;
            tool_text.clear();
        } else if meet_tool {
            tool_text.push_str(&line);
        }
    }

    for text in &tool_list {
        if let Ok(item) = serde_json::from_str::<mcp::tool::ToolCall>(&text) {
            if item.name.is_empty() {
                continue;
            }

            if let Some(tool) = client.tool_set.get_tool(&item.name) {
                log::info!("calling tool: {}", item.name);
                log::info!("tool args: {:?}", item.arguments);

                match tool.call(item.arguments).await {
                    Ok(result) => {
                        let ui_handle = ui.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();

                            let rows = store_current_chat_session_histories!(ui).row_count();
                            if rows <= 0 {
                                return;
                            }

                            let last_index = rows - 1;
                            let mut entry = store_current_chat_session_histories!(ui)
                                .row_data(last_index)
                                .unwrap();

                            entry.bot.push_str(&format!("\n```\n{}\n```\n", &result));

                            store_current_chat_session_histories!(ui)
                                .set_row_data(last_index, entry);

                            md::parse_stream_bot_text(&ui);
                        });
                    }
                    Err(e) => {
                        toast::async_toast_warn(
                            ui.clone(),
                            format!(
                                "{} - {}. {}: {e:?}",
                                item.name,
                                tr("MCP server tool call failed"),
                                tr("Reason")
                            ),
                        );
                    }
                }
            } else {
                toast::async_toast_warn(
                    ui.clone(),
                    format!("{} - {}", item.name, tr("MCP server tool not found"),),
                );
            }
        }
    }

    _ = slint::invoke_from_event_loop(move || {
        update_db_entry(&ui.unwrap());
    });
}
