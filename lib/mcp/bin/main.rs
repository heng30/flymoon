mod chat;
mod client;
mod model;

use anyhow::Result;
use chat::ChatSession;
use client::OpenAIClient;
use std::sync::Arc;

struct ChatConfig {
    api_key: String,
    url: String,
    model_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mcp_config = r#"
        {
            "mcpServers": {
                "fetch": {
                    "command": "npx",
                    "args": [
                        "-y",
                        "@modelcontextprotocol/server-filesystem",
                        "/tmp"
                    ]
                }
            }
        }
        "#;

    let client = mcp::create_mcp_client(mcp_config).await?;
    println!("{}", client.name);

    let chat_config = ChatConfig {
        api_key: "Your-API-Key".to_string(),
        url: "https://api.deepseek.com/v1/chat/completions".to_string(),
        model_name: "deepseek-chat".to_string(),
    };

    let chat_client = Arc::new(OpenAIClient::new(chat_config.api_key, chat_config.url));

    let mut session =
        ChatSession::new(chat_client, client.tool_set.clone(), chat_config.model_name);

    let mut system_prompt =
        "you are a assistant, you can help user to complete various tasks. you have the following tools to use:\n".to_string();

    for tool in session.get_tools() {
        system_prompt.push_str(&format!(
            "\ntool name: {}\ndescription: {}\nparameters: {}\n",
            tool.name(),
            tool.description(),
            serde_json::to_string_pretty(&tool.parameters()).unwrap_or_default()
        ));
    }

    system_prompt.push_str(
        r#"\nif you need to call tools, please use the following format:\n
        ToolStart\n
        {"name": "tool_name", "arguments": "tool_arguments"}\n
        ToolEnd\n"#,
    );

    session.add_system_prompt(system_prompt);
    session.chat().await?;

    Ok(())
}
