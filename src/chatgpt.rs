use async_trait::async_trait;
use regex::Regex;
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use std::{collections::HashMap, env, io::Result};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::Event;
use crate::llm::*;

#[derive(Debug)]
pub struct ChatGPT {
    cli: Client,
}

impl ChatGPT {
    pub fn new() -> Self {
        Self { cli: Client::new() }
    }
}

#[async_trait]
impl LLMService for ChatGPT {
    async fn request(
        &mut self,
        prompt: &str,
        mut history: Vec<Message>,
        tx: UnboundedSender<Event>,
    ) -> Result<()> {
        let endpoint = env::var("LLM_ENDPOINT").unwrap_or("".to_owned());
        let api_key = env::var("LLM_API_KEY").unwrap_or("".to_owned());
        let model = env::var("LLM_MODEL").unwrap_or("mixtral-8x7b-32768".to_owned());

        history.push(Message::user(prompt.to_owned()));
        let messages = history
            .iter()
            .map(|msg| {
                let mut hm = HashMap::new();
                hm.insert("role", msg.role.clone().unwrap());
                hm.insert("content", msg.content.clone().unwrap());
                hm
            })
            .collect::<Vec<_>>();

        let data = json!({
            "model": model,
            "stream": true,
            "max_tokens": 3000,
            "messages": messages
        });

        tx.send(Event::LLMEventStart).unwrap();

        let resp = self
            .cli
            .post(endpoint)
            .bearer_auth(api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .await
            .unwrap();

        match resp.error_for_status() {
            Err(_e) => {
                tx.send(Event::LLMEventEnd).unwrap();
            }
            Ok(mut resp) => {
                while let Some(bytes) = resp.chunk().await.unwrap() {
                    let str = std::str::from_utf8(&bytes).unwrap();
                    let re = Regex::new(r"data:\s(.*)").unwrap();

                    for caps in re.captures_iter(str) {
                        let (_, [payload]) = caps.extract();
                        if payload == "[DONE]" {
                            tx.send(Event::LLMEventEnd).unwrap();
                        } else {
                            match serde_json::from_str::<LLMResponse>(payload) {
                                Ok(data) => {
                                    assert!(data.choices.len() > 0);
                                    tx.send(Event::LLMEventDelta(data.extract_message()))
                                        .unwrap();
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
