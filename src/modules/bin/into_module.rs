use crate::{
    Environment, Expression, Int, LmError,
    expression::FileSize,
    modules::bin::{
        check_exact_args_len,
        parse_module::{expr_to_csv, expr_to_json, expr_to_toml, parse_command_output},
        time_module::parse,
    },
};
use common_macros::hash_map;

pub fn get() -> Expression {
    let into_module = hash_map! {
            String::from("str") => Expression::builtin("str", str, "format an expression to a string"),
            String::from("int") => Expression::builtin("int", int, "convert a float or string to an int"),
            String::from("float") => Expression::builtin("float", float, "convert a int or string to an float"),
            String::from("boolean") => Expression::builtin("boolean", boolean, "convert a int or string to an float"),
            String::from("filesize") => Expression::builtin("filesize", filesize, "convert a int or string to an float"),
            String::from("time") => Expression::builtin("time", parse, "convert a string to a datetime"),
            String::from("table") => Expression::builtin("time", parse_command_output, "convert third-part cmd output to a table"),
            String::from("toml") => Expression::builtin("to_toml", expr_to_toml, "parse lumesh expression into TOML"),
            String::from("json") => Expression::builtin("to_json", expr_to_json, "parse lumesh expression into JSON"),
            String::from("csv") => Expression::builtin("to_csv", expr_to_csv, "parse lumesh expression into CSV"),
    };
    Expression::from(into_module)
}

fn boolean(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("boolean", args, 1)?;
    Ok(Expression::Boolean(args[0].eval(env)?.is_truthy()))
}
fn str(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("str", args, 1)?;
    Ok(Expression::String(args[0].eval(env)?.to_string()))
}

fn int(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("int", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::Integer(x)),
        Expression::Float(x) => Ok(Expression::Integer(x as Int)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<Int>() {
                Ok(Expression::Integer(n))
            } else {
                Err(LmError::CustomError(format!(
                    "could not convert {:?} to an integer",
                    x
                )))
            }
        }
        otherwise => Err(LmError::CustomError(format!(
            "could not convert {:?} to an integer",
            otherwise
        ))),
    }
}

fn float(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("float", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::Float(x as f64)),
        Expression::Float(x) => Ok(Expression::Float(x)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<f64>() {
                Ok(Expression::Float(n))
            } else {
                Err(LmError::CustomError(format!(
                    "could not convert {:?} to a float",
                    x
                )))
            }
        }
        otherwise => Err(LmError::CustomError(format!(
            "could not convert {:?} to a float",
            otherwise
        ))),
    }
}

fn filesize(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    check_exact_args_len("filesize", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::FileSize(FileSize::from_bytes(x as u64))),
        Expression::Float(x) => Ok(Expression::FileSize(FileSize::from_bytes(x as u64))),
        Expression::FileSize(x) => Ok(Expression::FileSize(x)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<u64>() {
                Ok(Expression::FileSize(FileSize::from_bytes(n)))
            } else if let Some((num, unit)) = split_file_size(&x) {
                Ok(Expression::FileSize(FileSize::from(num as u64, unit)))
            } else {
                Err(LmError::CustomError(format!(
                    "could not convert {:?} to a filesize",
                    x
                )))
            }
        }
        otherwise => Err(LmError::CustomError(format!(
            "could not convert {:?} to a filesize",
            otherwise
        ))),
    }
}
fn split_file_size(size_str: &str) -> Option<(f64, &'static str)> {
    // 定义单位数组
    let units = ["B", "K", "M", "G", "T", "P"];

    // 去除字符串中的空格
    let trimmed = size_str.trim();

    // 查找单位
    let mut unit_index = 0;
    for unit in units {
        // 检查单位是否在字符串中
        if let Some(pos) = trimmed.find(unit) {
            // 提取数字部分
            let number_part = &trimmed[..pos].trim();
            let number: f64 = number_part.parse().ok()?;
            if number_part.contains(".") && unit_index > 0 {
                // 处理可选的"B"
                return Some((number * 1024 as f64, units[unit_index - 1]));
            }
            return Some((number, unit));
        }
        unit_index += 1;
    }

    // 如果没有找到单位，返回None
    None
}
