use lazy_static::lazy_static;
use regex_lite::{Captures, Regex};

use crate::{Environment, parse};

// 定义支持两种变量格式的正则表达式
lazy_static! {
    static ref VAR_REGEX: Regex = Regex::new(r"\$\{([^{}]+)\}|\$([\w.-]+)").unwrap();
}

pub fn render_template(template: &str, env: &mut Environment) -> String {
    VAR_REGEX
        .replace_all(template, |caps: &Captures<'_>| {
            // 优先处理带大括号的变量,解析并执行
            if let Some(name) = caps.get(1) {
                return match parse(name.as_str()) {
                    Ok(expr) => match expr.eval_in_pipe(env) {
                        Ok(r) => r.to_string(),
                        Err(e) => {
                            eprintln!(
                                "template `{}` execute failed:\n{}",
                                name.as_str(),
                                e.to_string()
                            );
                            "".to_string()
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "template `{}` render failed:\n{}",
                            name.as_str(),
                            e.to_string()
                        );
                        "".to_string()
                    }
                };
            }

            // 如果没有带大括号的变量，则处理不带大括号的变量
            if let Some(name) = caps.get(2) {
                // dbg!(&name);
                return env
                    .get(name.as_str())
                    .map(|v| v.to_string())
                    .unwrap_or("".to_string());
            }

            // 默认返回空字符串
            "".to_string()
        })
        .into_owned()
}
