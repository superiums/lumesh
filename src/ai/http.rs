// 注意：此文件只在 mod.rs 中以 #[cfg(not(feature = "ai-https"))] 引入

use std::io::{self, BufRead, Error, Write};
use std::net::TcpStream;
use std::time::Duration;
use tinyjson::JsonValue;

use super::{AIClient, MockAIClient};

impl MockAIClient {
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String> {
        // 从 "http://host:port" 中提取 "host:port"
        let host_port = self
            .host
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        let mut stream = TcpStream::connect(host_port)?;
        stream.set_read_timeout(Some(Duration::new(5, 0)))?;
        stream.set_write_timeout(Some(Duration::new(5, 0)))?;

        let hostname = host_port.split(':').next().unwrap_or(host_port);
        let auth = self
            .api_key
            .as_ref()
            .map(|k| format!("Authorization: Bearer {}\r\n", k))
            .unwrap_or_default();

        let request = format!(
            "POST {} HTTP/1.1\r\n\
            Host: {}\r\n\
            Content-Type: application/json\r\n\
            {}Content-Length: {}\r\n\
            Connection: close\r\n\r\n\
            {}",
            endpoint,
            hostname,
            auth,
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

impl AIClient for MockAIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error> {
        self.chat(true, prompt)
    }

    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error> {
        let max_token = if is_completion {
            self.complete_max_tokens as f64
        } else {
            self.chat_max_tokens as f64
        };
        let identify = if is_completion {
            &self.hint_prompt
        } else {
            &self.chat_prompt
        };
        let system_prompt = JsonValue::String(format!("{}\n{}", identify, self.syntax));

        let extra_body = JsonValue::Object(
            vec![("enable_thinking".to_string(), JsonValue::Boolean(false))]
                .into_iter()
                .collect(),
        );
        let json_payload = JsonValue::Object(
            vec![
                ("model".to_string(), JsonValue::String(self.model.clone())),
                (
                    "session_id".to_string(),
                    JsonValue::String(self.session_id.clone()),
                ),
                ("max_tokens".to_string(), JsonValue::Number(max_token)),
                (
                    "messages".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::Object(
                            vec![
                                ("role".to_string(), JsonValue::String("system".to_string())),
                                ("content".to_string(), system_prompt),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        JsonValue::Object(
                            vec![
                                ("role".to_string(), JsonValue::String("user".to_string())),
                                ("content".to_string(), JsonValue::String(prompt.to_string())),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ]),
                ),
                ("extra_body".to_string(), extra_body.clone()),
                ("chat_template_kwargs".to_string(), extra_body),
            ]
            .into_iter()
            .collect(),
        );

        let json_string = json_payload.stringify().unwrap();
        let chat_response = self.send_request(&self.chat_url, &json_string)?;

        match chat_response.parse::<JsonValue>() {
            Ok(parsed) => {
                if let JsonValue::Object(obj) = parsed {
                    if let Some(JsonValue::Array(choices)) = obj.get("choices") {
                        if let Some(JsonValue::Object(choice)) = choices.first() {
                            if let Some(JsonValue::Object(message)) = choice.get("message")
                                && let Some(JsonValue::String(content)) = message.get("content")
                            {
                                return Ok(content.clone());
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
                eprintln!("JSON parse error: {e}");
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid JSON response",
                ))
            }
        }
    }
}
