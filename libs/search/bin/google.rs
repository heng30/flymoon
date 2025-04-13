use anyhow::Result;
use search::google;

#[tokio::main]
async fn main() -> Result<()> {
    let query = "How to learn Rust programming language?";

    let config = google::Config {
        cx: "Your-CX".to_string(),
        api_key: "Your-API-Key".to_string(),
        num: 10,
    };

    let text = google::search(query, config).await?.unwrap();
    println!("{text}");

    Ok(())
}
