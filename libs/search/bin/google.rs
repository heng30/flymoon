use anyhow::Result;
use search::google;

#[tokio::main]
async fn main() -> Result<()> {
    let query = "How to learn Rust?";

    let config = google::Config {
        cx: "a0e7eb672d5134b97".to_string(),
        api_key: "AIzaSyA7-Nzj5OPo6hpirlTepFRBXuWKTj42aio".to_string(),
        num: 10,
    };

    let text = google::search(query, config).await?.unwrap();
    println!("{text}");

    Ok(())
}
