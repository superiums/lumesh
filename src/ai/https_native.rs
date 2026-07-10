// 编译条件：feature = "ai-tls" 且未启用 "ai-https"
// 依赖：native-tls（系统 TLS）+ tinyjson（已有依赖，无需新增）

use native_tls::TlsConnector;
use std::io::{self, BufRead, Error, Write};
use std::net::TcpStream;
use std::time::Duration;
use tinyjson::JsonValue;

use super::{AIClient, MockAIClient};

// ── URL 解析 ──────────────────────────────────────────────────────────────────

/// 从 "http(s)://host[:port]" 解析出 (is_https, hostname, port)
fn parse_host(url: &str) -> (bool, String, u16) {
    let (is_https, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        // 无 scheme，视为 HTTP
        (false, url)
    };
    // 去掉路径部分（如果有）
    let host_port = rest.split('/').next().unwrap_or(rest);
    if let Some((h, p)) = host_port.split_once(':') {
        let port = p.parse().unwrap_or(if is_https { 443 } else { 80 });
        (is_https, h.to_string(), port)
    } else {
        (
            is_https,
            host_port.to_string(),
            if is_https { 443 } else { 80 },
        )
    }
}

// ── send_request（TcpStream + 可选 TLS）──────────────────────────────────────

impl MockAIClient {
    fn send_request(&self, endpoint: &str, request_body: &str) -> io::Result<String> {
        let (is_https, host, port) = parse_host(&self.host);
        let addr = format!("{}:{}", host, port);

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
            host,
            auth,
            request_body.len(),
            request_body
        );

        let tcp = TcpStream::connect(&addr)?;
        tcp.set_read_timeout(Some(Duration::new(15, 0)))?;
        tcp.set_write_timeout(Some(Duration::new(5, 0)))?;

        let mut body_started = false;
        let mut json_body = String::new();

        if is_https {
            let connector = TlsConnector::new()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            let mut tls = connector
                .connect(&host, tcp)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            tls.write_all(request.as_bytes())?;
            let reader = io::BufReader::new(tls);
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
        } else {
            // 降级为普通 HTTP（host 以 http:// 开头时）
            let mut stream = tcp;
            stream.write_all(request.as_bytes())?;
            let reader = io::BufReader::new(stream);
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
        }

        Ok(json_body)
    }
}

// ── AIClient impl（JSON 构建与解析复用 tinyjson）─────────────────────────────

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
