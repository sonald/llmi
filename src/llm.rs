use serde::{Deserialize, Serialize};

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
    usage: Usage,
    system_fingerprint: String,
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
    message: Message,
    logprobs: Option<()>,
    finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl LLMResponse {
    pub fn new() -> Self {
        LLMResponse::default()
    }

    pub fn extract_message(&self) -> Message {
        self.choices[0].message.clone()
    }
}

impl Message {
    fn new(role: String, content: String) -> Self {
        Message { role, content }
    }

    pub fn user(content: String) -> Self {
        Message::new("user".to_string(), content)
    }

    pub fn assistant(content: String) -> Self {
        Message::new("assistant".to_string(), content)
    }
}
