use std::env;

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// 提示符状态缓存
#[derive(Clone)]
struct PromptCache {
    last_update: Instant,
    content: String,
    ttl: Duration,
}

#[derive(Clone)]
struct PromptEngine {
    starship_enabled: bool,
    custom_template: Option<String>,
    cache: Arc<Mutex<PromptCache>>,
}
pub trait PromptEngineCommon {
    fn get_prompt(&self) -> String;
    fn get_incomplete_prompt(&self) -> String;
}
struct MyPrompt {}

impl PromptEngineCommon for MyPrompt {
    fn get_prompt(&self) -> String {
        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                return format!("\x1b[1;34m(lumesh)\x1b[0m{} \x1b[32m❯\x1b[0m ", cwd_str);
            }
        }
        ">> ".into()
    }
    fn get_incomplete_prompt(&self) -> String {
        "... ".into()
    }
}
impl PromptEngineCommon for PromptEngine {
    // 核心提示符生成方法
    fn get_prompt(&self) -> String {
        // 1. 检查缓存有效性
        if let Ok(cache) = self.cache.lock() {
            if cache.last_update.elapsed() < cache.ttl {
                return cache.content.clone();
            }
        }

        // 2. 生成新提示符
        let prompt = if let Some(template) = &self.custom_template {
            self.render_template(template)
        } else if self.starship_enabled {
            self.get_starship_prompt()
                .unwrap_or_else(|| "> ".to_string())
        } else {
            self.default_prompt()
        };

        // 3. 更新缓存
        if let Ok(mut cache) = self.cache.lock() {
            *cache = PromptCache {
                last_update: Instant::now(),
                content: prompt.clone(),
                ttl: Duration::from_secs(2),
            };
        }

        prompt
    }
    fn get_incomplete_prompt(&self) -> String {
        "... ".into()
    }
}
impl PromptEngine {
    pub fn new() -> Self {
        let starship_enabled = env::var("STARSHIP_SHELL")
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        Self {
            starship_enabled,
            custom_template: None,
            cache: Arc::new(Mutex::new(PromptCache {
                last_update: Instant::now(),
                content: "> ".to_string(),
                ttl: Duration::from_secs(2),
            })),
        }
    }

    // 设置自定义模板 (支持 {cwd}, {git} 等占位符)
    pub fn set_template(&mut self, template: String) {
        self.custom_template = Some(template);
    }

    fn render_template(&self, template: &str) -> String {
        // 实现简单的占位符替换
        let mut result = template.to_string();

        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                result = result.replace("{cwd}", cwd_str);
            }
        }

        // 可以扩展更多占位符...
        result
    }

    fn get_starship_prompt(&self) -> Option<String> {
        // 异步调用 starship prompt 子进程
        let output = Command::new("starship")
            .arg("prompt")
            .env_clear()
            .envs(env::vars().filter(|(k, _)| {
                // 只传递 starship 需要的环境变量
                k.starts_with("STARSHIP_") || k == "TERM" || k == "PWD" || k == "HOME"
            }))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        String::from_utf8(output.stdout).ok()
    }

    fn default_prompt(&self) -> String {
        // 简约但有用的默认提示符
        env::current_dir()
            .map(|p| {
                if let Some(s) = p.file_name().and_then(|s| s.to_str()) {
                    format!("{}> ", s)
                } else {
                    "> ".to_string()
                }
            })
            .unwrap_or("> ".to_string())
    }
}

pub fn get_prompt_engine() -> Box<dyn PromptEngineCommon> {
    let starship = env::var("STARSHIP_SHELL").unwrap_or("".into());
    //dbg!(&starship);

    if starship.is_empty() {
        Box::new(MyPrompt {})
    } else {
        let mut prompt_engine = PromptEngine::new();
        // Initialize it with a custom template
        prompt_engine.set_template("{cwd}|> ".to_string());
        Box::new(prompt_engine)
    }
}
