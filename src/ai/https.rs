// 注意：此文件只在 mod.rs 中以 #[cfg(feature = "ai-https")] 引入

use serde::{Deserialize, Serialize};
use std::io::{self, Error};

use super::{AIClient, MockAIClient};

// ── 请求体 structs ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Clone)]
struct ExtraBody {
    enable_thinking: bool,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    session_id: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
    extra_body: ExtraBody,
    chat_template_kwargs: ExtraBody,
}

// ── 响应体 structs ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChatChoice {
    message: Option<ChatChoiceMessage>,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Option<Vec<ChatChoice>>,
    message: Option<String>,
}

// ── 私有发送方法 ──────────────────────────────────────────────────────────────

impl MockAIClient {
    fn send_request(&self, body: &ChatRequest) -> io::Result<ChatResponse> {
        let url = format!("{}{}", self.host, self.chat_url);
        let mut req = self.agent.post(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", &format!("Bearer {}", key));
        }
        let mut res = req
            .send_json(body)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        res.body_mut()
            .read_json::<ChatResponse>()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

// ── AIClient impl ─────────────────────────────────────────────────────────────

impl AIClient for MockAIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error> {
        self.chat(true, prompt)
    }

    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error> {
        let max_tokens = if is_completion {
            self.complete_max_tokens as u32
        } else {
            self.chat_max_tokens as u32
        };
        let system_content = format!(
            "{}\n{}",
            if is_completion {
                &self.hint_prompt
            } else {
                &self.chat_prompt
            },
            self.syntax
        );
        let extra_body = ExtraBody {
            enable_thinking: false,
        };
        let body = ChatRequest {
            model: self.model.clone(),
            session_id: self.session_id.clone(),
            max_tokens,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_content,
                },
                ChatMessage {
                    role: "user".into(),
                    content: prompt.to_string(),
                },
            ],
            extra_body: extra_body.clone(),
            chat_template_kwargs: extra_body,
        };

        let response = self.send_request(&body)?;
        if let Some(choices) = response.choices {
            if let Some(choice) = choices.into_iter().next() {
                if let Some(msg) = choice.message {
                    if let Some(content) = msg.content {
                        return Ok(content);
                    }
                }
            }
        } else if let Some(message) = response.message {
            return Ok(message);
        }
        Ok("Unknown Response Format".to_string())
    }
}
