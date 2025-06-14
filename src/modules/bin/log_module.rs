use super::Int;
use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
use lazy_static::lazy_static;
use std::sync::RwLock;

// 日志级别常量
const NONE: Int = 0;
const ERROR: Int = 1;
const WARN: Int = 2;
const INFO: Int = 3;
const DEBUG: Int = 4;
const TRACE: Int = 5;

lazy_static! {
    static ref LOG_LEVEL: RwLock<Int> = RwLock::new(INFO); // 默认INFO级别
}

// 检查日志级别是否启用
fn is_log_level_enabled(level: Int) -> bool {
    *LOG_LEVEL.read().unwrap() >= level
}

pub fn get() -> Expression {
    (hash_map! {
        String::from("level") => Expression::from(hash_map! {
            String::from("none") => Expression::Integer(NONE),
            String::from("trace") => Expression::Integer(TRACE),
            String::from("debug") => Expression::Integer(DEBUG),
            String::from("info") => Expression::Integer(INFO),
            String::from("warn") => Expression::Integer(WARN),
            String::from("error") => Expression::Integer(ERROR),
        }),

        // 日志级别控制
               String::from("set_level") => Expression::builtin("set_level", set_log_level, "set the log level", "<level>"),
               String::from("get_level") => Expression::builtin("get_level", get_log_level, "get the current log level", ""),
               String::from("disable") => Expression::builtin("disable", disable_logging, "disable all logging output", ""),
               String::from("enabled") => Expression::builtin("enabled", is_level_enabled, "check if a log level is enabled", "<level>"),

               // 日志记录函数
               String::from("info") => Expression::builtin("info", log_info, "log info message", "<message>"),
               String::from("warn") => Expression::builtin("warn", log_warn, "log warning message", "<message>"),
               String::from("debug") => Expression::builtin("debug", log_debug, "log debug message", "<message>"),
               String::from("error") => Expression::builtin("error", log_error, "log error message", "<message>"),
               String::from("trace") => Expression::builtin("trace", log_trace, "log trace message", "<message>"),

               // 原始输出
               String::from("echo") => Expression::builtin("echo", log_echo, "print message without formatting", "<message>"),
    }).into()
}

// 日志级别管理函数
fn set_log_level(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("set_level", args, 1)?;

    if let Expression::Integer(level) = args[0].eval(env)? {
        *LOG_LEVEL.write().unwrap() = level;
        Ok(Expression::None)
    } else {
        Err(LmError::TypeError {
            expected: "integer".into(),
            sym: args[0].type_name(),
            found: args[0].to_string(),
        })
    }
}

fn get_log_level(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    Ok(Expression::Integer(*LOG_LEVEL.read().unwrap()))
}

fn disable_logging(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    *LOG_LEVEL.write().unwrap() = NONE;
    Ok(Expression::None)
}

fn is_level_enabled(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("enabled", args, 1)?;

    if let Expression::Integer(level) = args[0].eval(env)? {
        Ok(Expression::Boolean(is_log_level_enabled(level)))
    } else {
        Err(LmError::TypeError {
            expected: "integer".into(),
            sym: args[0].type_name(),
            found: args[0].to_string(),
        })
    }
}

// 通用日志打印函数
fn log_message(
    level: Int,
    prefix: &str,
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    if !is_log_level_enabled(level) {
        return Ok(Expression::None);
    }

    let mut output = String::new();
    let mut first_arg = true;

    for arg in args {
        let value = arg.eval(env)?.to_string();

        if !first_arg {
            output.push(' ');
        }

        output.push_str(&value);
        first_arg = false;
    }

    // 处理多行输出
    for (i, line) in output.lines().enumerate() {
        if i == 0 {
            println!("{}{}", prefix, line);
        } else {
            println!("{}{}", " ".repeat(prefix.len()), line);
        }
    }

    // 处理没有换行符的结尾
    if !output.ends_with('\n') && !output.is_empty() {
        println!();
    }

    Ok(Expression::None)
}

// 各日志级别专用函数
fn log_info(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    log_message(INFO, "\x1b[92m[INFO] \x1b[m", args, env)
}

fn log_warn(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    log_message(WARN, "\x1b[93m[WARN] \x1b[m", args, env)
}

fn log_debug(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    log_message(DEBUG, "\x1b[94m[DEBUG]\x1b[m ", args, env)
}

fn log_error(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    log_message(ERROR, "\x1b[91m[ERROR]\x1b[m ", args, env)
}

fn log_trace(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    log_message(TRACE, "\x1b[95m[TRACE]\x1b[m ", args, env)
}

// 简单回显函数
fn log_echo(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let mut output = String::new();
    let mut first_arg = true;

    for arg in args {
        let value = arg.eval(env)?.to_string();

        if !first_arg {
            output.push(' ');
        }

        output.push_str(&value);
        first_arg = false;
    }

    println!("{}", output);
    Ok(Expression::None)
}
