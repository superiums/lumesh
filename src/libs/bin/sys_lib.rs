use std::rc::Rc;

use common_macros::hash_map;

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_exact_args_len, get_integer_ref};
use crate::{
    CFM_ENABLED, Environment, Expression, Int, LmError, MAX_RUNTIME_RECURSION,
    MAX_SYNTAX_RECURSION, MAX_USEMODE_RECURSION, PRINT_DIRECT, RuntimeError, STRICT_ENABLED,
    set_cfm_enabled, set_print_direct, set_strict_enabled,
};
use std::collections::BTreeMap;

use crate::libs::lazy_module::LazyModule;
use crate::{reg_info, reg_lazy};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        dirs, env, vars, has, defined,
        quote, ecodes_rt, ecodes_lm,

        info,modes,
        // throw,
        max_syntax,
        max_runtime,
        max_usemode,
        set_cfm,
        set_pdm,
        set_strict
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        dirs => "get system directories", ""
        env => "get root environment map/var value", "[var]"
        vars => "get defined variables in current enviroment", ""
        has => "check if a variable is defined in current environment", "<var>"
        defined => "check if a variable is defined in current environment tree", "<var>"

        quote => "quote an expression", "<expr>"
        ecodes_rt => "display runtime error codes", ""
        ecodes_lm => "display Lmerror codes", ""
        // throw => "return a runtime error", "<msg>"

        info => "get os info", ""
        modes => "get lume modes", ""

        max_syntax => "get/set max syntax recursion","[int]"
        max_runtime=> "get/set max runtime recursion","[int]"
        max_usemode=> "get/set max use mode recursion","[int]"
        set_cfm=> "enable/disable CFM","<boolean>"
        set_pdm=> "enable/disable print direct mode","<boolean>"
        set_strict=> "enable/disable strict mode","<boolean>"

    })
}

// System Directory Functions
fn dirs(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut dir_tree = BTreeMap::<String, String>::new();

    if let Some(home_dir) = dirs::home_dir() {
        dir_tree.insert("home".into(), home_dir.to_string_lossy().into());
    }
    if let Some(config_dir) = dirs::config_dir() {
        dir_tree.insert("config".into(), config_dir.to_string_lossy().into());
    }
    if let Some(cache_dir) = dirs::cache_dir() {
        dir_tree.insert("cache".into(), cache_dir.to_string_lossy().into());
    }
    if let Some(data_dir) = dirs::data_dir() {
        dir_tree.insert("data".into(), data_dir.to_string_lossy().into());
    }
    if let Some(picture_dir) = dirs::picture_dir() {
        dir_tree.insert("pic".into(), picture_dir.to_string_lossy().into());
    }
    if let Some(desktop_dir) = dirs::desktop_dir() {
        dir_tree.insert("desk".into(), desktop_dir.to_string_lossy().into());
    }
    if let Some(document_dir) = dirs::document_dir() {
        dir_tree.insert("docs".into(), document_dir.to_string_lossy().into());
    }
    if let Some(download_dir) = dirs::download_dir() {
        dir_tree.insert("down".into(), download_dir.to_string_lossy().into());
    }

    Ok(Expression::from(dir_tree))
}

fn info(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let info = os_info::get();
    Ok(Expression::String(info.to_string()))
}
fn modes(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(hash_map! {
        String::from("cfm") => CFM_ENABLED.with_borrow(|c|c==&true),
        String::from("strict") => STRICT_ENABLED.with_borrow(|c|c==&true),
        String::from("pdm") => PRINT_DIRECT.with_borrow(|c|c==&true),
    }))
}

fn quote(
    mut args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("quote", &args, 1, ctx)?;
    Ok(Expression::Quote(Rc::new(args.pop().unwrap())))
}

fn env(
    args: Vec<Expression>,
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.len() == 1 {
        let name = args[0].to_string();
        return Ok(env.get(&name).unwrap_or(Expression::None));
    }

    Ok(Expression::from(env.get_root().clone()))
}

fn vars(
    _args: Vec<Expression>,
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::from(env.get_bindings_map()))
}

fn has(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("has", &args, 1, ctx)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.has(&name)))
}
fn defined(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("defined", &args, 1, ctx)?;
    let name = args[0].to_string();
    Ok(Expression::Boolean(env.is_defined(&name)))
}

fn ecodes_rt(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(RuntimeError::codes())
}
fn ecodes_lm(
    _args: Vec<Expression>,
    _env: &mut Environment,
    _: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(LmError::codes())
}

fn max_syntax(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_SYNTAX_RECURSION.with_borrow(|x| *x as Int),
        ));
    }
    let i = get_integer_ref(&args[0], ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_SYNTAX_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn max_runtime(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_RUNTIME_RECURSION.with_borrow(|x| *x as Int),
        ));
    }
    let i = get_integer_ref(&args[0], ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_RUNTIME_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn max_usemode(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.is_empty() {
        return Ok(Expression::Integer(
            MAX_USEMODE_RECURSION.with_borrow(|x| *x as Int),
        ));
    }
    let i = get_integer_ref(&args[0], ctx)?;
    // MAX_SYNTAX_RECURSION = run_rec as usize;
    MAX_USEMODE_RECURSION.with_borrow_mut(|v| *v = i as usize);
    Ok(Expression::None)
}
fn set_strict(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_strict", &args, 1, ctx)?;
    let b = args[0].is_truthy();
    env.define("IS_STRICT", Expression::Boolean(b));
    set_strict_enabled(b);
    if b {
        println!("\x1b[38;5;141m[STRICT Mode: ON]\x1b[0m");
    } else {
        println!("\x1b[38;5;209m[STRICT Mode: OFF]\x1b[0m");
    }
    Ok(Expression::None)
}
fn set_cfm(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_cfm", &args, 1, ctx)?;
    let b = args[0].is_truthy();
    env.define_in_root("IS_CFM", Expression::Boolean(b));
    set_cfm_enabled(b);
    if b {
        println!("\x1b[38;5;141m[Cmd First Mode: ON]\x1b[0m");
    } else {
        println!("\x1b[38;5;209m[Cmd First Mode: OFF]\x1b[0m");
    }
    Ok(Expression::None)
}
fn set_pdm(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_pdm", &args, 1, ctx)?;
    let b = args[0].is_truthy();
    set_print_direct(b);
    if b {
        println!("\x1b[38;5;141m[Print Direct Mode: ON]\x1b[0m");
    } else {
        println!("\x1b[38;5;209m[Print Direct Mode: OFF]\x1b[0m");
    }
    Ok(Expression::None)
}
