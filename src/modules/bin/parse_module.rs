// use super::{get_list_arg, get_string_arg};
use crate::{Environment, Expression, LmError, parse};
use common_macros::hash_map;
use regex_lite::Regex;
use std::collections::BTreeMap;
use tinyjson::JsonValue;

use super::check_exact_args_len;

pub fn get() -> Expression {
    (hash_map! {
        // 数据格式解析
             String::from("toml") => Expression::builtin("toml", parse_toml,
                 "parse TOML into lumesh expression", "<toml_string>"),

             String::from("json") => Expression::builtin("json", parse_json,
                 "parse JSON into lumesh expression", "<json_string>"),

             String::from("csv") => Expression::builtin("csv", parse_csv,
                 "parse CSV into lumesh expression", "<csv_string>"),

             // 数据格式序列化
             String::from("to_toml") => Expression::builtin("to_toml", expr_to_toml,
                 "parse lumesh expression into TOML", "<expr>"),

             String::from("to_json") => Expression::builtin("to_json", expr_to_json,
                 "parse lumesh expression into JSON", "<expr>"),

             String::from("to_csv") => Expression::builtin("to_csv", expr_to_csv,
                 "parse lumesh expression into CSV", "<expr>"),

             // 表达式解析
             String::from("expr") => Expression::builtin("expr", parse_expr,
                 "parse script str to lumesh expression", "<script_string>"),

             // 命令输出解析
             String::from("cmd") => Expression::builtin("cmd", parse_command_output,
                 "parse command output into structured data", "<cmd_output_string>"),

             // 数据查询
             String::from("jq") => Expression::builtin("jq", jq,
                 "Apply jq-like query to JSON or TOML data", "<query_string> <json_data>"),
    })
    .into()
}

// TOML Parser Functions

fn parse_toml(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("toml", args, 1)?;
    let text = args[0].eval(env)?;
    let text_str = text.to_string();

    toml::from_str(&text_str)
        .map(toml_to_expr)
        .map_err(|e| LmError::CustomError(format!("TOML parse error: {}", e)))
}

fn toml_to_expr(val: toml::Value) -> Expression {
    match val {
        toml::Value::Boolean(b) => Expression::Boolean(b),
        toml::Value::Float(n) => Expression::Float(n),
        toml::Value::Integer(n) => Expression::Integer(n),
        toml::Value::Datetime(s) => Expression::String(s.to_string()),
        toml::Value::String(s) => Expression::String(s),
        toml::Value::Array(a) => {
            Expression::from(a.into_iter().map(toml_to_expr).collect::<Vec<Expression>>())
        }
        toml::Value::Table(o) => Expression::from(
            o.into_iter()
                .map(|(k, v)| (k, toml_to_expr(v)))
                .collect::<BTreeMap<String, Expression>>(),
        ),
    }
}

// JSON Parser Functions

fn parse_json(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("json", args, 1)?;
    let text = args[0].eval(env)?;
    let text_str = text.to_string();

    if text_str.is_empty() {
        return Ok(Expression::None);
    }

    text_str
        .parse::<JsonValue>()
        .map(json_to_expr)
        .map_err(|e| LmError::CustomError(format!("JSON parse error: {}", e)))
}

fn json_to_expr(val: JsonValue) -> Expression {
    match val {
        JsonValue::Null => Expression::None,
        JsonValue::Boolean(b) => Expression::Boolean(b),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 {
                Expression::Integer(n as i64)
            } else {
                Expression::Float(n)
            }
        }
        JsonValue::String(s) => Expression::String(s),
        JsonValue::Array(a) => {
            Expression::from(a.into_iter().map(json_to_expr).collect::<Vec<Expression>>())
        }
        JsonValue::Object(o) => Expression::from(
            o.into_iter()
                .map(|(k, v)| (k, json_to_expr(v)))
                .collect::<BTreeMap<String, Expression>>(),
        ),
    }
}

// Expression Parser

fn parse_expr(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("expr", args, 1)?;
    let script = args[0].eval(env)?.to_string();

    if script.is_empty() {
        return Ok(Expression::None);
    }

    Ok(parse(&script)?)
}

// Command Output Parser
pub fn parse_command_output(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    // super::check_args_len("parse_cmd", args, 1..)?;

    let headers = match args.len() {
        3.. => args[..args.len() - 1]
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>(),
        2 => {
            let a = args[0].eval(env)?;
            if let Expression::List(list) = a {
                list.as_ref()
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
            } else {
                vec![a.to_string()]
            }
        }

        1 => Vec::new(),
        0 => return Err(LmError::CustomError("no cmd outoupt received".into())),
    };

    let output = args.last().unwrap().eval(env)?.to_string();
    let mut lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return Ok(Expression::from(Vec::<Expression>::new()));
    } else {
        // 检测已经是列表格式的：首行首尾都是相同的符号
        let mut c = lines.first().unwrap().chars();
        if let Some(first) = c.next() {
            if first.is_ascii_punctuation() && Some(first) == c.last() {
                return Ok(args.last().unwrap().clone());
            }
        }
    }

    // filter too short tips lines
    if lines.len() > 2 {
        if lines[0].split_whitespace().collect::<Vec<&str>>().len()
            < lines[1].split_whitespace().collect::<Vec<&str>>().len()
        {
            lines.remove(0);
        }
        if lines
            .last()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .len()
            < lines[lines.len() - 2]
                .split_whitespace()
                .collect::<Vec<&str>>()
                .len()
        {
            lines.pop();
        }
    }
    // Try to detect if first line looks like headers
    let (data_lines, detected_headers) = if headers.is_empty() {
        let maybe_header = lines[0];
        let looks_like_header = maybe_header
            .split_whitespace()
            .all(|s| s.chars().any(|c| c.is_uppercase() || !c.is_ascii()));

        if looks_like_header {
            let detected = maybe_header
                .split_whitespace()
                .map(|s| {
                    s.replace(":", "_")
                        .replace("\"", "")
                        .replace("(", "_")
                        .replace(")", "")
                })
                .collect();
            (&lines[1..], detected)
        } else {
            // No headers detected, use column numbers
            let cols = lines[0]
                .split_whitespace()
                .enumerate()
                .map(|(i, _)| format!("C{}", i))
                .collect();
            (&lines[..], cols)
        }
    } else {
        // Use provided headers
        (&lines[..], headers)
    };

    let mut result = Vec::new();
    for line in data_lines {
        if line.trim().is_empty() {
            continue;
        }

        let values: Vec<&str> = line.split_whitespace().collect();
        let mut row = BTreeMap::new();

        for (i, header) in detected_headers.iter().enumerate() {
            if let Some(&value) = values.get(i) {
                row.insert(header.clone(), Expression::String(value.to_string()));
            }
        }

        if !row.is_empty() {
            result.push(Expression::from(row));
        }
    }
    Ok(Expression::from(result))
}

// CSV Reader and Converter Functions
fn parse_csv(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("csv", args, 1)?;
    let text = args[0].eval(env)?.to_string();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(text.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|e| LmError::CustomError(format!("CSV header error: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut result = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| LmError::CustomError(format!("CSV parse error: {}", e)))?;
        let mut row = BTreeMap::new();
        for (i, value) in record.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("C{}", i));
            row.insert(key, Expression::String(value.to_string()));
        }
        result.push(Expression::from(row));
    }
    Ok(Expression::from(result))
}

// Expression to TOML Conversion
pub fn expr_to_toml(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_toml", args, 1)?;
    let expr = &args[0].eval(env)?;
    let toml_str = expr_to_toml_string(expr, None);
    Ok(Expression::String(toml_str))
}

// fn expr_to_toml_string(expr: &Expression) -> String {
//     match expr {
//         Expression::None => "".to_string(),
//         Expression::Boolean(b) => b.to_string(),
//         Expression::Integer(i) => i.to_string(),
//         Expression::Float(f) => f.to_string(),
//         Expression::String(s) => format!("\"{}\"", s),
//         Expression::List(list) => {
//             let items: Vec<String> = list.iter().map(expr_to_toml_string).collect();
//             format!("[{}]", items.join(", "))
//         }
//         Expression::Map(map) => {
//             let pairs: Vec<String> = map
//                 .iter()
//                 .map(|(k, v)| format!("{} = {}", k, expr_to_toml_string(v)))
//                 .collect();
//             pairs.join("\n")
//         }
//         other => other.to_string(),
//     }
// }

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
                    Some(prefix) => format!("{}.{}", prefix, table_name),
                    None => table_name.clone(),
                };

                // 添加表头
                output.push(format!("\n[{}]", full_table_name));

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
pub fn expr_to_json(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    check_exact_args_len("to_json", args, 1)?;
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
        Expression::String(s) => format!("\"{}\"", s),
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

// fn expr_to_csv_string(expr: &Expression) -> String {
//     match expr {
//         Expression::None => "null".to_string(),
//         Expression::List(list) => {
//             let items: Vec<String> = list.iter().map(expr_to_json_string).collect();
//             format!("[{}]", items.join(","))
//         }
//         Expression::Map(map) => {
//             let pairs: Vec<String> = map
//                 .iter()
//                 .map(|(k, v)| format!("\"{}\":{}", k, expr_to_json_string(v)))
//                 .collect();
//             format!("{{{}}}", pairs.join(","))
//         }
//         other => other.to_string(),
//     }
// }

// Expression to CSV
pub fn expr_to_csv(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("to_csv", args, 1)?;
    let expr = &args[0].eval(env)?;

    let result = match expr {
        Expression::List(rows) => {
            let mut writer = csv::Writer::from_writer(vec![]);

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
            let mut writer = csv::Writer::from_writer(vec![]);
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

// 定义操作步骤的枚举
#[derive(Debug)]
enum JqStep {
    Field(String),
    Index(usize),
    Wildcard,
    Function(String, String),
}

fn jq(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("jq", args, 2)?;
    let query = args[0].eval(env)?;
    let input = args[1].eval(env)?;

    let json_value = match input {
        Expression::String(s) => s
            .parse::<JsonValue>()
            .map_err(|_| LmError::CustomError("Invalid JSON string".into()))?,
        _ => return Err(LmError::CustomError("Input must be a JSON string".into())),
    };

    let query_result = match query {
        Expression::String(q) => {
            // 解析管道查询
            let pipeline = parse_jq_pipeline(&q);
            apply_jq_pipeline(&pipeline, &json_value)
        }

        _ => {
            return Err(LmError::CustomError(
                "Query must be a string or function".into(),
            ));
        }
    };

    Ok(Expression::String(
        query_result.stringify().unwrap_or("".to_string()),
    ))
}

// 解析管道查询字符串
fn parse_jq_pipeline(query: &str) -> Vec<JqStep> {
    let mut steps = Vec::new();
    // 按管道符分割查询
    for part in query.split('|').map(|s| s.trim()) {
        if part.starts_with("select(") && part.ends_with(')') {
            // 处理select函数
            let arg = &part[7..part.len() - 1];
            steps.push(JqStep::Function("select".to_string(), arg.to_string()));
        } else if part == ".[]" {
            // 处理通配符
            steps.push(JqStep::Wildcard);
        } else if part.starts_with('[') && part.ends_with(']') {
            // 处理数组索引
            let index_str = &part[1..part.len() - 1];
            if let Ok(index) = index_str.parse::<usize>() {
                steps.push(JqStep::Index(index));
            }
        } else if part.starts_with('.') {
            // 处理字段访问
            let field_name = part.trim_start_matches('.').to_string();
            steps.push(JqStep::Field(field_name));
        }
    }
    steps
}

// 应用管道查询
fn apply_jq_pipeline(pipeline: &[JqStep], json_value: &JsonValue) -> JsonValue {
    let mut current_value = json_value.clone();
    for step in pipeline {
        current_value = apply_jq_step(step, &current_value);
    }
    current_value
}

// 应用单个查询步骤
fn apply_jq_step(step: &JqStep, json_value: &JsonValue) -> JsonValue {
    match step {
        JqStep::Field(field) => {
            if let JsonValue::Object(obj) = json_value {
                obj.get(field).cloned().unwrap_or(JsonValue::Null)
            } else {
                JsonValue::Null
            }
        }
        JqStep::Index(index) => {
            if let JsonValue::Array(arr) = json_value {
                if *index < arr.len() {
                    arr[*index].clone()
                } else {
                    JsonValue::Null
                }
            } else {
                JsonValue::Null
            }
        }
        JqStep::Wildcard => {
            if let JsonValue::Array(arr) = json_value {
                // 通配符返回整个数组
                JsonValue::Array(arr.clone())
            } else {
                JsonValue::Null
            }
        }
        JqStep::Function(func_name, arg) => {
            if func_name == "select" {
                apply_select_function(arg, json_value)
            } else {
                JsonValue::Null
            }
        }
    }
}

// 应用select函数
fn apply_select_function(condition: &str, json_value: &JsonValue) -> JsonValue {
    // 简化版条件解析：只支持数字比较
    let re = Regex::new(r"\.(\w+)\s*([><=!]+)\s*(\d+)").unwrap();

    if let Some(caps) = re.captures(condition) {
        let field = caps.get(1).unwrap().as_str();
        let op = caps.get(2).unwrap().as_str();
        let value: i64 = caps.get(3).unwrap().as_str().parse().unwrap();

        if let JsonValue::Array(arr) = json_value {
            let filtered: Vec<JsonValue> = arr
                .iter()
                .filter(|item| {
                    if let JsonValue::Object(obj) = item {
                        if let Some(JsonValue::Number(n)) = obj.get(field) {
                            let n_int = *n as i64;
                            match op {
                                ">" => n_int > value,
                                "<" => n_int < value,
                                ">=" => n_int >= value,
                                "<=" => n_int <= value,
                                "==" | "=" => n_int == value,
                                "!=" => n_int != value,
                                _ => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            JsonValue::Array(filtered)
        } else {
            JsonValue::Null
        }
    } else {
        JsonValue::Null
    }
}
