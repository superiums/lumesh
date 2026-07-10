use crate::Expression;
use rand::distr::SampleString;

#[cfg(not(any(feature = "ai-https", feature = "ai-tls")))]
mod http;
#[cfg(feature = "ai-https")]
mod https;
#[cfg(all(feature = "ai-tls", not(feature = "ai-https")))]
mod https_native;
// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait AIClient {
    fn complete(&self, prompt: &str) -> Result<String, std::io::Error>;
    fn chat(&self, is_completion: bool, prompt: &str) -> Result<String, std::io::Error>;
}

// ── Client struct ─────────────────────────────────────────────────────────────

pub struct MockAIClient {
    #[cfg(feature = "ai-https")]
    pub(crate) agent: ureq::Agent,
    pub(crate) host: String,
    pub(crate) chat_url: String,
    pub(crate) complete_max_tokens: u8,
    pub(crate) chat_max_tokens: u8,
    pub(crate) model: String,
    pub(crate) hint_prompt: String,
    pub(crate) chat_prompt: String,
    pub(crate) syntax: String,
    pub(crate) session_id: String,
    pub(crate) api_key: Option<String>,
}

impl MockAIClient {
    pub fn new(host: String, chat_url: String) -> Self {
        let session_id = rand::distr::Alphabetic.sample_string(&mut rand::rng(), 16);
        #[cfg(feature = "ai-https")]
        let agent = ureq::Agent::config_builder()
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
}

// ── 初始化 ────────────────────────────────────────────────────────────────────

pub fn init_ai(ai_cfg: Expression) -> MockAIClient {
    let session_id = rand::distr::Alphabetic.sample_string(&mut rand::rng(), 16);
    #[cfg(feature = "ai-https")]
    let agent = ureq::Agent::config_builder()
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
