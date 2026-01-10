use bot::openai::{
    Chat, ChatConfig,
    request::{APIConfig, HistoryChat},
    response::StreamTextItem,
};

#[tokio::main]
async fn main() {
    env_logger::init();

    let api_key = std::env::var("OPENAI_API_KEY").expect("Missing OPENAI_API_KEY in environment");

    let prompt = "Your are a chat bot.";
    let question = "hi";

    let request_config = APIConfig {
        api_base_url: "https://api.deepseek.com/v1".to_string(),
        api_model: "deepseek-chat".to_string(),
        api_key,
        temperature: None,
    };

    // let config = APIConfig {
    //     api_base_url: "https://api.deepseek.com/v1".to_string(),
    //     api_model: "deepseek-reasoner".to_string(),
    //     api_key,
    //     temperature: None,
    // };

    let histories = vec![HistoryChat {
        utext: "hi".to_string(),
        btext: "Hello! 👋 How can I assist you today? 😊".to_string(),
    }];

    let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamTextItem>(100);

    let chat_config = ChatConfig { tx };
    let chat = Chat::new(prompt, question, chat_config, request_config, histories);

    let handle = tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            println!("{item:?}");
        }
    });

    if let Err(e) = chat.start().await {
        eprintln!("Chat error: {e:?}");
    }

    _ = handle.await;
}
