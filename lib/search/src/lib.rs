use anyhow::Result;
use cutil::{http, reqwest};
use html2md::parse_html;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub mod google;

#[derive(Serialize, Debug, Clone)]
struct SearchItem {
    title: String,
    contents: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchLink {
    pub title: String,
    pub link: String,
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
        let filtered_html = strip_tags(&html_content, &["script"]);
        let md_contents = parse_html(&filtered_html);
        if !md_contents.is_empty() {
            return Ok(Some(md_contents));
        }
    }

    Ok(None)
}

fn strip_tags(html: &str, tags: &[&str]) -> String {
    let mut processed_html = html.to_string();
    for tag in tags {
        let re = Regex::new(&format!(r"(?is)<{}.*?>.*?</{}>", tag, tag)).unwrap();
        processed_html = re.replace_all(&processed_html, "").to_string();
    }
    processed_html
}
