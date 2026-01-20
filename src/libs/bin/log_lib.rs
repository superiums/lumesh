use crate::{Environment, Expression};

use crate::libs::BuiltinInfo;
use crate::libs::helper::check_exact_args_len;
use crate::libs::lazy_module::LazyModule;
use crate::{Int, RuntimeError, RuntimeErrorKind, reg_info, reg_lazy};

use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 日志级别控制
        set_level , get_level , disable , enabled ,
        // 日志记录函数
        info , warn , debug , error , trace ,
        // 原始输出
        echo ,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // level =>

        // 日志级别控制
        set_level => "set the log level", "<level>"
        get_level => "get the current log level", ""
        disable => "disable all logging output", ""
        enabled => "check if a log level is enabled", "<level>"

        // 日志记录函数
        info => "log info message", "<message>"
        warn => "log warning message", "<message>"
        debug => "log debug message", "<message>"
        error => "log error message", "<message>"
        trace => "log trace message", "<message>"

        // 原始输出
        echo => "print message without formatting", "<message>"
    })
}

// 日志级别常量
const NONE: Int = 0;
const ERROR: Int = 1;
const WARN: Int = 2;
const INFO: Int = 3;
const DEBUG: Int = 4;
const TRACE: Int = 5;

static LOG_LEVEL: LazyLock<RwLock<Int>> = LazyLock::new(|| RwLock::new(INFO));

// Helper Functions
// 检查日志级别是否启用
fn is_log_level_enabled(level: Int) -> bool {
    *LOG_LEVEL.read().unwrap() >= level
}
// 日志级别管理函数
fn set_level(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_level", args, 1, ctx)?;

    if let Expression::Integer(level) = args[0].eval(env)? {
        *LOG_LEVEL.write().unwrap() = level;
        Ok(Expression::None)
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer".to_string(),
                sym: args[0].type_name(),
                found: args[0].to_string(),
            },
            ctx.clone(),
            0,
        ))
    }
}

fn get_level(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::Integer(*LOG_LEVEL.read().unwrap()))
}

fn disable(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    *LOG_LEVEL.write().unwrap() = NONE;
    Ok(Expression::None)
}

fn enabled(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("enabled", args, 1, ctx)?;

    if let Expression::Integer(level) = args[0].eval(env)? {
        Ok(Expression::Boolean(is_log_level_enabled(level)))
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer".to_string(),
                sym: args[0].type_name(),
                found: args[0].to_string(),
            },
            ctx.clone(),
            0,
        ))
    }
}
// 通用日志打印函数
fn log_message(
    level: Int,
    prefix: &str,
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
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
            println!("{prefix}{line}");
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
fn info(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    log_message(INFO, "\x1b[92m[INFO] \x1b[m", args, env, ctx)
}

fn warn(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    log_message(WARN, "\x1b[93m[WARN] \x1b[m", args, env, ctx)
}

fn debug(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    log_message(DEBUG, "\x1b[94m[DEBUG]\x1b[m ", args, env, ctx)
}

fn error(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    log_message(ERROR, "\x1b[91m[ERROR]\x1b[m ", args, env, ctx)
}

fn trace(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    log_message(TRACE, "\x1b[95m[TRACE]\x1b[m ", args, env, ctx)
}
// 简单回显函数
fn echo(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
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

    println!("{output}");
    Ok(Expression::None)
}
