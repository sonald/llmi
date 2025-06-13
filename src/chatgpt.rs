use async_trait::async_trait;
use regex::Regex;
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::json;
use std::{collections::HashMap, error::Error, io::Result as IoResult, sync::Arc}; // Added Arc, removed env
use tokio::sync::mpsc::UnboundedSender;

use crate::config::Config; // Added
use crate::event::Event;
use crate::llm::*;

#[derive(Debug)]
pub struct ChatGPT {
    cli: Client,
    config: Arc<Config>, // Added
}

impl ChatGPT {
    // Modified to accept Arc<Config>
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            cli: Client::new(),
            config, // Store config
        }
    }
}

#[async_trait]
impl LLMService for ChatGPT {
    async fn request(
        &mut self,
        prompt: &str,
        mut history: Vec<Message>,
        tx: UnboundedSender<Event>,
    ) -> Result<(), Box<dyn Error>> {
        // Use config fields instead of env::var
        let endpoint = &self.config.llm_endpoint;
        let api_key = &self.config.llm_api_key;
        let model = &self.config.llm_model;

        history.push(Message::user(prompt.to_owned()));
        let messages = history
            .iter()
            .map(|msg| {
                // Assuming msg.role and msg.content are Option<String>
                // If they are guaranteed to be Some, .clone() is fine.
                // If they can be None, this would panic. Let's assume they should exist.
                // For robust error handling, consider checking for None here.
                let role = msg.role.as_ref().ok_or_else(|| "Message role is missing".to_string())?;
                let content = msg.content.as_ref().ok_or_else(|| "Message content is missing".to_string())?;
                let mut hm = HashMap::new();
                hm.insert("role", role.clone());
                hm.insert("content", content.clone());
                Ok(hm)
            })
            .collect::<Result<Vec<_>, String>>() // Collect results, propagating the first error
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;


        let data = json!({
            "model": model, // model is already a String from config
            "stream": true,
            "max_tokens": 3000,
            "messages": messages
        });

        tx.send(Event::LLMEventStart).map_err(|e| format!("Failed to send LLMEventStart: {}", e))?;

        let resp = self
            .cli
            .post(&endpoint) // endpoint needs to be a reference if it's a String
            .bearer_auth(&api_key) // api_key needs to be a reference if it's a String
            .header(CONTENT_TYPE, "application/json")
            .json(&data)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        match resp.error_for_status_ref() { // Use error_for_status_ref to avoid consuming resp
            Err(e) => {
                let error_message = format!("API request failed: {}", e);
                tx.send(Event::Notification(error_message.clone())).map_err(|e_send| format!("Failed to send error notification: {}", e_send))?;
                tx.send(Event::LLMEventEnd).map_err(|e_send| format!("Failed to send LLMEventEnd: {}", e_send))?; // Also send End event
                return Err(error_message.into());
            }
            Ok(_) => {} // Continue if Ok
        }

        // Must consume the response body now
        let mut resp_stream = resp;

        while let Some(bytes_result) = resp_stream.chunk().await {
            let bytes = bytes_result.map_err(|e| format!("Failed to read chunk: {}", e))?;
            let str_slice = std::str::from_utf8(&bytes).map_err(|e| format!("Failed to parse UTF-8 from chunk: {}", e))?;

            // Regex creation can fail, though unlikely for a fixed pattern.
            let re = Regex::new(r"data:\s(.*)").map_err(|e| format!("Failed to compile regex: {}", e))?;

            for caps in re.captures_iter(str_slice) {
                let payload = caps.get(1).map_or("", |m| m.as_str());
                if payload == "[DONE]" {
                    tx.send(Event::LLMEventEnd).map_err(|e| format!("Failed to send LLMEventEnd: {}", e))?;
                } else if !payload.is_empty() {
                    match serde_json::from_str::<LLMResponse>(payload) {
                        Ok(data) => {
                            if !data.choices.is_empty() {
                                tx.send(Event::LLMEventDelta(data.extract_message()))
                                    .map_err(|e| format!("Failed to send LLMEventDelta: {}", e))?;
                            }
                        }
                        Err(e) => {
                            // Decide if this is a critical error. For now, let's log it or send a notification.
                            // If it's critical, propagate it.
                            // For now, let's assume it might be a malformed part of the stream and try to continue.
                            // Consider sending a specific notification for parsing errors.
                            let parse_error_msg = format!("Failed to parse LLM response chunk: {}", e);
                            tx.send(Event::Notification(parse_error_msg)).map_err(|e_send| format!("Failed to send parse error notification: {}", e_send))?;
                        }
                    }
                }
            }
        }
        // Ensure LLMEventEnd is sent if the loop finishes without "[DONE]" explicitly seen,
        // though typically the stream should end with "[DONE]".
        // However, the original code sent LLMEventEnd only upon seeing "[DONE]" or error.
        // Adding a final send here might be redundant if "[DONE]" is guaranteed.
        // For now, keeping behavior closer to original: LLMEventEnd is sent on "[DONE]" or error.
        Ok(())
    }
}
