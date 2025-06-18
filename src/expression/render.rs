use lazy_static::lazy_static;
use regex_lite::{Captures, Regex};

use crate::Environment;

// 定义支持两种变量格式的正则表达式
lazy_static! {
    static ref VAR_REGEX: Regex = Regex::new(r"\$\{([\w.-]+)\}|\$([\w.-]+)").unwrap();
}

pub fn render_template(template: &str, env: &mut Environment) -> String {
    VAR_REGEX
        .replace_all(template, |caps: &Captures<'_>| {
            // dbg!(&caps);
            // 优先处理带大括号的变量
            if let Some(name) = caps.get(1) {
                // dbg!(&name);
                return env
                    .get(name.as_str())
                    .map(|v| v.to_string())
                    .unwrap_or("".to_string());
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
