#[cfg(test)]
mod tests {
    use crate::llm::*;

    fn split_str_by_40_chars(input: &str, n: usize) -> Vec<String> {
        input
            .chars()
            .collect::<Vec<char>>()
            .chunks(n)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect()
    }

    #[test]
    fn llm_resolve_empty_delta() {
        let payload = r#"{"id":"chatcmpl-814de58f-d884-4f09-a0dd-276b02a04e41","object":"chat.completion.chunk","created":1711681319,"model":"mixtral-8x7b-32768","system_fingerprint":"fp_1cc6d039b0","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}],"x_groq":{"id":"req_01ht42gbv3ev59x93kqrkbstnr","usage":{"queue_time":0.060529005,"prompt_tokens":12,"prompt_time":0.005,"completion_tokens":164,"completion_time":0.289,"total_tokens":176,"total_time":0.294}}}"#;
        let msg = serde_json::from_str::<LLMResponse>(payload);
        // split_str_by_40_chars(payload, 60).iter().for_each(|s| {
        //     eprintln!("{}", s);
        // });
        eprintln!("msg: {:?}", msg);
        assert!(msg.is_ok());
    }

    #[test]
    fn llm_resolve_mid_delta() {
        let payload = r#"{"id":"chatcmpl-814de58f-d884-4f09-a0dd-276b02a04e41","object":"chat.completion.chunk","created":1711681319,"model":"mixtral-8x7b-32768","system_fingerprint":"fp_1cc6d039b0","choices":[{"index":0,"delta":{"content":" soft"},"logprobs":null,"finish_reason":null}]}"#;
        let msg = serde_json::from_str::<LLMResponse>(payload);
        eprintln!("msg: {:?}", msg);
        assert!(msg.is_ok());
    }
}
