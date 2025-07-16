use anyhow::Result;
use rmcp::{service::RunningService, RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Stdio};

#[derive(Debug, Serialize, Deserialize)]
struct RawMcpServerConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: RawMcpServers,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawMcpServers {
    #[serde(flatten)]
    servers: HashMap<String, RawServerConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawServerConfig {
    #[serde(default)]
    command: String,

    #[serde(default)]
    url: String,

    #[serde(default)]
    args: Vec<String>,

    #[serde(default)]
    env: HashMap<String, String>,
}

impl RawMcpServerConfig {
    fn from_str(content: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(content)?;
        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct McpServerConfig {
    pub(crate) name: String,
    #[serde(flatten)]
    transport: McpServerTransportConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
enum McpServerTransportConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
}

impl McpServerConfig {
    #[allow(dead_code)]
    fn from_str(content: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(content)?;
        Ok(config)
    }

    pub(crate) fn from_raw_str(content: &str) -> Result<Self> {
        let raw_config: RawMcpServerConfig = RawMcpServerConfig::from_str(content)?;

        let mut name: String = String::default();
        let mut transport: McpServerTransportConfig = McpServerTransportConfig::Sse {
            url: Default::default(),
        };

        if raw_config.mcp_servers.servers.contains_key("url") {
            for (k, v) in raw_config.mcp_servers.servers.into_iter() {
                name = k;
                transport = McpServerTransportConfig::Sse { url: v.url };
            }
        } else {
            for (k, v) in raw_config.mcp_servers.servers.into_iter() {
                name = k;
                transport = McpServerTransportConfig::Stdio {
                    command: v.command,
                    args: v.args,
                    env: v.env,
                };
            }
        }
        Ok(McpServerConfig { name, transport })
    }

    pub(crate) async fn start(&self) -> Result<RunningService<RoleClient, ()>> {
        let client = match &self.transport {
            McpServerTransportConfig::Sse { url } => {
                let transport =
                    rmcp::transport::sse_client::SseClientTransport::start(url.as_str()).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio { command, args, env } => {
                let mut cmd = tokio::process::Command::new(command);
                cmd.args(args)
                    .envs(env)
                    .stderr(Stdio::inherit())
                    .stdout(Stdio::inherit());

                let transport = rmcp::transport::child_process::TokioChildProcess::new(cmd)?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_mcp_config_stdio() -> Result<()> {
        let stdio_json_data = r#"
        {
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": [
                        "-y",
                        "@modelcontextprotocol/server-filesystem",
                        "/Users/username/Desktop",
                        "/path/to/other/allowed/dir"
                    ],
                    "env": {
                        "APIFY_TOKEN": "your-apify-token"
                    }
                }
            }
        }
        "#;

        let parsed: RawMcpServerConfig = RawMcpServerConfig::from_str(stdio_json_data)?;
        println!("{:#?}", parsed);

        for (k, v) in parsed.mcp_servers.servers.iter() {
            println!("name: {k}");
            println!("Command: {}", v.command);
            println!("Args: {:?}", v.args);
            println!("Env: {:?}", v.env);
        }

        Ok(())
    }

    #[test]
    fn raw_mcp_config_sse() -> Result<()> {
        let sse_json_data = r#"
        {
            "mcpServers": {
                "apify": {
                    "url": "http://localhost:8000/sse",
                    "env": {
                        "APIFY_TOKEN": "your-apify-token"
                    }
                }
            }
        }
        "#;

        let parsed: RawMcpServerConfig = RawMcpServerConfig::from_str(sse_json_data)?;
        println!("{:#?}", parsed);

        for (k, v) in parsed.mcp_servers.servers.iter() {
            println!("name: {k}");
            println!("url: {}", v.url);
            println!("Env: {:?}", v.env);
        }

        Ok(())
    }

    #[test]
    fn mcp_config_stdio_from_raw_str() -> Result<()> {
        let stdio_json_data = r#"
        {
            "mcpServers": {
                "filesystem": {
                    "command": "uvx",
                    "args": [
                        "@modelcontextprotocol/server-filesystem"
                    ],
                    "env": {
                        "APIFY_TOKEN": "your-apify-token"
                    }
                }
            }
        }
        "#;

        let parsed: McpServerConfig = McpServerConfig::from_raw_str(stdio_json_data)?;
        println!("{:#?}", parsed);
        Ok(())
    }

    #[test]
    fn mcp_config_sse_from_raw_str() -> Result<()> {
        let sse_json_data = r#"
        {
            "mcpServers": {
                "apify": {
                    "url": "http://localhost:8080/sse",
                    "env": {
                        "APIFY_TOKEN": "your-apify-token"
                    }
                }
            }
        }
        "#;

        let parsed: RawMcpServerConfig = RawMcpServerConfig::from_str(sse_json_data)?;
        println!("{:#?}", parsed);

        Ok(())
    }

    #[test]
    fn mcp_config() -> Result<()> {
        let stdio_json_data = r#"
        {
            "name": "MCP stdio server name",
            "protocol": "stdio",
            "command": "MCP server path",
            "args": ["foo"]
        }
        "#;

        let sse_json_data = r#"
        {
            "name": "MCP SSE server name",
            "protocol": "sse",
            "url": "http://localhost:8000/sse"
        }
        "#;

        let config_stdio = McpServerConfig::from_str(stdio_json_data)?;
        println!("{config_stdio:#?}");

        let config_sse = McpServerConfig::from_str(sse_json_data)?;
        println!("{config_sse:#?}");

        Ok(())
    }
}
