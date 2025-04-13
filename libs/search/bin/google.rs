use anyhow::Result;
use search::google;

#[tokio::main]
async fn main() -> Result<()> {
    let query = "How to learn Rust programming language?";

    let config = google::Config {
        cx: "Your-cx",
        api_key: "Your-API-Key",
        num: 10,
    };

    let text = google::search(query, config).await?.unwrap();
    println!("{text}");

    Ok(())
}
