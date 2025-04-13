use anyhow::Result;
use cutil::{http, reqwest};
use scraper::{Html, Selector};
use serde::Serialize;

pub mod google;

#[derive(Serialize, Debug, Clone)]
struct SearchItem {
    title: String,
    contents: String,
}

async fn req_link(link: &str) -> Result<Option<String>> {
    let client = reqwest::Client::new();
    let headers = http::headers();

    if let Ok(html_content) = client
        .get(link)
        .headers(headers)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?
        .text()
        .await
    {
        let document = Html::parse_document(&html_content);

        if let Ok(p_selector) = Selector::parse("p") {
            let mut contents = String::default();

            for element in document.select(&p_selector) {
                contents.push_str(&element.text().collect::<String>());
            }

            let contents: String = contents.split_whitespace().collect::<Vec<&str>>().join(" ");

            if !contents.is_empty() {
                return Ok(Some(contents));
            }
        }
    }

    Ok(None)
}
