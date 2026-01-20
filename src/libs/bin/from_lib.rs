// use{get_list_arg, get_string_arg};
use crate::{
    Environment, Expression, RuntimeError,
    libs::{
        BuiltinInfo,
        helper::{check_args_len, check_exact_args_len},
        lazy_module::LazyModule,
    },
    parse, reg_info, reg_lazy,
};
use regex_lite::Regex;
use std::collections::{BTreeMap, HashMap};
use tinyjson::JsonValue;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 数据格式解析
        toml, json, csv,
        // 表达式解析
        script,
        // 命令输出解析
        cmd,
        // 数据查询
        jq,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({

        // 数据格式解析
        toml => "parse TOML into lumesh expression", "<toml_string>"
        json => "parse JSON into lumesh expression", "<json_string>"
        csv => "parse CSV into lumesh expression", "<csv_string>"

        // 表达式解析
        script => "parse script str to lumesh expression", "<script_string>"

        // 命令输出解析
        cmd => "parse command output into structured data", "[headers|header...] <cmd_output_string>"

        // 数据查询
        jq => "Apply jq-like query to JSON or TOML data", "<query_string> <json_data>"

    })
}

// TOML Parser Functions

fn toml(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("toml", args, 1, ctx)?;
    let text = args[0].eval(env)?;
    let text_str = text.to_string();

    toml::from_str(&text_str).map(toml_to_expr).map_err(|e| {
        RuntimeError::common(format!("Toml parser error:\n{e}").into(), ctx.clone(), 0)
    })
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

fn json(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("json", args, 1, ctx)?;
    let text = args[0].eval(env)?;
    let text_str = text.to_string();

    if text_str.is_empty() {
        return Ok(Expression::None);
    }

    text_str
        .parse::<JsonValue>()
        .map(json_to_expr)
        .map_err(|e| {
            RuntimeError::common(format!("Json parser error:\n{e}").into(), ctx.clone(), 0)
        })
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

fn script(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("script", args, 1, ctx)?;
    let script = args[0].eval(env)?.to_string();

    if script.is_empty() {
        return Ok(Expression::None);
    }

    Ok(parse(&script).map_err(|e| {
        RuntimeError::common(format!("Script parser error:\n{e}").into(), ctx.clone(), 0)
    })?)
}

// Command Output Parser
pub fn cmd(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("cmd", args, 1.., ctx)?;

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

        _ => Vec::new(),
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

    // delimeter
    // let delimiter = match env.get("IFS") {
    //     Some(Expression::String(fs)) if fs != "\n" => fs,
    //     _ => " ".to_string(), // 使用空格作为默认分隔符
    // };

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
                        .replace("%", "")
                        .replace("(", "_")
                        .replace(")", "")
                        .replace("$", "")
                })
                .collect();
            (&lines[1..], detected)
        } else {
            // No headers detected, use column numbers
            let cols = lines[0]
                .split_whitespace()
                .enumerate()
                .map(|(i, _)| format!("C{i}"))
                .collect();
            (&lines[..], cols)
        }
    } else {
        // Use provided headers
        (&lines[..], headers)
    };

    let mut result = Vec::with_capacity(data_lines.len());
    for line in data_lines {
        if line.trim().is_empty() {
            continue;
        }

        let slist = line.split_whitespace().collect::<Vec<_>>();

        let mut row = BTreeMap::new();

        for (i, header) in detected_headers.iter().enumerate() {
            if let Some(&value) = slist.get(i) {
                row.insert(header.clone(), Expression::String(value.to_string()));
            } else {
                row.insert(header.clone(), Expression::None);
            }
        }

        if !row.is_empty() {
            result.push(Expression::from(row));
        }
    }
    Ok(Expression::from(result))
}

// CSV Reader and Converter Functions
fn csv(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("csv", args, 1, ctx)?;
    let text = args[0].eval(env)?.to_string();

    // 获取自定义分隔符
    let delimiter = match env.get("IFS") {
        Some(Expression::String(fs)) if fs != "\n" => fs.as_bytes()[0],
        _ => ",".as_bytes()[0].to_owned(), // 默认分隔符
    };

    // 设置 CSV 解析器的分隔符
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter) // 将字符串转换为字节并取第一个字符
        .from_reader(text.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|e| {
            RuntimeError::common(format!("Csv header error:\n{e}").into(), ctx.clone(), 0)
        })?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut result = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| {
            RuntimeError::common(format!("Csv parser error:\n{e}").into(), ctx.clone(), 0)
        })?;
        let mut row = BTreeMap::new();
        for (i, value) in record.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("C{i}"));
            row.insert(key, Expression::String(value.to_string()));
        }
        result.push(Expression::from(row));
    }
    Ok(Expression::from(result))
}

// 定义操作步骤的枚举
#[derive(Debug)]
enum JqStep {
    Field(String),
    Index(usize),
    Wildcard,
    Function(String, String),
}

fn jq(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("jq", args, 2, ctx)?;
    let query = args[0].eval(env)?;
    let input = args[1].eval(env)?;

    let json_value = match input {
        Expression::String(s) => s.parse::<JsonValue>().map_err(|e| {
            RuntimeError::common(format!("Json parser error:\n{e}").into(), ctx.clone(), 0)
        })?,
        _ => {
            return Err(RuntimeError::common(
                "input must be a json string".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let query_result = match query {
        Expression::String(q) => {
            // 解析管道查询
            let pipeline = parse_jq_pipeline(&q);
            apply_jq_pipeline(&pipeline, &json_value)
        }

        _ => {
            return Err(RuntimeError::common(
                "Query must be a string or function".into(),
                ctx.clone(),
                0,
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
