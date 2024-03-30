use async_trait::async_trait;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use serde::{Deserialize, Serialize};
use std::io::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::{chatgpt::ChatGPT, event::Event};

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
    pub(crate) choices: Vec<Choice>,
    pub(crate) usage: Option<Usage>,
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

#[async_trait]
pub trait LLMService: Send + Sync {
    async fn request(
        &mut self,
        prompt: &str,
        mut history: Vec<Message>,
        tx: UnboundedSender<Event>,
    ) -> Result<()>;
}

pub struct LLMProvider {}

impl LLMProvider {
    pub fn new() -> Box<dyn LLMService> {
        Box::new(ChatGPT::new())
    }
}
