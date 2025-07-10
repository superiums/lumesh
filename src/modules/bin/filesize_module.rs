use crate::{Environment, Expression, Int, LmError, expression::FileSize};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        String::from("from") => Expression::builtin("from", from, "create a Filesize", "<size_str|byte_int>"),
        String::from("to_string") => Expression::builtin("to_string", to_string, "Filesize to human readable string", "<filesize>"),
        String::from("b") => Expression::builtin("b", bytes, "get btyes of a Filesize", "<filesize>"),
        String::from("kb") => Expression::builtin("kb", kb, "get kb of a Filesize", "<filesize>"),
        String::from("mb") => Expression::builtin("mb", mb, "get mb of a Filesize", "<filesize>"),
        String::from("gb") => Expression::builtin("gb", gb, "get gb of a Filesize", "<filesize>"),
        String::from("tb") => Expression::builtin("tb", tb, "get tb of a Filesize", "<filesize>"),

    }).into()
}

fn from(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("from", args, 1)?;
    let s = match &args[0].eval(env)? {
        Expression::String(s) => {
            // 使用正则表达式来匹配数字和单位
            let re = regex_lite::Regex::new(r"(\d+)([KMGT]*)B?").unwrap();

            if let Some(caps) = re.captures(s) {
                // 提取数字部分并转换为u64
                let number = caps[1]
                    .parse::<u64>()
                    .map_err(|e| LmError::CustomError(e.to_string()))?;
                // 提取单位部分并转换为String
                let unit = caps[2].as_ref();
                FileSize::from(number, unit)
            } else {
                return Err(LmError::CustomError("invalid Filesize string: `{}`".into()));
            }
        }
        Expression::Integer(i) => FileSize::from_bytes(*i as u64),
        Expression::FileSize(r) => r.clone(),
        other => {
            return Err(LmError::TypeError {
                expected: "String/Integer as Filesize".into(),
                found: other.type_name(),
                sym: other.to_string(),
            });
        }
    };

    Ok(Expression::FileSize(s))
}

fn bytes(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("btyes", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?;
    Ok(Expression::Integer(s.to_bytes() as Int))
}
fn kb(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("kb", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?;

    Ok(Expression::Integer((s.to_bytes() >> 10) as Int))
}
fn mb(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("mb", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?.to_bytes();
    let r = (s >> 20) as f64 + ((s >> 10) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn gb(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("gb", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?.to_bytes();
    let r = (s >> 30) as f64 + ((s >> 20) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn tb(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("tb", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?.to_bytes();
    let r = (s >> 40) as f64 + ((s >> 30) & 1023) as f64 * 0.0009765625;

    Ok(Expression::Float(r))
}
fn to_string(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_string", args, 1)?;
    let s = get_fsize_arg(&args[0], env)?;

    Ok(Expression::String(s.to_human_readable()))
}

fn get_fsize_arg(arg: &Expression, env: &mut Environment) -> Result<FileSize, LmError> {
    match arg.eval(env)? {
        Expression::FileSize(s) => Ok(s),
        _ => Err(LmError::CustomError(
            "Filesize.bytes requires only Filesize as argument".into(),
        )),
    }
}
