use std::{fs::OpenOptions, io::Write, rc::Rc};

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_exact_args_len, get_integer_arg, get_string_arg};
use crate::{
    Environment, Expression, Int, LmError, MAX_RUNTIME_RECURSION, MAX_SYNTAX_RECURSION,
    MAX_USEMODE_RECURSION, RuntimeError, set_cfm_enabled, set_print_direct,
};
use std::collections::BTreeMap;

use crate::libs::lazy_module::LazyModule;
use crate::{reg_info, reg_lazy};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        env, set, unset,
        vars, has, defined,
        quote, ecodes_rt, ecodes_lm,
        print_tty, discard,
        info,
        // throw,
        max_syntax,
        max_runtime,
        max_usemode,
        set_cfm,
        set_pdm,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        env => "get root environment as a map", ""
        set => "define a variable in root environment", "<var> <val>"
        unset => "undefine a variable in root environment", "<var>"

        vars => "get defined variables in current enviroment", ""
        has => "check if a variable is defined in current environment", "<var>"
        defined => "check if a variable is defined in current environment tree", "<var>"

        quote => "quote an expression", "<expr>"
        ecodes_rt => "display runtime error codes", ""
        ecodes_lm => "display Lmerror codes", ""
        // throw => "return a runtime error", "<msg>"
        print_tty => "print control sequence to tty", "<arg>"
        discard => "send data to /dev/null", "<arg>"

        info => "get os info", ""

        max_syntax => "get/set max syntax recursion","[int]"
        max_runtime=> "get/set max runtime recursion","[int]"
        max_usemode=> "get/set max use mode recursion","[int]"
        set_cfm=> "enable/disable CFM","<boolean>"
        set_pdm=> "enable/disable print direct mode","<boolean>"

    })
}

fn info(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let info = os_info::get();
    Ok(Expression::String(info.to_string()))
}
fn print_tty(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("print_tty", args, 1, ctx)?;

    // 判断操作系统
    let tty_path = if cfg!(windows) {
        "CON" // Windows控制台
    } else {
        "/dev/tty" // Unix
    };

    let mut tty = OpenOptions::new()
        .write(true)
        .open(tty_path)
        .map_err(|e| RuntimeError::from_io_error(e, "open tty".into(), Expression::None, 0))?;
    let v = get_string_arg(args[0].eval(env)?, ctx)?;
    tty.write_all(v.as_bytes())
        .map_err(|e| RuntimeError::from_io_error(e, "write tty".into(), Expression::None, 0))?;

    Ok(Expression::None)
}

fn discard(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // 不用打开任何设备，只是丢弃参数
    Ok(Expression::None)
}

fn quote(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("quote", args, 1, ctx)?;
    Ok(Expression::Quote(Rc::new(args[0].clone())))
}

fn env(
    _args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(env.get_root().clone()))
}

fn vars(
    _: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(env.get_bindings_map()))
}

pub fn set(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set", args, 2, ctx)?;
    let name = args[0].to_string();
    let expr = args[1].eval(env)?;
    env.define_in_root(&name, expr);
    Ok(Expression::None)
}

pub fn unset(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("unset", args, 1, ctx)?;
    let name = args[0].to_string();
    env.undefine_in_root(&name);
    Ok(Expression::None)
}

fn has(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("has", args, 1, ctx)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.has(&name)))
}
fn defined(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("defined", args, 1, ctx)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.is_defined(&name)))
}

fn ecodes_rt(
    _args: &[Expression],
    _env: &mut Environment,
    _: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(RuntimeError::codes())
}
fn ecodes_lm(
    _args: &[Expression],
    _env: &mut Environment,
    _: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(LmError::codes())
}

fn max_syntax(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_SYNTAX_RECURSION.with_borrow(|x| x.clone() as Int),
        ));
    }
    let i = get_integer_arg(args[0].eval(env)?, ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_SYNTAX_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn max_runtime(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_RUNTIME_RECURSION.with_borrow(|x| x.clone() as Int),
        ));
    }
    let i = get_integer_arg(args[0].eval(env)?, ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_RUNTIME_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn max_usemode(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_USEMODE_RECURSION.with_borrow(|x| x.clone() as Int),
        ));
    }
    let i = get_integer_arg(args[0].eval(env)?, ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_USEMODE_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn set_cfm(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_cfm", args, 1, ctx)?;
    let b = args[0].eval(env)?.is_truthy();
    set_cfm_enabled(b);
    Ok(Expression::None)
}
fn set_pdm(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_print_direct", args, 1, ctx)?;
    let b = args[0].eval(env)?.is_truthy();
    set_print_direct(b);
    Ok(Expression::None)
}
