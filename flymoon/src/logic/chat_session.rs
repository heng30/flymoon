use super::{md, toast, tr::tr};
use crate::{
    config::{data::Model as SettingModel, model as setting_model},
    db::{
        self,
        def::{ChatEntry, ChatSession, CHAT_SESSION_TABLE as DB_TABLE},
    },
    slint_generatedAppWindow::{
        AppWindow, ChatEntry as UIChatEntry, ChatPhase, ChatSession as UIChatSession, Logic,
        MCPElement as UIMCPElement, MCPEntry as UIMCPEntry, PromptEntry as UIPromptEntry,
        PromptType, SearchLink as UISearchLink, Store,
    },
    store_mcp_entries, store_prompt_entries, toast_success, toast_warn,
};
use bot::openai::{
    request::{APIConfig as ChatAPIConfig, HistoryChat},
    response::StreamTextItem,
    Chat,
};
use cutil::time::chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use uuid::Uuid;

struct ChatCache {
    id: u64,
    ui: Weak<AppWindow>,
    stop_tx: Arc<mpsc::Sender<()>>,
    reasoner_start: Option<DateTime<Utc>>,
    bot_text: String,
}

const MCP_TOOL_START_SEP: &'static str = "```";
const MCP_TOOL_END_SEP: &'static str = "```";

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

#[macro_export]
macro_rules! store_current_chat_session_histories_mcp {
    ($entry:expr) => {
        $entry
            .mcp
            .as_any()
            .downcast_ref::<VecModel<UIMCPElement>>()
            .expect("We know we set UIMCPElement a VecModel earlier")
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
        let mcp_resp = if entry.mcp.row_count() > 0 {
            entry
                .mcp
                .iter()
                .map(|entry| entry.resp.clone().to_string())
                .collect::<String>()
        } else {
            String::default()
        };

        let btext = format!("{}\n\n{}", entry.bot, mcp_resp).into();

        HistoryChat {
            utext: entry.user.into(),
            btext,
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
            prompt_type: entry.prompt_type,
            mcp_config: entry.mcp_config.into(),
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
            prompt_type: entry.prompt_type,
            mcp_config: entry.mcp_config.into(),
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
    ui.global::<Logic>().on_copy_last_bot_text(move || {
        let ui = ui_handle.unwrap();
        let index = store_current_chat_session_histories!(ui).row_count();
        if index <= 0 {
            return;
        }

        let entry = store_current_chat_session_histories!(ui)
            .row_data(index - 1)
            .unwrap();

        ui.global::<Logic>().invoke_copy_to_clipboard(entry.bot);
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
            session.prompt_type = PromptType::Normal;
            ui.global::<Store>().set_current_chat_session(session);

            toast_success!(ui, tr("Clear current session prompt successfully"));
        });
}

fn parse_prompt(
    ui: &AppWindow,
    question: SharedString,
) -> (SharedString, SharedString, Option<f32>) {
    let mut session = store_current_chat_session!(ui);

    if question.is_empty() || (!question.starts_with("/") && !question.starts_with("@")) {
        return (session.prompt, question, None);
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
                session.prompt_type = PromptType::Normal;
                ui.global::<Store>().set_current_chat_session(session);
                return (entry.detail, question, Some(entry.temperature));
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

                session.prompt_type = PromptType::MCP;
                session.mcp_config = entry.config;
                ui.global::<Store>().set_current_chat_session(session);
                return (Default::default(), question, Some(entry.temperature));
            }
        }
    }

    (session.prompt, question, None)
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

        let chat_phase = ui.global::<Store>().get_chat_phase();

        if item.reasoning_text.is_some() {
            if chat_phase != ChatPhase::Chatting && chat_phase != ChatPhase::MCP {
                ui.global::<Store>().set_chat_phase(ChatPhase::Chatting);
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
                if chat_phase != ChatPhase::Chatting && chat_phase != ChatPhase::MCP {
                    ui.global::<Store>().set_chat_phase(ChatPhase::Chatting);
                }

                md::parse_stream_bot_text(&ui);
            }
        }
    });
}

async fn search_webpages(
    ui: Weak<AppWindow>,
    question: &str,
    histories: &mut Vec<HistoryChat>,
) -> bool {
    log::info!("start searching wabpages...");

    async_update_chat_phase(ui.clone(), ChatPhase::Searching);
    let config = setting_model().into();

    match search::google::search(question, config).await {
        Ok((Some(text), search_links)) => {
            log::info!("webpages content length: {}", text.len());
            log::info!("finished searching webpages");

            let text = format!(
                "The following web contents are relevant to the user's question. Please consult these resources when preparing your answer. {text}"
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
        Err(e) => {
            async_update_chat_phase(ui.clone(), ChatPhase::None);
            toast::async_toast_warn(
                ui.clone(),
                format!("{}. {}: {e:?}", tr("Search webpages failed"), tr("Reason")),
            );
            return false;
        }
        _ => (),
    }

    true
}

fn chat_histories(ui: &AppWindow, question: SharedString) -> Vec<HistoryChat> {
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
        user: question,
        md_elems: ModelRc::new(VecModel::from(vec![])),
        link_urls: ModelRc::new(VecModel::from(vec![])),
        search_links: ModelRc::new(VecModel::from(vec![])),
        mcp: ModelRc::new(VecModel::from(vec![])),
        ..Default::default()
    });

    if is_new_chat {
        add_db_entry(ui);
    }

    histories
}

async fn create_mcp_client(
    ui: Weak<AppWindow>,
    config: &str,
) -> (Option<mcp::Client>, Option<String>) {
    async_update_chat_phase(ui.clone(), ChatPhase::MCP);

    match mcp::create_mcp_client(config).await {
        Ok(client) => match gen_mcp_prompt(&client) {
            Some(prompt) => {
                async_set_current_chat_session_prompt(ui.clone(), prompt.clone().into());

                return (Some(client), Some(prompt));
            }
            _ => {
                toast::async_toast_warn(ui.clone(), format!("{}", tr("No MCP server tools")));
            }
        },
        Err(e) => {
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

    async_update_chat_phase(ui.clone(), ChatPhase::None);
    (None, None)
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

async fn start_chat(ui: Weak<AppWindow>, chat: Chat, id: u64, mcp_client: Option<mcp::Client>) {
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
        _ => {
            if mcp_client.is_some() {
                call_mcp_server_tool(ui.clone(), mcp_client.unwrap(), id).await;
            }
        }
    }

    async_update_chat_phase(ui, ChatPhase::None);
}

fn send_question(ui: &AppWindow, question: SharedString) {
    let (mut prompt, question, temperature) = parse_prompt(ui, question);
    let mut histories = chat_histories(ui, question.clone());

    let mcp_config = store_current_chat_session!(ui).mcp_config;
    let prompt_type = store_current_chat_session!(ui).prompt_type;

    let enabled_reasoner_model = ui.global::<Store>().get_enabled_reasoner_model();
    let enabled_search_webpages = ui.global::<Store>().get_enabled_search_webpages();

    let ui = ui.as_weak();
    tokio::spawn(async move {
        if enabled_search_webpages && !search_webpages(ui.clone(), &question, &mut histories).await
        {
            return;
        }

        let mut mcp_client = None;
        if !mcp_config.is_empty() && prompt_type == PromptType::MCP {
            log::info!("start create mcp client...");
            match create_mcp_client(ui.clone(), &mcp_config).await {
                (Some(client), Some(p)) => {
                    mcp_client = Some(client);
                    prompt = p.into();
                }
                _ => return,
            }
        }

        log::info!("start sending question to model...");
        let (chat, id) = prepare_chat(
            ui.clone(),
            prompt,
            question,
            histories,
            temperature,
            enabled_reasoner_model,
        );

        start_chat(ui, chat, id, mcp_client).await;
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

fn async_update_db_entry(ui: Weak<AppWindow>) {
    _ = slint::invoke_from_event_loop(move || {
        update_db_entry(&ui.unwrap());
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
        "You are a assistant, you can help user to complete various tasks. You have the following tools to use:\n".to_string();

    for tool in tools {
        prompt.push_str(&format!(
            "\ntool name: {}\ndescription: {}\nparameters: {}\n",
            tool.name(),
            tool.description(),
            serde_json::to_string_pretty(&tool.parameters()).unwrap_or_default()
        ));
    }

    prompt.push_str(&format!(
        r#"
Each tool calling format:
{}
{{"name": "tool_name", "arguments": "tool_arguments"}}
{}
"#,
        MCP_TOOL_START_SEP, MCP_TOOL_END_SEP
    ));

    Some(prompt)
}

async fn call_mcp_server_tool(ui: Weak<AppWindow>, client: mcp::Client, id: u64) {
    let content = get_chat_cache_bot_text();
    if content.is_empty() {
        return;
    }

    log::info!("start call mcp server tool...");

    async_update_chat_phase(ui.clone(), ChatPhase::MCP);

    let tool_list = parse_tool_list(&content);
    pretty_mcp_tool_sep(ui.clone(), tool_list.clone());

    for text in &tool_list {
        if let Ok(item) = serde_json::from_str::<mcp::tool::ToolCall>(&text) {
            if item.name.is_empty() {
                continue;
            }

            if !is_current_chat(id) {
                return;
            }

            match client.tool_set.get_tool(&item.name) {
                Some(tool) => {
                    log::info!("tool: {}", item.name);
                    log::info!("tool arguments: {:?}", item.arguments);

                    match tool.call(item.arguments).await {
                        Ok(result) => {
                            if !is_current_chat(id) {
                                return;
                            }

                            add_mcp_tool_response(ui.clone(), item.name, result);
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
                }
                _ => {
                    toast::async_toast_warn(
                        ui.clone(),
                        format!("{} - {}", item.name, tr("MCP server tool not found"),),
                    );
                }
            }
        }
    }

    async_update_db_entry(ui);
}

fn pretty_mcp_tool_sep(ui: Weak<AppWindow>, tool_list: Vec<String>) {
    if tool_list.is_empty() {
        return;
    }

    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        let rows = store_current_chat_session_histories!(ui).row_count();
        if rows <= 0 {
            return;
        }

        let last_index = rows - 1;
        let mut entry = store_current_chat_session_histories!(ui)
            .row_data(last_index)
            .unwrap();

        for item in tool_list.iter() {
            entry.bot = entry
                .bot
                .replace(item.as_str(), &pretty_json(item.clone().into()))
                .into();
        }

        entry.bot = entry
            .bot
            .replace(MCP_TOOL_START_SEP, "```")
            .replace(MCP_TOOL_END_SEP, "```")
            .into();

        store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
        md::parse_last_history_bot_text(&ui);
    });
}

#[allow(dead_code)]
fn paser_mcp_response(result: &str) -> Option<String> {
    let response_text = match serde_json::from_str::<super::mcp::MCPResponse>(result) {
        Ok(v) => Some(
            v.content
                .into_iter()
                .filter(|item| item.content_type == "text")
                .map(|item| item.text)
                .collect::<Vec<String>>()
                .join("\n")
                .to_string(),
        ),
        _ => None,
    };

    response_text
}

fn add_mcp_tool_response(ui: Weak<AppWindow>, name: String, result: String) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();

        let rows = store_current_chat_session_histories!(ui).row_count();
        if rows <= 0 {
            return;
        }

        let last_index = rows - 1;
        let entry = store_current_chat_session_histories!(ui)
            .row_data(last_index)
            .unwrap();

        store_current_chat_session_histories_mcp!(entry).push(UIMCPElement {
            tool_name: name.into(),
            resp: pretty_json(result.into()),
        });

        store_current_chat_session_histories!(ui).set_row_data(last_index, entry);
        md::parse_stream_bot_text(&ui);
    });
}

fn pretty_json(content: SharedString) -> SharedString {
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(v) => serde_json::to_string_pretty(&v)
            .unwrap_or(content.into())
            .into(),
        _ => content,
    }
}

fn parse_tool_list(content: &str) -> Vec<String> {
    let content = content.replace("```json", "```");

    let re = Regex::new(&format!(
        r"(?s){}[\s]*(.*?)[\s]*{}",
        MCP_TOOL_START_SEP, MCP_TOOL_END_SEP
    ))
    .unwrap();

    re.captures_iter(&content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
}

fn get_chat_cache_bot_text() -> String {
    let cc = CHAT_CACHE.lock().unwrap();
    if cc.is_some() {
        cc.as_ref().unwrap().bot_text.clone()
    } else {
        String::default()
    }
}

fn is_current_chat(id: u64) -> bool {
    let cc = CHAT_CACHE.lock().unwrap();
    if cc.is_some() {
        cc.as_ref().unwrap().id == id
    } else {
        false
    }
}

fn async_update_chat_phase(ui: Weak<AppWindow>, phase: ChatPhase) {
    _ = slint::invoke_from_event_loop(move || {
        ui.unwrap().global::<Store>().set_chat_phase(phase);
    });
}

fn async_set_current_chat_session_prompt(ui: Weak<AppWindow>, prompt: SharedString) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();
        let mut session = ui.global::<Store>().get_current_chat_session();
        session.prompt = prompt;
        ui.global::<Store>().set_current_chat_session(session);
    });
}
