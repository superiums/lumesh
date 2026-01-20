use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind,
    expression::FileSize,
    libs::{BuiltinInfo, helper::check_exact_args_len, lazy_module::LazyModule},
    reg_info, reg_lazy,
};
use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        from, to_string, b,
        kb, mb, gb, tb,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
            from => "create a Filesize", "<size_str|byte_int>"
            to_string => "Filesize to human readable string", "<filesize>"
            b =>  "get btyes of a Filesize", "<filesize>"
            kb => "get kb of a Filesize", "<filesize>"
            mb => "get mb of a Filesize", "<filesize>"
            gb => "get gb of a Filesize", "<filesize>"
            tb => "get tb of a Filesize", "<filesize>"

    })
}

fn from(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("from", args, 1, ctx)?;
    let s = match &args[0].eval(env)? {
        Expression::String(s) => {
            // 使用正则表达式来匹配数字和单位
            let re = regex_lite::Regex::new(r"(\d+)([KMGT]*)B?").unwrap();

            if let Some(caps) = re.captures(s) {
                // 提取数字部分并转换为u64
                let number = caps[1]
                    .parse::<u64>()
                    .map_err(|e| RuntimeError::common(e.to_string().into(), ctx.clone(), 0))?;
                // 提取单位部分并转换为String
                let unit = caps[2].as_ref();
                FileSize::from(number, unit)
            } else {
                return Err(RuntimeError::common(
                    format!("invalid Filesize string: `{s}`").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
        Expression::Integer(i) => FileSize::from_bytes(*i as u64),
        Expression::FileSize(r) => r.clone(),
        other => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "String/Integer as Filesize".into(),
                    found: other.type_name(),
                    sym: other.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::FileSize(s))
}

fn b(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("btyes", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?;
    Ok(Expression::Integer(s.to_bytes() as Int))
}
fn kb(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("kb", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?;

    Ok(Expression::Integer((s.to_bytes() >> 10) as Int))
}
fn mb(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("mb", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?.to_bytes();
    let r = (s >> 20) as f64 + ((s >> 10) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn gb(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("gb", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?.to_bytes();
    let r = (s >> 30) as f64 + ((s >> 20) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn tb(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("tb", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?.to_bytes();
    let r = (s >> 40) as f64 + ((s >> 30) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn to_string(
    args: &[Expression],
    env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_string", args, 1, _ctx)?;
    let s = get_fsize_arg(&args[0], env, _ctx)?;

    Ok(Expression::String(s.to_human_readable()))
}

fn get_fsize_arg(
    arg: &Expression,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<FileSize, RuntimeError> {
    match arg.eval(env)? {
        Expression::FileSize(s) => Ok(s),
        _ => Err(RuntimeError::common(
            "Filesize.bytes requires only Filesize as argument".into(),
            ctx.clone(),
            0,
        )),
    }
}
