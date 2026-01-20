use crate::{
    Environment, Expression, RuntimeError, VERSION,
    libs::{BuiltinInfo, lazy_module::LazyModule},
    reg_info, reg_lazy,
};
use common_macros::hash_map;
use std::collections::BTreeMap;
use std::env::current_exe;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        version,
        bin,
        prelude,
        info
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        version => "print version",""
        bin => "print bin path",""
        prelude => "print prelude path",""
        info => "print all info",""
    })
}

fn info(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let info = hash_map! {
        String::from("author") => Expression::String("Santo; Adam McDaniel".to_string()),
        String::from("git") => Expression::String("https://codeberg.com/santo/lumesh".to_string()),
        String::from("homepage") => Expression::String("https://lumesh.codeberg.page".to_string()),
        String::from("version") => Expression::String(VERSION.to_string()),
        String::from("bin") => bin(args, env, ctx)?,

        String::from("license") => Expression::String("MIT".to_string()),
        String::from("prelude") => prelude(args, env, ctx)?
    };
    Ok(Expression::from(info))
}
fn version(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::String(VERSION.to_string()))
}
fn bin(
    _args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    current_exe()
        .and_then(|b| Ok(Expression::String(b.to_string_lossy().to_string())))
        .map_err(|e| RuntimeError::from_io_error(e, "read current executor".into(), ctx.clone(), 0))
}
fn prelude(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(if let Some(c) = dirs::config_dir() {
        let prelude_path = c.join("lumesh").join("config.lm");
        if prelude_path.exists() {
            Expression::String(prelude_path.to_str().unwrap().to_string())
        } else {
            Expression::String(prelude_path.to_str().unwrap().to_string() + " !")
        }
    } else {
        Expression::String("config.lm".to_string())
    })
}
