pub mod def;

pub use sqldb::{create_db, entry};

pub async fn init(db_path: &str) {
    create_db(db_path).await.expect("create db");

    entry::new(def::PROMPT_TABLE)
        .await
        .expect("prompt table failed");

    entry::new(def::MCP_TABLE)
        .await
        .expect("mcp table failed");

    entry::new(def::CHAT_SESSION_TABLE)
        .await
        .expect("chat session table failed");
}
