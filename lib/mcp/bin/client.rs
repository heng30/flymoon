use anyhow::Result;
use async_trait::async_trait;
use cutil::reqwest::Client as HttpClient;

use crate::model::{CompletionRequest, CompletionResponse};

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
}

pub struct OpenAIClient {
    api_key: String,
    client: HttpClient,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, url: String) -> Self {
        Self {
            api_key,
            base_url: url,
            client: HttpClient::new(),
        }
    }
}

#[async_trait]
impl ChatClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("API error: {}", error_text);
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }
}
