use super::{
    client::ChatClient,
    model::{CompletionRequest, Message},
};
use anyhow::Result;
use mcp::tool::{Tool as ToolTrait, ToolCall, ToolSet};
use std::{
    io::{self, Write},
    sync::Arc,
};

pub struct ChatSession {
    client: Arc<dyn ChatClient>,
    tool_set: ToolSet,
    model: String,
    messages: Vec<Message>,
}

impl ChatSession {
    pub fn new(client: Arc<dyn ChatClient>, tool_set: ToolSet, model: String) -> Self {
        Self {
            client,
            tool_set,
            model,
            messages: Vec::new(),
        }
    }

    pub fn add_system_prompt(&mut self, prompt: impl ToString) {
        self.messages.push(Message::system(prompt));
    }

    pub fn get_tools(&self) -> Vec<Arc<dyn ToolTrait>> {
        self.tool_set.tools()
    }

    pub async fn chat(&mut self) -> Result<()> {
        println!("welcome to use simple chat client, use 'exit' to quit");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input = input.trim().to_string();

            if input.is_empty() {
                continue;
            }

            if input == "exit" {
                break;
            }

            self.messages.push(Message::user(&input));

            // create request
            let request = CompletionRequest {
                model: self.model.clone(),
                messages: self.messages.clone(),
                temperature: Some(0.7),
            };

            // {
            //     let s = serde_json::to_string(&request);
            //     println!("{}", s.unwrap());
            // }

            // send request
            let response = self.client.complete(request).await?;

            if let Some(choice) = response.choices.first() {
                println!("AI:\n {}", choice.message.content);
                println!("==================");
                self.messages.push(choice.message.clone());

                let mut meet_tool = false;
                let mut tool_list = vec![];
                let mut tool_text = String::default();
                let lines: Vec<&str> = choice.message.content.split('\n').collect();

                for line in lines {
                    if line.starts_with("ToolStart") {
                        meet_tool = true;
                    } else if line.starts_with("ToolEnd") {
                        tool_list.push(tool_text.clone());
                        meet_tool = false;
                        tool_text.clear();
                    } else if meet_tool {
                        tool_text.push_str(&line);
                    }
                }

                for text in &tool_list {
                    if let Ok(item) = serde_json::from_str::<ToolCall>(&text) {
                        if item.name.is_empty() {
                            continue;
                        }

                        if let Some(tool) = self.tool_set.get_tool(&item.name) {
                            println!("calling tool: {}", item.name);
                            println!("tool args: {:?}", item.arguments);

                            match tool.call(item.arguments).await {
                                Ok(result) => {
                                    println!("tool result: {}", result);
                                    self.messages.push(Message::user(result));
                                }
                                Err(e) => {
                                    println!("tool call failed: {}", e);
                                    self.messages
                                        .push(Message::user(format!("tool call failed: {}", e)));
                                }
                            }
                        } else {
                            println!("tool not found: {}", item.name);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
