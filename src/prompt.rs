use std::env;

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common_macros::hash_map;

use crate::{CFM_ENABLED, Environment, Expression, STRICT_ENABLED};

// 提示符状态缓存
#[derive(Clone)]
struct PromptCache {
    last_update: Instant,
    content: String,
    ttl: Duration,
}

#[derive(Clone)]
struct PromptEngine {
    mode: u8,
    custom_template: Option<String>,
    template_func: Option<Expression>,
    cache: Arc<Mutex<PromptCache>>,
}
pub trait PromptEngineCommon {
    fn get_prompt(&self) -> String;
    fn get_incomplete_prompt(&self) -> String;
}
// struct MyPrompt {}

// impl PromptEngineCommon for MyPrompt {
//     fn get_prompt(&self) -> String {
//         if let Ok(cwd) = env::current_dir() {
//             if let Some(cwd_str) = cwd.to_str() {
//                 return format!("\x1b[1;34m(lumesh)\x1b[0m{} \x1b[32m❯\x1b[0m ", cwd_str);
//             }
//         }
//         ">> ".into()
//     }
//     fn get_incomplete_prompt(&self) -> String {
//         "... ".into()
//     }
// }
impl PromptEngineCommon for PromptEngine {
    // 核心提示符生成方法
    fn get_prompt(&self) -> String {
        // dbg!("getting prompt");
        // 1. 检查缓存有效性
        if let Ok(cache) = self.cache.lock() {
            if cache.last_update.elapsed() < cache.ttl {
                return cache.content.clone();
            }
        }

        // 2. 生成新提示符
        let prompt = match self.mode {
            1 => {
                if let Some(func) = &self.template_func {
                    self.render_from_func(func)
                } else if let Some(template) = &self.custom_template {
                    self.render_template(template)
                } else {
                    self.default_prompt()
                }
            }
            2 => self
                .get_starship_prompt()
                .unwrap_or_else(|| self.default_prompt()),
            _ => self.default_prompt(),
        };
        // // dbg!("rendering prompt");
        // let prompt = if let Some(func) = &self.template_func {
        //     self.render_from_func(func)
        // } else if let Some(template) = &self.custom_template {
        //     // dbg!("rendering template");
        //     self.render_template(template)
        // } else if self.starship_enabled {
        //     self.get_starship_prompt()
        //         .unwrap_or_else(|| "> ".to_string())
        // } else {
        //     self.default_prompt()
        // };

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
    fn render_from_func(&self, func: &Expression) -> String {
        // dbg!(&func.type_name());
        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                let cfm = CFM_ENABLED.with_borrow(|cfm| cfm == &true);
                let strict = STRICT_ENABLED.with_borrow(|s| s == &true);
                let ctx = Expression::from(hash_map! {
                    String::from("cfm") => Expression::from(cfm),
                    String::from("strict") => Expression::from(strict),
                });
                let r = func
                    .apply(vec![Expression::String(cwd_str.to_string()), ctx])
                    .eval(&mut Environment::new());
                return match r {
                    Ok(s) => s.to_string(),
                    _ => self.default_prompt(),
                };
            }
        }
        self.default_prompt()
    }
    fn render_template(&self, template: &str) -> String {
        // 实现简单的占位符替换
        let mut result = template
            .replace(
                "$CFM_TAG",
                if CFM_ENABLED.with_borrow(|cfm| cfm == &true) {
                    "CFM"
                } else {
                    ""
                },
            )
            .replace(
                "$STRICT_TAG",
                if STRICT_ENABLED.with_borrow(|cfm| cfm == &true) {
                    "S"
                } else {
                    ""
                },
            );

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
        if let Ok(cwd) = env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                return format!("\x1b[1;34m(lumesh)\x1b[0m{cwd_str} \x1b[32m❯\x1b[0m ");
            }
        }
        ">> ".into()
    }
}

fn get_short_path(path: &Path) -> String {
    // 将路径的组件收集到一个向量中
    let components = path.components().collect::<Vec<_>>();
    // dbg!(&components, components.len());

    // #[cfg(windows)]
    // let is_home = false;

    // #[cfg(unix)]
    let is_home = dirs::home_dir().is_some_and(|home_dir| path.starts_with(home_dir));
    // #[cfg(unix)]
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

    let sep = if cfg!(windows) { "\\" } else { "/" };

    let first_two: Vec<String> = match is_home {
        true => vec![
            "~".to_owned() + sep,
            components
                .get(3)
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
    format!("{}...{}{}", first_two.join(""), sep, last_two.join(sep))
}

pub fn get_prompt_engine(
    settings: Option<Expression>,
    template: Option<Expression>,
) -> Box<dyn PromptEngineCommon> {
    let (mode, ttl) = match settings {
        Some(Expression::Map(sets)) => {
            let ttl = sets
                .get("TTL_SECS")
                .map(|t| match t {
                    Expression::Integer(ttl) => *ttl as u64,
                    _ => 2,
                })
                .unwrap_or(2);
            let mode = sets
                .get("MODE")
                .map(|s| match s {
                    Expression::Integer(m) => *m as u8,
                    _ => 0,
                })
                .unwrap_or(0);
            (mode, ttl)
        }

        _ => (0, 2),
    };

    Box::new(PromptEngine {
        mode,
        template_func: template.clone().and_then(|f| match f {
            Expression::Lambda(..) => Some(f),
            Expression::Function(..) => Some(f),
            _ => None,
        }),
        custom_template: template.and_then(|t| match t {
            Expression::String(p) => Some(p),
            _ => None,
        }),
        cache: Arc::new(Mutex::new(PromptCache {
            content: "> ".to_string(),
            ttl: Duration::from_secs(ttl),
            last_update: Instant::now()
                .checked_sub(Duration::from_secs(ttl))
                .unwrap(),
        })),
    })
}
