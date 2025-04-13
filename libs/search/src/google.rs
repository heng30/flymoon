use super::{SearchItem, req_link};
use anyhow::Result;
use cutil::reqwest;
use serde::Deserialize;
use std::sync::{Arc, mpsc::channel};

#[derive(Deserialize, Debug)]
struct SearchResult {
    items: Vec<SearchResultItem>,
}

#[derive(Deserialize, Debug)]
struct SearchResultItem {
    title: String,
    link: String,
}

#[derive(Debug)]
pub struct Config {
    pub cx: String,
    pub api_key: String,
    pub num: u8,
}

pub async fn search(query: &str, config: Config) -> Result<Option<String>> {
    let url = format!(
        "https://www.googleapis.com/customsearch/v1?key={}&cx={}&num={}&start=1&q={}",
        config.api_key,
        config.cx,
        config.num,
        urlencoding::encode(query)
    );

    let gs = reqwest::get(&url).await?.json::<SearchResult>().await?;
    log::debug!("{:#?}", gs);

    let (sender, receiver) = channel();
    let (sender, mut bris) = (Arc::new(sender), vec![]);

    for item in gs.items.into_iter() {
        if item.link.is_empty() {
            continue;
        }

        let sender = sender.clone();
        tokio::spawn(async move {
            if let Ok(Some(contents)) = req_link(&item.link).await {
                _ = sender.send(SearchItem {
                    title: item.title,
                    contents,
                });
            }
        });
    }

    drop(sender);

    for item in receiver {
        bris.push(item);
    }

    if bris.is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::to_string(&bris).unwrap()))
}
