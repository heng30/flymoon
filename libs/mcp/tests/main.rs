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

        let (name, peer) = create_mcp_client(config).await?;
        log::info!("{name}");

        let info = peer.peer_info();
        log::info!("{info:?}");

        let prompt = peer.list_all_prompts().await?;
        log::info!("{prompt:?}");

        Ok(())
    }
}
