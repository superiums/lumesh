use crate::{Environment, Expression, LmError, SyntaxError, SyntaxErrorKind, parse_script};
use common_macros::hash_map;
use std::collections::HashMap;
use tinyjson::JsonValue;

pub fn get() -> Expression {
    (hash_map! {
        String::from("toml") => Expression::builtin("toml", parse_toml, "parse a TOML value into a lumesh expression"),
        String::from("json") => Expression::builtin("json", parse_json, "parse a JSON value into a lumesh expression"),
        String::from("expr") => Expression::builtin("expr", parse_expr, "parse a lumesh script"),
        String::from("parse_cmd") => Expression::builtin("parse_cmd", parse_command_output,
            "parse command output (like ls -l, lsblk) into a list of maps"),
        String::from("where") => Expression::builtin("where", filter_rows,
            "filter rows in a list of maps based on condition"),
        String::from("select") => Expression::builtin("select", select_columns,
            "select specific columns from a list of maps"),
    })
    .into()
}

fn parse_toml(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("toml", &args, 1)?;
    let text = args[0].eval(env)?.to_string();
    if let Ok(val) = text.parse::<toml::Value>() {
        Ok(toml_to_expr(val))
    } else {
        Err(LmError::CustomError(format!(
            "could not parse `{}` as TOML",
            text
        )))
    }
}

fn toml_to_expr(val: toml::Value) -> Expression {
    match val {
        toml::Value::Boolean(b) => Expression::Boolean(b),
        toml::Value::Float(n) => Expression::Float(n),
        toml::Value::Integer(n) => Expression::Integer(n),
        toml::Value::Datetime(s) => Expression::String(s.to_string()),
        toml::Value::String(s) => Expression::String(s),
        toml::Value::Array(a) => {
            let mut v = Vec::new();
            for e in a {
                v.push(toml_to_expr(e));
            }
            Expression::from(v)
        }
        toml::Value::Table(o) => {
            let mut m = HashMap::new();
            for (k, v) in o.iter() {
                m.insert(k.to_string(), toml_to_expr(v.clone()));
            }
            Expression::from(m)
        }
    }
}

fn parse_expr(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("expr", &args, 1)?;
    let script = args[0].eval(env)?.to_string();
    if script.is_empty() {
        return Ok(Expression::None);
    }
    match parse_script(&script) {
        Ok(val) => Ok(val),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(SyntaxError {
            source: script.into(),
            kind: e,
        })?,
        Err(nom::Err::Incomplete(_)) => Err(SyntaxError {
            source: script.into(),
            kind: SyntaxErrorKind::InternalError,
        })?,
    }
}

fn parse_json(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("json", &args, 1)?;
    let text = args[0].eval(env)?.to_string();
    if text.is_empty() {
        return Ok(Expression::None);
    }

    match text.parse::<JsonValue>() {
        Ok(val) => Ok(json_to_expr(val)),
        Err(e) => Err(LmError::CustomError(format!("could not parse JSON: {}", e))),
    }
}

fn json_to_expr(val: JsonValue) -> Expression {
    match val {
        JsonValue::Null => Expression::None,
        JsonValue::Boolean(b) => Expression::Boolean(b),
        JsonValue::Number(n) => {
            // tinyjson的Number类型是f64，需要区分整数和浮点数
            if n.fract() == 0.0 {
                Expression::Integer(n as i64)
            } else {
                Expression::Float(n)
            }
        }
        JsonValue::String(s) => Expression::String(s),
        JsonValue::Array(a) => {
            let v: Vec<Expression> = a.into_iter().map(json_to_expr).collect();
            Expression::from(v)
        }
        JsonValue::Object(o) => {
            let m: HashMap<String, Expression> =
                o.into_iter().map(|(k, v)| (k, json_to_expr(v))).collect();
            Expression::from(m)
        }
    }
}

// 其他函数保持不变...

/// 解析命令行输出为结构化数据
fn parse_command_output(
    args: Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    super::check_args_len("parse_cmd", &args, 1..2)?;

    let output = args.last().unwrap().eval(env)?.to_string();
    let headers = if args.len() > 1 {
        match args[0].eval(env)? {
            Expression::List(list) => list
                .as_ref()
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>(),
            _ => return Err(LmError::CustomError("Headers must be a list".to_string())),
        }
    } else {
        Vec::new()
    };

    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return Ok(Expression::from(Vec::<Expression>::new()));
    }

    // 尝试自动检测表头
    let (header_line, data_lines) = if lines[0].chars().any(|c| c.is_uppercase()) {
        (&lines[0], &lines[1..])
    } else {
        (&"", &lines[..])
    };

    // 解析列名
    let column_names = if !headers.is_empty() {
        headers
    } else if !header_line.is_empty() {
        // 从命令输出中提取列名
        header_line
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        // 使用数字作为列名
        data_lines
            .first()
            .map(|line| {
                (0..line.split_whitespace().count())
                    .map(|i| i.to_string())
                    .collect()
            })
            .unwrap_or_default()
    };

    // 解析数据行
    let mut result = Vec::new();
    for line in data_lines {
        let values: Vec<&str> = line.split_whitespace().collect();
        if values.is_empty() {
            continue;
        }

        let mut row = HashMap::new();
        for (i, value) in values.iter().enumerate() {
            if let Some(col_name) = column_names.get(i) {
                row.insert(col_name.clone(), Expression::String(value.to_string()));
            }
        }

        if !row.is_empty() {
            result.push(Expression::from(row));
        }
    }

    Ok(Expression::from(result))
}

/// 筛选行 (类似SQL WHERE)
fn filter_rows(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("where", &args, 2)?;

    let data = match args[0].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "Expected list of maps for filtering".to_string(),
            ));
        }
    };

    let condition = args[1].eval(env)?;

    let mut filtered = Vec::new();
    for row in data.as_ref() {
        if let Expression::Map(row_map) = row {
            // 创建临时环境包含当前行
            let mut row_env = env.fork();
            for (k, v) in row_map.as_ref().iter() {
                row_env.define(k, v.clone());
            }

            // 评估条件
            match condition.clone().eval(&mut row_env)? {
                Expression::Boolean(true) => filtered.push(row.clone()),
                Expression::Boolean(false) => (),
                _ => {
                    return Err(LmError::CustomError(
                        "Condition must evaluate to boolean".to_string(),
                    ));
                }
            }
        }
    }

    Ok(Expression::from(filtered))
}

/// 选择列 (类似SQL SELECT)
fn select_columns(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("select", &args, 1..2)?;

    let data = match args[0].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "Expected list of maps for column selection".to_string(),
            ));
        }
    };

    let columns = if args.len() > 1 {
        match args[1].eval(env)? {
            Expression::List(list) => list
                .as_ref()
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>(),
            Expression::String(s) => vec![s],
            _ => {
                return Err(LmError::CustomError(
                    "Columns must be a list or string".to_string(),
                ));
            }
        }
    } else {
        // 如果没有指定列，返回所有列
        data.as_ref()
            .first()
            .and_then(|row| {
                if let Expression::Map(map) = row {
                    Some(map.as_ref().keys().cloned().collect())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };

    let mut result = Vec::new();
    for row in data.as_ref() {
        if let Expression::Map(row_map) = row {
            let mut selected = HashMap::new();
            for col in &columns {
                if let Some(val) = row_map.as_ref().get(col) {
                    selected.insert(col.clone(), val.clone());
                }
            }
            result.push(Expression::from(selected));
        }
    }

    Ok(Expression::from(result))
}
