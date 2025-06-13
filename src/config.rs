use std::env;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)] // Added PartialEq for easier comparison
pub enum LlmProviderType {
    ChatGPT,
    // Ollama, // For future use
}

impl LlmProviderType {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "chatgpt" => Ok(LlmProviderType::ChatGPT),
            // "ollama" => Ok(LlmProviderType::Ollama),
            _ => Err(format!("Unsupported LLM provider type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub llm_endpoint: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub llm_provider: LlmProviderType, // Added field
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let llm_endpoint = env::var("LLM_ENDPOINT")
            .map_err(|_| "LLM_ENDPOINT environment variable not set")?;
        if llm_endpoint.is_empty() {
            return Err("LLM_ENDPOINT environment variable must not be empty".into());
        }

        let llm_api_key = env::var("LLM_API_KEY")
            .map_err(|_| "LLM_API_KEY environment variable not set")?;
        if llm_api_key.is_empty() {
            return Err("LLM_API_KEY environment variable must not be empty".into());
        }

        let mut actual_llm_model = env::var("LLM_MODEL").unwrap_or_else(|_| "mixtral-8x7b-32768".to_string());
        if actual_llm_model.is_empty() {
            actual_llm_model = "mixtral-8x7b-32768".to_string();
        }

        let llm_provider_str = env::var("LLM_PROVIDER").unwrap_or_else(|_| "chatgpt".to_string());
        let llm_provider = LlmProviderType::from_str(&llm_provider_str)
            .unwrap_or_else(|warning| {
                // Log a warning if an unsupported provider is specified, but default to ChatGPT.
                // In a real application, this might use a logging framework.
                // For now, printing to stderr is a simple way to provide feedback.
                eprintln!("Warning: {}", warning);
                eprintln!("Defaulting to ChatGPT provider.");
                LlmProviderType::ChatGPT
            });

        Ok(Self {
            llm_endpoint,
            llm_api_key,
            llm_model: actual_llm_model,
            llm_provider,
        })
    }
}
