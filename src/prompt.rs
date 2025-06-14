use std::env;

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::Expression;

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
                ttl: cache.ttl,
            };
        }

        prompt
    }
    fn get_incomplete_prompt(&self) -> String {
        "... ".into()
    }
}
impl PromptEngine {
    // pub fn new() -> Self {
    //     let starship_enabled = env::var("STARSHIP_SHELL")
    //         .map(|s| !s.is_empty())
    //         .unwrap_or(false);

    //     Self {
    //         starship_enabled,
    //         custom_template: None,
    //         cache: Arc::new(Mutex::new(PromptCache {
    //             last_update: Instant::now(),
    //             content: "> ".to_string(),
    //             ttl: Duration::from_secs(2),
    //         })),
    //     }
    // }

    // 设置自定义模板 (支持 {cwd}, {git} 等占位符)
    // pub fn set_template(&mut self, template: String) {
    //     self.custom_template = Some(template);
    // }

    fn render_template(&self, template: &str) -> String {
        // 实现简单的占位符替换
        let mut result = template.to_string();

        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                result = if result.contains("$CWD_SHORT") {
                    result.replace("$CWD_SHORT", &get_short_path(cwd.as_path()))
                } else {
                    #[cfg(unix)]
                    if cwd_str.starts_with("/home/") {
                        if let Some(home_dir) = dirs::home_dir() {
                            let cwd_new_str = cwd_str
                                .to_owned()
                                .replace(home_dir.to_string_lossy().as_ref(), "~");
                            return result.replace("$CWD", &cwd_new_str);
                        }
                    }
                    result.replace("$CWD", cwd_str)
                };
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

fn get_short_path(path: &Path) -> String {
    // 将路径的组件收集到一个向量中
    let components = path.components().collect::<Vec<_>>();
    // dbg!(&components, components.len());

    #[cfg(windows)]
    let is_home = false;

    #[cfg(unix)]
    let is_home = dirs::home_dir().is_some_and(|home_dir| path.starts_with(home_dir));
    #[cfg(unix)]
    if is_home {
        if let Some(home_dir) = dirs::home_dir() {
            if components.len() < 6 {
                return path
                    .to_string_lossy()
                    .to_string()
                    .replace(home_dir.to_str().unwrap(), "~");
            }
        }
    }

    // 检查路径组件数量

    if !is_home && components.len() < 5 {
        return path.to_string_lossy().to_string();
    }

    let first_two: Vec<String> = match is_home {
        true => vec![
            "~/".to_string(),
            components
                .get(3)
                .take()
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .to_string(),
        ],
        false => components
            .iter()
            .take(2)
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect(),
    };
    let last_two: Vec<String> = components
        .iter()
        .rev()
        .take(2)
        .rev()
        .map(|comp| comp.as_os_str().to_string_lossy().to_string())
        .collect();

    // 生成短路径格式
    format!("{}.../{}", first_two.join(""), last_two.join("/"))
}

pub fn get_prompt_engine(
    modes: Option<Expression>,
    template: Option<Expression>,
) -> Box<dyn PromptEngineCommon> {
    match modes {
        Some(setting) => {
            match setting {
                Expression::Map(sets) => Box::new(PromptEngine {
                    starship_enabled: sets.get("STARSHIP_ENABLED").is_some_and(|s| s.is_truthy()),
                    custom_template: template.map(|t| t.to_string()),
                    cache: Arc::new(Mutex::new(PromptCache {
                        last_update: Instant::now()
                            .checked_sub(Duration::from_secs(360))
                            .unwrap(),
                        content: "> ".to_string(),
                        ttl: Duration::from_secs(
                            sets.get("TTL_SECS")
                                .and_then(|t| match t {
                                    Expression::Integer(ttl) => Some(*ttl as u64),
                                    _ => Some(2),
                                })
                                .unwrap_or(2),
                        ),
                    })),
                }),

                _ => {
                    eprintln!("LUME_PROMPT_MODE must be a map");
                    Box::new(MyPrompt {})
                }
            }

            // Initialize it with a custom template
            // prompt_engine.set_template("{cwd}|> ".to_string());
            // Box::new(prompt_engine)
        }
        _ if template.is_some() => Box::new(PromptEngine {
            starship_enabled: false,
            custom_template: template.map(|t| t.to_string()),
            cache: Arc::new(Mutex::new(PromptCache {
                last_update: Instant::now()
                    .checked_sub(Duration::from_secs(360))
                    .unwrap(),
                content: "> ".to_string(),
                ttl: Duration::from_secs(2),
            })),
        }),
        _ => Box::new(MyPrompt {}),
    }
}
