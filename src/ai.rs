use crate::Expression;
use rand::distr::SampleString;
use std::io::Error;
use std::io::{self, BufRead, Write};
use std::net::TcpStream;
use std::time::Duration;
use tinyjson::JsonValue;

pub fn init_ai(ai_cfg: Expression) -> MockAIClient {
    let mut rng = rand::rng();
    let session_id = rand::distr::Alphabetic.sample_string(&mut rng, 16);

    match ai_cfg {
        Expression::Map(cfg_map) => MockAIClient {
            host: match cfg_map.as_ref().get("host") {
                Some(h) => h.to_string(),
                _ => "localhost:11434".into(),
            },
            // complete_url: match cfg_map.as_ref().get("complete_url") {
            //     Some(h) => h.to_string(),
            //     _ => "/completion".into(),
            // },
            chat_url: match cfg_map.as_ref().get("chat_url") {
                Some(h) => h.to_string(),
                _ => "/v1/chat/completions".into(),
            },
            complete_max_tokens: match cfg_map.as_ref().get("complete_max_tokens") {
                Some(Expression::Integer(c_token)) => *c_token as u8,
                _ => 10,
            },
            chat_max_tokens: match cfg_map.as_ref().get("chat_max_tokens") {
                Some(Expression::Integer(c_token)) => *c_token as u8,
                _ => 100,
            },
            model: match cfg_map.as_ref().get("model") {
                Some(h) => h.to_string(),
                _ => "".into(),
            },
            hint_prompt: match cfg_map.as_ref().get("system_prompt") {
                Some(h) => h.to_string(),
                _ => String::from(
                    "Append to the partial lumesh command. Output ONLY the missing suffix.\
                        Do NOT repeat any part of the input.\
                        Do NOT use code fences, backticks, markdown, comments, or explanations.\
                        Output raw text only. If unsure, output nothing.",
                ),
            },
            chat_prompt: match cfg_map.as_ref().get("system_prompt") {
                Some(h) => h.to_string(),
                _ => String::from(
                    "You are a Lumesh shell assistant.\
Generate one fully executable Lumesh command from the user input.\
Follow Lumesh syntax strictly. Use `#` for comments.\
Output ONLY the command text. No explanations, no markdown, no prose.\
Never use code fences, backticks, or triple quotes.",
                ),
            },
            syntax: match cfg_map.as_ref().get("syntax") {
                Some(h) => h.to_string(),
                _ => "".into(),
            },
            session_id,
        },
        _ => {
            eprintln!("invalid config:AI config should be a map.\nloading default.");
            MockAIClient::new("localhost:11434".into(), "/v1/chat/completions".into())
        }
    }
}

pub trait AIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error>;
    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error>;
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String>;
}

pub struct MockAIClient {
    host: String,
    // complete_url: String,
    chat_url: String,
    complete_max_tokens: u8,
    chat_max_tokens: u8,
    model: String,
    hint_prompt: String,
    chat_prompt: String,
    syntax: String,
    session_id: String,
}

impl MockAIClient {
    pub fn new(host: String, chat_url: String) -> Self {
        let mut rng = rand::rng();
        let session_id = rand::distr::Alphabetic.sample_string(&mut rng, 16);

        Self {
            host,
            // complete_url,
            chat_url,
            complete_max_tokens: 20,
            chat_max_tokens: 100,
            model: "".into(),
            hint_prompt: String::from(
                "Append to the partial lumesh command. Output ONLY the missing suffix.\
                    Do NOT repeat any part of the input.\
                    Do NOT use code fences, backticks, markdown, comments, or explanations.\
                    Output raw text only. If unsure, output nothing.",
            ),
            chat_prompt: String::from(
                "You are a Lumesh shell assistant. \
                     Given the user's partial or natural language input, output a single executable Lumesh command. \
                     Output ONLY the command, no explanation, no markdown, no code fences.",
            ),
            syntax: "".into(),
            session_id,
        }
    }
}

impl AIClient for MockAIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error> {
        let r = self.chat(true, prompt)?;
        dbg!(&r);
        Ok(r)
        //     let json_payload = JsonValue::Object(
        //         vec![
        //             ("model".to_string(), JsonValue::String(self.model.clone())),
        //             (
        //                 "max_tokens".to_string(),
        //                 JsonValue::Number(self.complete_max_tokens as f64),
        //             ),
        //             (
        //                 "system_prompt".to_string(),
        //                 JsonValue::String(self.system_prompt.clone()),
        //             ),
        //             ("prompt".to_string(), JsonValue::String(prompt.to_string())),
        //         ]
        //         .into_iter()
        //         .collect(),
        //     );

        //     let json_string = json_payload.stringify().unwrap();
        //     let completion_response = self.send_request(&self.complete_url, &json_string)?;
        //     // dbg!(&completion_response);

        //     match completion_response.parse::<JsonValue>() {
        //         Ok(parsed) => {
        //             if let JsonValue::Object(obj) = parsed {
        //                 if let Some(JsonValue::String(content)) = obj.get("content") {
        //                     // dbg!(&content);
        //                     return Ok(content.clone());
        //                 } else if let Some(JsonValue::Array(choices)) = obj.get("choices")
        //                     && let Some(JsonValue::Object(choice)) = choices.first()
        //                     && let Some(JsonValue::String(text)) = choice.get("text")
        //                 {
        //                     return Ok(text.clone());
        //                 }
        //             }
        //             Ok("No suggestion".to_string())
        //         }
        //         Err(e) => {
        //             eprintln!("JSON parse error: {e}");
        //             Err(io::Error::new(
        //                 io::ErrorKind::InvalidData,
        //                 "Invalid JSON response",
        //             ))
        //         }
        //     }
    }

    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error> {
        // let json_string = format!(
        //     r#"{{
        //         "model": "{}",
        //         "max_tokens": {},
        //         "messages": [
        //             {{ "role": "system", "content": "{}" }},
        //             {{ "role": "user", "content": "{}" }}
        //         ]
        //     }}"#,
        //     self.model, self.chat_max_tokens, self.system_prompt, prompt
        // );
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
        dbg!(&chat_response);
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
