#[cfg(test)]
mod tests {
    use bot::openai::{
        Chat,
        request::{APIConfig, HistoryChat},
        response::StreamTextItem,
    };

    fn stream_text(item: StreamTextItem) {
        // println!("{item:?}");
    }

    #[tokio::test]
    async fn main() {
        env_logger::init();

        let prompt = "Your are a chat bot.";
        let question = "show me a zig hello world example.";

        let config = APIConfig {
            api_base_url: "https://api.deepseek.com/v1".to_string(),
            api_model: "deepseek-chat".to_string(),
            api_key: "Your-API-Key".to_string(),
        };

        let histories = vec![HistoryChat {
            utext: "hi".to_string(),
            btext: "Hello! 👋 How can I assist you today? 😊".to_string(),
        }];

        let (chat, stop_tx) = Chat::new(prompt, question, config, histories);

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(10));
            _ = stop_tx.send(());
        });

        _ = chat.start("uuid".to_string(), stream_text).await;
    }
}
