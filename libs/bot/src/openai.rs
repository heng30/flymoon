use anyhow::Result;
use cutil::reqwest::{
    self,
    header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, HeaderMap},
};
use log::debug;
use std::sync::mpsc;
use std::time::Duration;
use tokio_stream::StreamExt;

pub mod request {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Clone, Debug)]
    pub struct HistoryChat {
        pub utext: String,
        pub btext: String,
    }

    #[derive(Serialize, Deserialize, Default, Debug, Clone)]
    pub struct APIConfig {
        pub api_base_url: String,
        pub api_model: String,
        pub api_key: String,
        pub temperature: Option<f32>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub(crate) struct ChatCompletion {
        pub messages: Vec<Message>,
        pub model: String,
        pub stream: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub temperature: Option<f32>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub(crate) struct Message {
        pub role: String,
        pub content: String,
    }
}

pub mod response {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Default, Clone, Debug)]
    pub struct StreamTextItem {
        pub id: u64,
        pub text: Option<String>,
        pub reasoning_text: Option<String>,
        pub etext: Option<String>,
        pub finished: bool,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ChunkChoice {
        pub index: usize,
        pub delta: HashMap<String, Option<String>>,
        pub finish_reason: Option<String>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ChatCompletionChunk {
        pub id: String,
        pub object: String,
        pub created: i64,
        pub model: String,
        pub choices: Vec<ChunkChoice>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Error {
        pub error: HashMap<String, String>,
    }
}

#[derive(Debug)]
pub struct Chat {
    pub config: request::APIConfig,
    messages: Vec<request::Message>,
    stop_rx: mpsc::Receiver<()>,
}

impl Chat {
    pub fn new(
        prompt: impl ToString,
        question: impl ToString,
        config: request::APIConfig,
        chats: Vec<request::HistoryChat>,
    ) -> (Chat, mpsc::Sender<()>) {
        let (stop_tx, stop_rx) = mpsc::channel();

        let mut messages = vec![];
        messages.push(request::Message {
            role: "system".to_string(),
            content: prompt.to_string(),
        });

        for item in chats.into_iter() {
            messages.push(request::Message {
                role: "user".to_string(),
                content: item.utext,
            });

            messages.push(request::Message {
                role: "assistant".to_string(),
                content: item.btext,
            })
        }

        messages.push(request::Message {
            role: "user".to_string(),
            content: question.to_string(),
        });

        (
            Chat {
                messages,
                config,
                stop_rx,
            },
            stop_tx,
        )
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.config.api_key).parse().unwrap(),
        );
        headers.insert(ACCEPT, "text/event-stream".parse().unwrap());
        headers.insert(CACHE_CONTROL, "no-cache".parse().unwrap());

        headers
    }

    pub async fn start(self, id: u64, cb: impl Fn(response::StreamTextItem)) -> Result<()> {
        let headers = self.headers();
        let client = reqwest::Client::new();

        let url = format!("{}{}", self.config.api_base_url, "/chat/completions");
        let request_body = request::ChatCompletion {
            messages: self.messages,
            model: self.config.api_model,
            temperature: self.config.temperature,
            stream: true,
        };

        let mut stream = client
            .post(url)
            .headers(headers)
            .json(&request_body)
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .bytes_stream();

        loop {
            if self.stop_rx.try_recv().is_ok() {
                debug!("stopped by channel");
                break;
            }

            match stream.next().await {
                Some(Ok(chunk)) => {
                    let body = String::from_utf8_lossy(&chunk);

                    // debug!("{body:?}");

                    if let Ok(err) = serde_json::from_str::<response::Error>(&body) {
                        if let Some(estr) = err.error.get("message") {
                            cb(response::StreamTextItem {
                                etext: Some(estr.clone()),
                                id,
                                ..Default::default()
                            });
                            debug!("{}", estr);
                        }
                        break;
                    }

                    if body.starts_with("data: [DONE]") {
                        break;
                    }

                    let lines: Vec<_> = body.split("\n\n").collect();

                    for line in lines.into_iter() {
                        if !line.starts_with("data:") {
                            continue;
                        }

                        match serde_json::from_str::<response::ChatCompletionChunk>(&line[5..]) {
                            Ok(chunk) => {
                                let choice = &chunk.choices[0];
                                if choice.finish_reason.is_some() {
                                    cb(response::StreamTextItem {
                                        id,
                                        finished: true,
                                        ..Default::default()
                                    });

                                    debug!(
                                        "finish_reason: {}",
                                        choice.finish_reason.as_ref().unwrap()
                                    );
                                    break;
                                }

                                if choice.delta.contains_key("content")
                                    && choice.delta["content"].is_some()
                                {
                                    cb(response::StreamTextItem {
                                        text: choice.delta["content"].clone(),
                                        id,
                                        ..Default::default()
                                    });
                                    // debug!("{:?}", choice.delta["content"]);
                                } else if choice.delta.contains_key("reasoning_content")
                                    && choice.delta["reasoning_content"].is_some()
                                {
                                    cb(response::StreamTextItem {
                                        reasoning_text: choice.delta["reasoning_content"].clone(),

                                        id,
                                        ..Default::default()
                                    });
                                    // debug!("{:?}", choice.delta["reasoning_content"]);
                                } else if choice.delta.contains_key("role") {
                                    debug!("role: {:?}", choice.delta["role"]);
                                    continue;
                                }
                            }
                            Err(e) => {
                                debug!("{e:?} {}", &line);
                                break;
                            }
                        }
                    }
                }
                Some(Err(_)) => (),
                None => {
                    break;
                }
            }
        }

        Ok(())
    }
}
