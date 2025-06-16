use anyhow::Result;
use cutil::{http, reqwest};
use html2text::from_read;
use html5ever::tree_builder::TreeSink;
use scraper::{Html, HtmlTreeSink, Selector};
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
        let body_selector = Selector::parse("body").unwrap();
        let remove_selector = Selector::parse("a,script,svg,style").unwrap();
        let document = Html::parse_document(&html_content);

        if let Some(body) = document.select(&body_selector).next() {
            let body_html = body.html().to_string();
            let document = Html::parse_document(&body_html);

            let node_ids: Vec<_> = document.select(&remove_selector).map(|x| x.id()).collect();

            let sink = HtmlTreeSink::new(document);
            for id in node_ids {
                sink.remove_from_parent(&id);
            }

            let html = sink.0.borrow().html();
            let content = from_read(html.as_bytes(), 80)?.to_string();

            if !content.is_empty() {
                return Ok(Some(content));
            }
        }
    }

    Ok(None)
}
