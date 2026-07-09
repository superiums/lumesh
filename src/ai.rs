use crate::Expression;
use rand::distr::SampleString;
use std::io::{self, Error};
// use std::time::Duration;
#[cfg(feature = "ai-https")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai-https")]
use ureq::Agent;

// ── 请求体 structs ────────────────────────────────────────────────────────────

#[cfg(feature = "ai-https")]
#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[cfg(feature = "ai-https")]
#[derive(Serialize, Clone)]
struct ExtraBody {
    enable_thinking: bool,
}

#[cfg(feature = "ai-https")]
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
#[cfg(feature = "ai-https")]
#[derive(Deserialize, Debug)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[cfg(feature = "ai-https")]
#[derive(Deserialize, Debug)]
struct ChatChoice {
    message: Option<ChatChoiceMessage>,
}

#[cfg(feature = "ai-https")]
#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Option<Vec<ChatChoice>>,
    message: Option<String>,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait AIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error>;
    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error>;
}

// ── Client struct ─────────────────────────────────────────────────────────────

pub struct MockAIClient {
    #[cfg(feature = "ai-https")]
    agent: Agent,
    host: String,
    chat_url: String,
    complete_max_tokens: u8,
    chat_max_tokens: u8,
    model: String,
    hint_prompt: String,
    chat_prompt: String,
    syntax: String,
    session_id: String,
    api_key: Option<String>,
}

impl MockAIClient {
    pub fn new(host: String, chat_url: String) -> Self {
        let session_id = rand::distr::Alphabetic.sample_string(&mut rand::rng(), 16);

        #[cfg(feature = "ai-https")]
        let agent = Agent::config_builder()
            // .timeout_global(Some(Duration::from_secs(15)))
            .max_idle_connections(0)
            .build()
            .new_agent();

        Self {
            host,
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
            api_key: None,

            #[cfg(feature = "ai-https")]
            agent,
        }
    }

    // 私有方法，不暴露在 trait 中
    #[cfg(feature = "ai-https")]
    fn send_request(&self, body: &ChatRequest) -> io::Result<ChatResponse> {
        let url = format!("{}{}", self.host, self.chat_url);

        let mut req = self.agent.post(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", &format!("Bearer {}", key));
        }

        let mut res = req.send_json(body).map_err(|e| {
            // dbg!(&e);
            io::Error::new(io::ErrorKind::Other, e.to_string())
        })?;
        res.body_mut().read_json::<ChatResponse>().map_err(|e| {
            // dbg!(&e);
            io::Error::new(io::ErrorKind::Other, e.to_string())
        })
    }
}

#[cfg(feature = "ai-https")]
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
        // dbg!(&response);
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
        Ok("Unkown Response Format".to_string())
    }
}

// ── 初始化 ────────────────────────────────────────────────────────────────────
pub fn init_ai(ai_cfg: Expression) -> MockAIClient {
    let session_id = rand::distr::Alphabetic.sample_string(&mut rand::rng(), 16);

    #[cfg(feature = "ai-https")]
    let agent = Agent::config_builder()
        // .timeout_global(Some(Duration::from_secs(15)))
        .max_idle_connections(0)
        .build()
        .new_agent();

    match ai_cfg {
        Expression::Map(cfg_map) => MockAIClient {
            host: match cfg_map.as_ref().get("host") {
                Some(Expression::String(s)) | Some(Expression::Symbol(s)) => {
                    if s.starts_with("http") {
                        s.to_string()
                    } else {
                        format!("http://{}", s)
                    }
                }
                _ => "http://localhost:11434".into(),
            },
            chat_url: match cfg_map.as_ref().get("chat_url") {
                Some(h) => h.to_string(),
                _ => "/v1/chat/completions".into(),
            },
            complete_max_tokens: match cfg_map.as_ref().get("complete_max_tokens") {
                Some(Expression::Integer(n)) => *n as u8,
                _ => 10,
            },
            chat_max_tokens: match cfg_map.as_ref().get("chat_max_tokens") {
                Some(Expression::Integer(n)) => *n as u8,
                _ => 100,
            },
            model: match cfg_map.as_ref().get("model") {
                Some(h) => h.to_string(),
                _ => "".into(),
            },
            hint_prompt: match cfg_map.as_ref().get("hint_prompt") {
                Some(h) => h.to_string(),
                _ => String::from(
                    "Append to the partial lumesh command. Output ONLY the missing suffix.\
                    Do NOT repeat any part of the input.\
                    Do NOT use code fences, backticks, markdown, comments, or explanations.\
                    Output raw text only. If unsure, output nothing.",
                ),
            },
            chat_prompt: match cfg_map.as_ref().get("chat_prompt") {
                Some(h) => h.to_string(),
                _ => String::from(
                    "You are a Lumesh shell assistant.\
                    Generate one fully executable Lumesh command from the user input.\
                    Follow Lumesh syntax strictly. Use `#` for comments.\
                    Output ONLY the command text. No explanations, no markdown, no prose.",
                ),
            },
            syntax: match cfg_map.as_ref().get("syntax") {
                Some(h) => h.to_string(),
                _ => "".into(),
            },
            session_id,
            api_key: match cfg_map.as_ref().get("api_key") {
                Some(h) => Some(h.to_string()),
                _ => None,
            },
            #[cfg(feature = "ai-https")]
            agent,
        },
        _ => {
            eprintln!("invalid config: AI config should be a map.\nloading default.");
            MockAIClient::new(
                "http://localhost:11434".into(),
                "/v1/chat/completions".into(),
            )
        }
    }
}

// ── HTTP-only 版本的实现（TcpStream）─────────────────────────────────────────
#[cfg(not(feature = "ai-https"))]
impl MockAIClient {
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String> {
        use std::io::{self, BufRead, Write};
        use std::net::TcpStream;

        use std::time::Duration;

        let mut stream = TcpStream::connect(self.host.clone())?;

        stream.set_read_timeout(Some(Duration::new(5, 0)))?;
        stream.set_write_timeout(Some(Duration::new(5, 0)))?;
        let auth = self
            .api_key
            .as_ref()
            .map(|k| format!("Authorization: Bearer {}\r\n", k))
            .unwrap_or_default();
        let request = format!(
            "POST {} HTTP/1.1\r\n\
            Host: localhost\r\n\
            Content-Type: application/json\r\n\
            {}Content-Length: {}\r\n\
            Connection: close\r\n\r\n\
            {}",
            endpoint,
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
    // fn send_request_http(&self, endpoint: &str, body: &str) -> io::Result<String> {
    //     use std::io::{BufRead, Write};
    //     use std::net::TcpStream;
    //     // use std::time::Duration;
    //     use tinyjson::JsonValue;
    //     // 原来的 TcpStream 实现，加上 api_key header 支持
    //     let mut stream = TcpStream::connect(&self.host)?;
    //     // stream.set_read_timeout(Some(Duration::new(10, 0)))?;
    //     let hostname = self.host.split(':').next().unwrap_or(&self.host);
    //     let auth = self
    //         .api_key
    //         .as_ref()
    //         .map(|k| format!("Authorization: Bearer {}\r\n", k))
    //         .unwrap_or_default();
    //     let request = format!(
    //         "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
    //         endpoint,
    //         hostname,
    //         auth,
    //         body.len(),
    //         body
    //     );
    //     stream.write_all(request.as_bytes())?;
    //     let reader = io::BufReader::new(stream);
    //     let mut started = false;
    //     let mut json_body = String::new();
    //     for line in reader.lines() {
    //         let line = line?;
    //         if line.is_empty() {
    //             started = true;
    //             continue;
    //         }
    //         if started {
    //             json_body.push_str(&line);
    //         }
    //     }
    //     Ok(json_body)
    // }
}

#[cfg(not(feature = "ai-https"))]
impl AIClient for MockAIClient {
    fn complete(&self, prompt: &str) -> Result<String, Error> {
        self.chat(true, prompt)
    }
    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, Error> {
        use tinyjson::JsonValue;

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
}
