use anyhow::Result;
use config::McpServerConfig;
use once_cell::sync::Lazy;
use rmcp::{Peer, RoleClient, service::RunningService};
use std::{collections::HashMap, sync::Mutex};

mod config;

static MCP_CLIENTS: Lazy<Mutex<HashMap<String, RunningService<RoleClient, ()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn create_mcp_client(config: &str) -> Result<(String, Peer<RoleClient>)> {
    let mcp_config = McpServerConfig::from_raw_str(config)?;

    {
        let clients = MCP_CLIENTS.lock().unwrap();
        if clients.contains_key(&mcp_config.name) {
            let peer = clients.get(&mcp_config.name).unwrap().peer().clone();
            return Ok((mcp_config.name, peer));
        }
    }

    let client = mcp_config.start().await?;
    let peer = client.peer().clone();

    {
        let mut clients = MCP_CLIENTS.lock().unwrap();
        clients.insert(mcp_config.name.clone(), client);
    }

    Ok((mcp_config.name, peer))
}

pub async fn cancel_mcp_client(name: &str) -> Result<()> {
    let client = {
        let mut clients = MCP_CLIENTS.lock().unwrap();
        clients.remove(name)
    };

    if let Some(client) = client {
        client.cancel().await?;
    }

    Ok(())
}

pub fn mcp_server_name_from_config(config: &str) -> Result<String> {
    let config = McpServerConfig::from_raw_str(config)?;
    Ok(config.name)
}
