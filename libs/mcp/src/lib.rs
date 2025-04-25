use anyhow::Result;
use config::McpServerConfig;
use once_cell::sync::Lazy;
use rmcp::{
    RoleClient,
    service::{RunningService, ServerSink},
};
use std::{collections::HashMap, sync::Mutex};
use tool::ToolSet;

mod config;
pub mod tool;

struct MCPClient {
    client: RunningService<RoleClient, ()>,
    tool_set: ToolSet,
}

pub struct Client {
    pub name: String,
    pub client: ServerSink,
    pub tool_set: ToolSet,
}

static MCP_CLIENTS: Lazy<Mutex<HashMap<String, MCPClient>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn create_mcp_client(config: &str) -> Result<Client> {
    let mcp_config = McpServerConfig::from_raw_str(config)?;

    {
        let clients = MCP_CLIENTS.lock().unwrap();
        if clients.contains_key(&mcp_config.name) {
            let mcp_client = clients.get(&mcp_config.name).unwrap();
            let (client, tool_set) = (
                mcp_client.client.peer().clone(),
                mcp_client.tool_set.clone(),
            );

            return Ok(Client {
                name: mcp_config.name,
                client,
                tool_set,
            });
        }
    }

    let client = mcp_config.start().await?;
    let peer = client.peer().clone();

    // init
    _ = peer.peer_info();

    let mut tool_set = ToolSet::default();
    let tools = tool::get_mcp_tools(peer.clone()).await?;
    for tool in tools {
        tool_set.add_tool(tool);
    }

    {
        let mut clients = MCP_CLIENTS.lock().unwrap();
        clients.insert(
            mcp_config.name.clone(),
            MCPClient {
                client,
                tool_set: tool_set.clone(),
            },
        );
    }

    Ok(Client {
        name: mcp_config.name,
        client: peer,
        tool_set,
    })
}

pub async fn cancel_mcp_client(name: &str) -> Result<()> {
    let client = {
        let mut clients = MCP_CLIENTS.lock().unwrap();
        clients.remove(name)
    };

    if let Some(client) = client {
        client.client.cancel().await?;
    }

    Ok(())
}

pub fn mcp_server_name_from_config(config: &str) -> Result<String> {
    let config = McpServerConfig::from_raw_str(config)?;
    Ok(config.name)
}

pub fn mcp_server_is_running(name: &str) -> bool {
    MCP_CLIENTS.lock().unwrap().contains_key(name)
}
