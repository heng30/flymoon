use bot::openai::{
    Chat,
    request::{APIConfig, HistoryChat},
    response::StreamTextItem,
};

fn stream_text(item: StreamTextItem) {
    println!("{item:?}");
}

#[cfg(feature = "test-bot")]
#[tokio::main]
async fn main() {
    env_logger::init();

    let api_key = std::env::var("OPENAI_API_KEY").expect("Missing DEEPSEEK_API_KEY in environment");

    let prompt = "Your are a chat bot.";
    let question = "hi";

    let config = APIConfig {
        api_base_url: "https://api.deepseek.com/v1".to_string(),
        api_model: "deepseek-chat".to_string(),
        api_key,
        temperature: 1.0_f32,
    };

    // let config = APIConfig {
    //     api_base_url: "https://api.deepseek.com/v1".to_string(),
    //     api_model: "deepseek-reasoner".to_string(),
    //     api_key,
    //     temperature: 1.0_f32,
    // };

    let histories = vec![HistoryChat {
        utext: "hi".to_string(),
        btext: "Hello! 👋 How can I assist you today? 😊".to_string(),
    }];

    let (chat, stop_tx) = Chat::new(prompt, question, config, histories);

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(100));
        _ = stop_tx.send(());
    });

    _ = chat.start(1, stream_text).await;
}
