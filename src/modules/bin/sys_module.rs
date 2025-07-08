use std::{fs::OpenOptions, io::Write, rc::Rc};

use crate::{Environment, Expression, LmError, RuntimeError};
use common_macros::hash_map;

pub fn get() -> Expression {
    Expression::from(hash_map! {
        String::from("env") => Expression::builtin("env", env_builtin, "get root environment as a map", ""),
        String::from("set") => Expression::builtin("set", set_builtin, "define a variable in root environment", "<var> <val>"),
        String::from("unset") => Expression::builtin("unset", unset_builtin, "undefine a variable in root environment", "<var>"),

        String::from("vars") => Expression::builtin("vars", vars_builtin, "get defined variables in current enviroment", ""),
        String::from("has") => Expression::builtin("has", has_builtin, "check if a variable is defined in current environment", "<var>"),
        String::from("defined") => Expression::builtin("defined", defined_builtin, "check if a variable is defined in current environment tree", "<var>"),

        String::from("quote") => Expression::builtin("quote", quote_builtin, "quote an expression", "<expr>"),
        String::from("ecodes_rt") => Expression::builtin("ecodes_rt", err_codes_runtime, "display runtime error codes", ""),
        String::from("ecodes_lm") => Expression::builtin("ecodes_lm", err_codes_lmerror, "display Lmerror codes", ""),
        String::from("error") => Expression::builtin("error", err, "return a runtime error", "<msg>"),
        String::from("print_tty") => Expression::builtin("print_tty", print_tty, "print control sequence to tty", "<arg>"),
        String::from("discard") => Expression::builtin("discard", discard, "send data to /dev/null", "<arg>"),

        String::from("info") => Expression::builtin("info", info, "get os info", "<arg>"),
    })
}
fn info(_args: &Vec<Expression>, _env: &mut Environment) -> Result<Expression, crate::LmError> {
    let info = os_info::get();
    Ok(Expression::String(info.to_string()))
}
fn print_tty(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("print_tty", args, 1)?;

    // 判断操作系统
    let tty_path = if cfg!(windows) {
        "CON" // Windows控制台
    } else {
        "/dev/tty" // Unix
    };

    let mut tty = OpenOptions::new().write(true).open(tty_path)?;
    let v = super::get_string_arg(args[0].eval(env)?)?;
    tty.write_all(v.as_bytes())?;

    Ok(Expression::None)
}

fn discard(_args: &Vec<Expression>, _env: &mut Environment) -> Result<Expression, crate::LmError> {
    // 不用打开任何设备，只是丢弃参数
    Ok(Expression::None)
}

fn quote_builtin(
    args: &Vec<Expression>,
    _env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("quote", args, 1)?;
    Ok(Expression::Quote(Rc::new(args[0].clone())))
}

fn env_builtin(
    _args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_root().clone()))
}

fn vars_builtin(_: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_bindings_map()))
}

pub fn set_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("set", args, 2)?;
    let name = args[0].to_string();
    let expr = args[1].eval(env)?;
    env.define_in_root(&name, expr);
    Ok(Expression::None)
}

pub fn unset_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("unset", args, 1)?;
    let name = args[0].to_string();
    env.undefine_in_root(&name);
    Ok(Expression::None)
}

fn has_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("has", args, 1)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.has(&name)))
}
fn defined_builtin(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("defined", args, 1)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.is_defined(&name)))
}

fn err_codes_runtime(
    _args: &Vec<Expression>,
    _env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    Ok(RuntimeError::codes())
}
fn err_codes_lmerror(
    _args: &Vec<Expression>,
    _env: &mut Environment,
) -> Result<Expression, crate::LmError> {
    Ok(LmError::codes())
}

fn err(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    super::check_exact_args_len("sys.error", args, 1)?;
    let msg = args[0].eval(env)?;
    Err(LmError::CustomError(msg.to_string()))
}
