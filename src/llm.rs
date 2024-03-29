use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use regex::Regex;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, io::Result};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::Event;

// LLMResponse Example:
// ```json
// {
//     "id":"chatcmpl-82ec8043-ef36-914d-b124-53f5cbffb9e9",
//     "object":"chat.completion","created":1711444186,
//     "model":"mixtral-8x7b-32768",
//     "choices":[
//         {"i ndex":0,
//         "message":{"role":"assistant","content":"Hello! How can I help you today? If you have any questions about a particular topic or just want to chat, I'm her e to assist. Let me know what's on your mind."},
//         "logprobs":null,
//         "finish_reason":"stop"
//     }],
//     "usage":{
//         "prompt_tokens":16,"prompt_time":0.005,"completion_tokens":42,"c ompletion_time":0.072,"total_tokens":58,"total_time":0.077
//     },
//     "system_fingerprint":"fp_13a4b82d64",
//     "x_groq":{"id":"2eDfhFtOnQU6ukxwCD0f6HWsM45"}
// }
// ```

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LLMResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    prompt_tokens: u64,
    prompt_time: f64,
    completion_tokens: u64,
    completion_time: f64,
    total_tokens: u64,
    total_time: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    index: u64,
    message: Option<Message>,
    delta: Option<Message>,
    logprobs: Option<()>,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Option<String>,
    pub content: Option<String>,
}

impl LLMResponse {
    pub fn new() -> Self {
        LLMResponse::default()
    }

    pub fn extract_message(&self) -> Message {
        if self.choices[0].message.is_some() {
            self.choices[0].message.clone().unwrap()
        } else {
            self.choices[0].delta.clone().unwrap()
        }
    }
}

impl Message {
    fn new(role: String, content: String) -> Self {
        Message {
            role: Some(role),
            content: Some(content),
        }
    }

    pub fn user(content: String) -> Self {
        Message::new("user".to_string(), content)
    }

    pub fn assistant(content: String) -> Self {
        Message::new("assistant".to_string(), content)
    }

    pub fn len_by_columns(&self, max_width: u16) -> usize {
        self.content
            .as_ref()
            .unwrap()
            .split('\n')
            .flat_map(|ln| {
                let len = ln.chars().count();
                let count = len / max_width as usize + 1;
                vec!['a'; count]
            })
            .count()
    }
}

impl Widget for &Message {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.content.is_none() {
            return;
        }

        let (align, title_color) = match self.role.as_deref() {
            Some("user") => (Alignment::Right, Color::Blue),
            _ => (Alignment::Left, Color::Green),
        };

        let block = Block::default()
            .title_top(self.role.as_ref().unwrap().as_str())
            .title_style(title_color)
            .title_alignment(align)
            .borders(Borders::ALL);

        let text = self
            .content
            .as_ref()
            .unwrap()
            .split('\n')
            .into_iter()
            .map(|line| Line::from(line))
            .collect::<Vec<_>>();
        // let text = self.content.clone().cyan();
        Paragraph::new(Text::from(text))
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

#[derive(Debug)]
pub struct ChatGPT {
    cli: Client,
}

impl ChatGPT {
    pub fn new() -> Self {
        Self { cli: Client::new() }
    }

    pub async fn request(&mut self, prompt: &str, tx: &UnboundedSender<Event>) -> Result<()> {
        let endpoint = env::var("LLM_ENDPOINT").unwrap_or("".to_owned());
        let api_key = env::var("LLM_API_KEY").unwrap_or("".to_owned());
        let model = env::var("LLM_MODEL").unwrap_or("mixtral-8x7b-32768".to_owned());

        let data = json!({
            "model": model,
            "stream": true,
            "messages": [
               { "role": "user", "content": prompt}
            ]
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
                                Err(e) => {
                                    // tx.send(Event::Notification(format!(
                                    //     "Error: {} data: | {} |",
                                    //     e, payload
                                    // )))
                                    // .unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
