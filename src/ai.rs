use json::parse;
// use lazy_static::lazy_static;
use std::io::Error;
use std::io::{self, BufRead, Write};
use std::net::TcpStream;
use std::time::Duration;
// lazy_static! {
//     // 双重锁设计：外层Mutex防止多线程竞争初始化，内层HashSet只读
//    pub static ref AI_CLIENT: Box<dyn AIClient + Sync + Send> = Box::new(MockAIClient);
// }

pub fn init_ai() -> MockAIClient {
    // 调用代码补全模式
    // let ai =
    MockAIClient::new(
        "localhost:11000".into(),
        "/completion".into(),
        "/v1/chat/completions".into(),
    )
    // let completion_response = ai.complete("fn sum(")?;
    // //
    // println!("Completion Response:\n{}", completion_response);

    // // 调用对话模式
    // let chat_response = ai.chat("hi")?;
    // println!("Chat Response:\n{}", chat_response);

    // Ok(())
}

// AI client trait for abstraction
pub trait AIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error>;
    fn chat(&self, prompt: &str) -> Result<String, Error>;
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String>;
}

// Mock implementation (replace with actual ollama/llama.cpp integration)
pub struct MockAIClient {
    host: String,
    complete_url: String,
    chat_url: String,
    complete_max_tokens: u8,
    chat_max_tokens: u8,
    model: String,
    system_content: String,
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
            system_content: "you're a shell helper".into(),
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

        match parse(&completion_response) {
            Ok(parsed) => {
                // Different APIs might return the content in different ways
                if parsed["content"].is_string() {
                    Ok(parsed["content"].to_string())
                } else if parsed["choices"].is_array() {
                    if let Some(choice) = parsed["choices"].members().last() {
                        Ok(choice["text"].to_string())
                    } else {
                        Ok("No suggestion".to_string())
                    }
                } else {
                    Ok("No suggestion".to_string())
                }
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
            self.model, self.chat_max_tokens, self.system_content, prompt
        );

        let chat_response = self.send_request(&self.chat_url, &json_string)?;

        match parse(&chat_response) {
            Ok(parsed) => {
                if parsed["choices"].is_array() {
                    if let Some(choice) = parsed["choices"].members().last() {
                        if choice["message"]["content"].is_string() {
                            Ok(choice["message"]["content"].to_string())
                        } else {
                            Ok(choice.to_string())
                        }
                    } else {
                        Ok("No message".to_string())
                    }
                } else if parsed["message"].is_string() {
                    Ok(parsed["message"].to_string())
                } else {
                    Ok("No message".to_string())
                }
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
        // let mut response = String::new();

        // Read until we find the empty line that separates headers from body
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
