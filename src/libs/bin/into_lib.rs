use std::collections::{BTreeMap, HashMap};

use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind,
    expression::FileSize,
    libs::{
        BuiltinInfo,
        helper::{
            check_args_len, check_exact_args_len, get_exact_string_arg, get_integer_arg,
            get_string_arg, get_string_args,
        },
        lazy_module::LazyModule,

        // bin::{from_module::parse_command_output, time_module::parse_time},
        pprint::strip_ansi_escapes,
    },
    reg_info, reg_lazy,
};

use crate::{
    runtime::{IFS_CSV, ifs_contains},
    syntax::highlight_dark_theme,
};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 类型转换函数（into库）
        str, int, float, boolean, filesize,
        // 时间解析（time库）
        // time,
        // 解析第三方命令输出（parse库）
        // table,
        // 数据格式序列化
        toml, json, csv,
        highlighted, striped,
    })
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({

        // 类型转换函数（into库）
        str => "format an expression to a string", "<value>"
        int => "convert a float or string to an int", "<value>"
        float => "convert an int or string to a float", "<value>"
        boolean => "convert a value to a boolean", "<value>"
        filesize => "parse a string representing a file size into bytes", "<size_str>"

        // 时间解析（time库）
        time => "convert a string to a datetime", "<datetime_str> [datetime_template]"

        // 解析第三方命令输出（parse库）
        table => "convert third-party command output to a table", "[headers|header...] <command_output>"

        // 数据格式序列化
        toml => "parse lumesh expression into TOML", "<expr>"
        json => "parse lumesh expression into JSON", "<expr>"
        csv => "parse lumesh expression into CSV", "<expr>"

        highlighted =>   "highlight script str with ANSI", "<script_string>"
        striped => "remove all ANSI escape codes from string", "<string>"
    })
}
fn boolean(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("boolean", args, 1, ctx)?;
    Ok(Expression::Boolean(args[0].eval(env)?.is_truthy()))
}

pub fn str(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("str", args, 1, ctx)?;
    Ok(Expression::String(args[0].eval(env)?.to_string()))
}

pub fn int(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("int", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::Integer(x)),
        Expression::Float(x) => Ok(Expression::Integer(x as Int)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<Int>() {
                Ok(Expression::Integer(n))
            } else {
                Err(RuntimeError::common(
                    format!("could not convert {x:?} to an integer").into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        otherwise => Err(RuntimeError::common(
            format!("could not convert {otherwise:?} to an integer").into(),
            ctx.clone(),
            0,
        )),
    }
}

pub fn float(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("float", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(x) => Ok(Expression::Float(x as f64)),
        Expression::Float(x) => Ok(Expression::Float(x)),
        Expression::String(x) => {
            let xt = x.trim();
            let r = match xt.ends_with("%") {
                true => xt
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .and_then(|f| Ok(f * 0.01)),
                false => xt.parse::<f64>(),
            };
            if let Ok(n) = r {
                Ok(Expression::Float(n))
            } else {
                Err(RuntimeError::common(
                    format!("could not convert {x:?} to a float").into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        otherwise => Err(RuntimeError::common(
            format!("could not convert {otherwise:?} to a float").into(),
            ctx.clone(),
            0,
        )),
    }
}

pub fn filesize(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filesize", args, 1, ctx)?;
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
                Err(RuntimeError::common(
                    format!("could not convert {x:?} to a filesize").into(),
                    ctx.clone(),
                    0,
                ))
            }
        }
        otherwise => Err(RuntimeError::common(
            format!("could not convert {otherwise:?} to a filesize").into(),
            ctx.clone(),
            0,
        )),
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
                return Some((number * 1024_f64, units[unit_index - 1]));
            }
            return Some((number, unit));
        }
        unit_index += 1;
    }

    // 如果没有找到单位，返回None
    None
}

// ===========parser==============

// Expression to TOML Conversion
pub fn toml(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("toml", args, 1, ctx)?;
    let expr = &args[0].eval(env)?;
    let toml_str = expr_to_toml_string(expr, None);
    Ok(Expression::String(toml_str))
}

// 递归序列化函数（新增表名前缀参数）
fn expr_to_toml_string(expr: &Expression, table_prefix: Option<&str>) -> String {
    match expr {
        // 基本类型处理
        Expression::None => "".to_string(),
        // Expression::Boolean(b) => b.to_string(),
        // Expression::Integer(i) => i.to_string(),
        // Expression::Float(f) => f.to_string(),

        // 字符串处理（禁用Unicode转义）
        Expression::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
        // Expression::DateTime(t) => t.to_string(),

        // 数组处理（保持原始结构）
        Expression::List(list) => {
            let items: Vec<String> = list.iter().map(|e| expr_to_toml_string(e, None)).collect();
            format!("[{}]", items.join(", "))
        }

        // 映射表处理（核心改进）
        Expression::Map(map) => {
            let mut output = Vec::new();
            let mut tables = BTreeMap::new();
            let mut simple_keys = BTreeMap::new();

            // 分离简单键和嵌套表
            for (key, value) in map.as_ref() {
                if let Expression::Map(_) = value {
                    tables.insert(key.clone(), value);
                } else {
                    simple_keys.insert(key.clone(), value);
                }
            }

            // 处理当前层简单键值对
            for (key, value) in &simple_keys {
                let line = format!("{} = {}", key, expr_to_toml_string(value, None));
                output.push(line);
            }

            // 处理嵌套表
            for (table_name, table_expr) in &tables {
                let full_table_name = match table_prefix {
                    Some(prefix) => format!("{prefix}.{table_name}"),
                    None => table_name.clone(),
                };

                // 添加表头
                output.push(format!("\n[{full_table_name}]"));

                // 递归处理子表
                let table_content = expr_to_toml_string(table_expr, Some(&full_table_name));

                // 添加子表内容（保留缩进）
                for line in table_content.lines() {
                    output.push(line.to_string());
                }
            }

            output.join("\n")
        }

        // 其他类型保持原样
        other => other.to_string(),
    }
}

// Expression to JSON Conversion (优化版)
pub fn json(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("json", args, 1, ctx)?;
    let expr = &args[0].eval(env)?;
    let json_str = match expr {
        Expression::Map(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        _ => expr_to_json_string(expr),
    };
    Ok(Expression::String(json_str))
}

fn expr_to_json_string(expr: &Expression) -> String {
    match expr {
        Expression::None => "null".to_string(),
        // Expression::Boolean(b) => b.to_string(),
        // Expression::Integer(i) => i.to_string(),
        // Expression::Float(f) => f.to_string(),
        Expression::String(s) => format!("\"{s}\""),
        Expression::List(list) => {
            let items: Vec<String> = list.iter().map(expr_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        Expression::Map(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        other => other.to_string(),
    }
}

// Expression to CSV
pub fn csv(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("csv", args, 1, ctx)?;
    let expr = &args[0].eval(env)?;

    // 获取自定义分隔符
    let ifs = env.get("IFS");
    let delimiter = match (ifs_contains(IFS_CSV, env), &ifs) {
        (true, Some(Expression::String(fs))) if fs != "\n" => fs.as_bytes()[0],
        _ => ",".as_bytes()[0],
    };

    let result = match expr {
        Expression::List(rows) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter) // 设置分隔符
                .from_writer(vec![]);

            // 获取所有可能的列名（按字母顺序）
            let mut all_keys = BTreeMap::new();
            for row in rows.as_ref() {
                if let Expression::Map(map) = row {
                    for key in map.keys() {
                        all_keys.insert(key.clone(), ());
                    }
                }
            }
            let sorted_keys: Vec<_> = all_keys.keys().collect();

            // 写入标题行
            writer.write_record(&sorted_keys).unwrap();

            // 写入数据行
            for row in rows.as_ref() {
                if let Expression::Map(map) = row {
                    let mut record = Vec::new();
                    for key in &sorted_keys {
                        // TODO while v is map/list
                        let value = map.get(*key).map(expr_to_json_string).unwrap_or_default();
                        record.push(value);
                    }
                    writer.write_record(&record).unwrap();
                }
            }

            String::from_utf8(writer.into_inner().unwrap()).unwrap()
        }
        Expression::Map(map) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter) // 设置分隔符
                .from_writer(vec![]);

            let sorted_keys: Vec<_> = map.keys().collect();

            writer.write_record(&sorted_keys).unwrap();

            let record: Vec<_> = sorted_keys
                .iter()
                .map(|k| expr_to_json_string(map.get(*k).unwrap()))
                .collect();

            writer.write_record(&record).unwrap();
            String::from_utf8(writer.into_inner().unwrap()).unwrap()
        }
        o => o.to_string(),
    };

    Ok(Expression::String(result))
}

fn highlighted(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("highlighted", args, 1, ctx)?;
    let script = args[0].eval(env)?.to_string();

    if script.is_empty() {
        return Ok(Expression::None);
    }

    let hi = highlight_dark_theme(script.as_str());
    Ok(Expression::String(hi))
}

// 单参数函数（字符串作为最后一个参数）
pub fn striped(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("striped", args, 1, ctx)?;
    Ok(strip_ansi_escapes(args[0].eval_in_assign(env)?.to_string().as_str()).into())
}
