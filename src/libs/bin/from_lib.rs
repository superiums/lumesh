// use{get_list_arg, get_string_arg};
use crate::{
    Environment, Expression, RuntimeError,
    expression::table::TableData,
    libs::{
        BuiltinInfo,
        bin::into_lib,
        helper::{check_exact_args_len, get_string_ref},
        lazy_module::LazyModule,
    },
    parse, reg_info, reg_lazy,
};
use regex_lite::Regex;
use std::collections::BTreeMap;
use tinyjson::JsonValue;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 数据格式解析
        toml, json, csv,
        // 表达式解析
        script,
        // 解析第三方命令输出（into库）
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
        cmd => "parse command output into structured data", "<cmd_output_string> [headers|header...]"

        // 数据查询
        jq => "Apply jq-like query to JSON or TOML data", "<query_string> <json_data>"

    })
}

// TOML Parser Functions

fn toml(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("toml", &args, 1, ctx)?;
    let text_str = get_string_ref(&args[0], ctx)?;

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
            // Check if this is an array of tables that could be a table
            if let Some((headers, rows)) = try_convert_toml_array_to_table(&a) {
                Expression::Table(TableData::new(headers, rows))
            } else {
                Expression::from(a.into_iter().map(toml_to_expr).collect::<Vec<Expression>>())
            }
        }
        toml::Value::Table(o) => Expression::from(
            o.into_iter()
                .map(|(k, v)| (k, toml_to_expr(v)))
                .collect::<BTreeMap<String, Expression>>(),
        ),
    }
}

// Helper function for TOML array of tables
fn try_convert_toml_array_to_table(
    arr: &[toml::Value],
) -> Option<(Vec<String>, Vec<Vec<Expression>>)> {
    if arr.is_empty() {
        return None;
    }

    // Check if all elements are tables
    if !arr.iter().all(|v| matches!(v, toml::Value::Table(_))) {
        return None;
    }

    // Similar logic to JSON version...
    let mut all_keys = std::collections::BTreeSet::new();
    for item in arr {
        if let toml::Value::Table(table) = item {
            for key in table.keys() {
                all_keys.insert(key.clone());
            }
        }
    }

    let headers: Vec<String> = all_keys.into_iter().collect();
    let mut rows = Vec::new();

    for item in arr {
        if let toml::Value::Table(table) = item {
            let row: Vec<Expression> = headers
                .iter()
                .map(|key| {
                    table
                        .get(key)
                        .map(|v| toml_to_expr(v.clone()))
                        .unwrap_or(Expression::None)
                })
                .collect();
            rows.push(row);
        }
    }

    Some((headers, rows))
}

// JSON Parser Functions
fn json(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("json", &args, 1, ctx)?;
    let text_str = get_string_ref(&args[0], ctx)?;

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

// TODO: add bset if needed
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
            // Check if this is an array of objects that could be a table
            if let Some((headers, rows)) = try_convert_array_to_table(&a) {
                Expression::Table(TableData::new(headers, rows))
            } else {
                Expression::from(a.into_iter().map(json_to_expr).collect::<Vec<Expression>>())
            }
        }
        JsonValue::Object(o) => Expression::from(
            o.into_iter()
                .map(|(k, v)| (k, json_to_expr(v)))
                .collect::<BTreeMap<String, Expression>>(),
        ),
    }
}

// Helper function to detect and convert array of objects to table
fn try_convert_array_to_table(arr: &[JsonValue]) -> Option<(Vec<String>, Vec<Vec<Expression>>)> {
    if arr.is_empty() {
        return None;
    }

    // Check if all elements are objects
    if !arr.iter().all(|v| matches!(v, JsonValue::Object(_))) {
        return None;
    }

    // Collect all unique keys from all objects
    let mut all_keys = std::collections::BTreeSet::new();
    for item in arr {
        if let JsonValue::Object(obj) = item {
            for key in obj.keys() {
                all_keys.insert(key.clone());
            }
        }
    }

    let headers: Vec<String> = all_keys.into_iter().collect();
    let mut rows = Vec::new();

    for item in arr {
        if let JsonValue::Object(obj) = item {
            let row: Vec<Expression> = headers
                .iter()
                .map(|key| {
                    obj.get(key)
                        .map(|v| json_to_expr(v.clone()))
                        .unwrap_or(Expression::None)
                })
                .collect();
            rows.push(row);
        }
    }

    Some((headers, rows))
}

// Expression Parser
fn script(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("script", &args, 1, ctx)?;
    let script = get_string_ref(&args[0], ctx)?;

    if script.is_empty() {
        return Ok(Expression::None);
    }

    Ok(parse(&script).map_err(|e| {
        RuntimeError::common(format!("Script parser error:\n{e}").into(), ctx.clone(), 0)
    })?)
}

// Command Output Parser
fn cmd(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    into_lib::table(args, env, ctx)
}

// CSV Reader and Converter Functions
fn csv(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("csv", &args, 1, ctx)?;
    let text = get_string_ref(&args[0], ctx)?;

    // 获取自定义分隔符
    let delimiter = match env.get("IFS") {
        Some(Expression::String(fs)) if fs != "\n" => fs.as_bytes()[0],
        _ => ",".as_bytes()[0].to_owned(), // 默认分隔符
    };

    // 设置 CSV 解析器的分隔符
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter) // 将字符串转换为字节并取第一个字符
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| {
            RuntimeError::common(format!("Csv header error:\n{e}").into(), ctx.clone(), 0)
        })?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut table = TableData::with_header(headers);
    for rec in reader.records() {
        let record = rec.map_err(|e| {
            RuntimeError::common(format!("CSV parse error: {e}").into(), ctx.clone(), 0)
        })?;

        let row: Vec<Expression> = record
            .iter()
            .map(|field| Expression::String(field.to_string()))
            .collect();
        table.push_row(row);
    }

    Ok(Expression::Table(table))
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("jq", &args, 2, ctx)?;
    let query = &args[0];
    let input = &args[1];

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
