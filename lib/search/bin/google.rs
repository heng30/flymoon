use anyhow::Result;
use search::google;

#[tokio::main]
async fn main() -> Result<()> {
    let query = "How to learn Rust?";

    let config = google::Config {
        cx: "a0e7eb672d5134b97".to_string(),
        api_key: "Your-API-Key".to_string(),
        num: 3,
    };

    let (text, links) = google::search(query, config).await?;
    println!("{text:?}");
    println!("{links:?}");

    Ok(())
}
