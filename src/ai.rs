use std::io::Error;
use std::io::{self, BufRead, Write};
use std::net::TcpStream;
use std::time::Duration;
use tinyjson::JsonValue;

use crate::Expression;

pub fn init_ai(ai_cfg: Expression) -> MockAIClient {
    match ai_cfg {
        Expression::BMap(cfg_map) => MockAIClient {
            host: match cfg_map.as_ref().get("host") {
                Some(h) => h.to_string(),
                _ => "localhost:11434".into(),
            },
            complete_url: match cfg_map.as_ref().get("complete_url") {
                Some(h) => h.to_string(),
                _ => "/completion".into(),
            },
            chat_url: match cfg_map.as_ref().get("chat_url") {
                Some(h) => h.to_string(),
                _ => "/v1/chat/completions".into(),
            },
            complete_max_tokens: match cfg_map.as_ref().get("complete_max_tokens") {
                Some(h) => match h {
                    Expression::Integer(c_token) => *c_token as u8,
                    _ => 10,
                },
                _ => 10,
            },
            chat_max_tokens: match cfg_map.as_ref().get("chat_max_tokens") {
                Some(h) => match h {
                    Expression::Integer(c_token) => *c_token as u8,
                    _ => 100,
                },
                _ => 100,
            },
            model: match cfg_map.as_ref().get("model") {
                Some(h) => h.to_string(),
                _ => "".into(),
            },
            system_prompt: match cfg_map.as_ref().get("system_prompt") {
                Some(h) => h.to_string(),
                _ => "you're a lumesh shell helper".into(),
            },
        },
        _ => {
            eprintln!("invalid config:AI config should be a map.\nloading default.");
            MockAIClient::new(
                "localhost:11434".into(),
                "/completion".into(),
                "/v1/chat/completions".into(),
            )
        }
    }
}

pub trait AIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error>;
    fn chat(&self, prompt: &str) -> Result<String, Error>;
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String>;
}

pub struct MockAIClient {
    host: String,
    complete_url: String,
    chat_url: String,
    complete_max_tokens: u8,
    chat_max_tokens: u8,
    model: String,
    system_prompt: String,
}

impl MockAIClient {
    pub fn new(host: String, complete_url: String, chat_url: String) -> Self {
        Self {
            host,
            complete_url,
            chat_url,
            complete_max_tokens: 20,
            chat_max_tokens: 100,
            model: "".into(),
            system_prompt: "you're a shell helper".into(),
        }
    }
}

impl AIClient for MockAIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error> {
        let json_string = format!(
            r#"{{"model": "{}", "max_tokens": {},"prompt": "{}"}}"#,
            &self.model, self.complete_max_tokens, prompt
        );

        let completion_response = self.send_request(&self.complete_url, &json_string)?;

        match completion_response.parse::<JsonValue>() {
            Ok(parsed) => {
                if let JsonValue::Object(obj) = parsed {
                    if let Some(JsonValue::String(content)) = obj.get("content") {
                        return Ok(content.clone());
                    } else if let Some(JsonValue::Array(choices)) = obj.get("choices") {
                        if let Some(JsonValue::Object(choice)) = choices.first() {
                            if let Some(JsonValue::String(text)) = choice.get("text") {
                                return Ok(text.clone());
                            }
                        }
                    }
                }
                Ok("No suggestion".to_string())
            }
            Err(e) => {
                eprintln!("JSON parse error: {}", e);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid JSON response",
                ))
            }
        }
    }

    fn chat(&self, prompt: &str) -> Result<String, Error> {
        let json_string = format!(
            r#"{{
                "model": "{}",
                "max_tokens": {},
                "messages": [
                    {{ "role": "system", "content": "{}" }},
                    {{ "role": "user", "content": "{}" }}
                ]
            }}"#,
            self.model, self.chat_max_tokens, self.system_prompt, prompt
        );

        let chat_response = self.send_request(&self.chat_url, &json_string)?;

        match chat_response.parse::<JsonValue>() {
            Ok(parsed) => {
                if let JsonValue::Object(obj) = parsed {
                    if let Some(JsonValue::Array(choices)) = obj.get("choices") {
                        if let Some(JsonValue::Object(choice)) = choices.first() {
                            if let Some(JsonValue::Object(message)) = choice.get("message") {
                                if let Some(JsonValue::String(content)) = message.get("content") {
                                    return Ok(content.clone());
                                }
                            }
                            return Ok(chat_response);
                        }
                    } else if let Some(JsonValue::String(message)) = obj.get("message") {
                        return Ok(message.clone());
                    }
                }
                Ok("No message".to_string())
            }
            Err(e) => {
                eprintln!("JSON parse error: {}", e);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid JSON response",
                ))
            }
        }
    }

    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String> {
        let mut stream = TcpStream::connect(self.host.clone())?;

        stream.set_read_timeout(Some(Duration::new(5, 0)))?;
        stream.set_write_timeout(Some(Duration::new(5, 0)))?;

        let request = format!(
            "POST {} HTTP/1.1\r\n\
            Host: localhost\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\r\n\
            {}",
            endpoint,
            request_body.len(),
            request_body
        );

        stream.write_all(request.as_bytes())?;

        let reader = io::BufReader::new(stream);
        let mut body_started = false;
        let mut json_body = String::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                body_started = true;
                continue;
            }
            if body_started {
                json_body.push_str(&line);
            }
        }

        Ok(json_body)
    }
}
