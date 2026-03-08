use regex_lite::{Captures, Regex};

use crate::{Environment, Expression, RuntimeError, eval::State, parse};
use std::sync::LazyLock;

// 定义支持两种变量格式的正则表达式
static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^{}]+)\}|\$([\w.-]+)").unwrap());

pub fn render_template(
    template: &str,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let r = VAR_REGEX
        .replace_all(template, |caps: &Captures<'_>| {
            // 优先处理带大括号的变量,解析并执行
            if let Some(name) = caps.get(1) {
                if name
                    .as_str()
                    .chars()
                    .any(|c| c.is_ascii_punctuation() || c.is_whitespace())
                {
                    return match parse(name.as_str()) {
                        Ok(expr) => expr
                            .eval_with_assign(state, env)
                            .map_or(name.as_str().to_string(), |x| x.to_string()),
                        Err(e) => {
                            eprintln!("template `{}` render failed:\n{}", name.as_str(), e);
                            name.as_str().to_string()
                        }
                    };
                }
                return ctx
                    .handle_variable(name.as_str(), false, state, env, depth)
                    .map_or(name.as_str().to_string(), |x| x.to_string());
            }

            // 如果没有带大括号的变量，则处理不带大括号的变量
            if let Some(name) = caps.get(2) {
                return ctx
                    .handle_variable(name.as_str(), false, state, env, depth)
                    .map_or(format!("${}", name.as_str()), |x| x.to_string());
            }

            // 默认返回空字符串
            "".to_string()
        })
        .into_owned();
    Ok(Expression::String(r))
}
