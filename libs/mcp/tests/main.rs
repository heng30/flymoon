// RUST_LOG=debug cargo test main -- --nocapture

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use mcp::*;

    #[tokio::test]
    async fn main() -> Result<()> {
        let config = r#"
        {
            "mcpServers": {
                "fetch": {
                    "command": "uvx",
                    "args": [
                        "mcp-server-fetch"
                    ]
                }
            }
        }
        "#;

        env_logger::init();

        let client = create_mcp_client(config).await?;
        log::info!("{}", client.name);

        let info = client.client.peer_info();
        log::info!("{info:?}");

        let prompt = client.client.list_all_prompts().await?;
        log::info!("{prompt:?}");

        Ok(())
    }
}
