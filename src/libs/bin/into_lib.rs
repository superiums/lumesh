use std::collections::BTreeMap;

use regex_lite::Regex;

use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind,
    expression::{FileSize, table::TableData},
    libs::{
        BuiltinInfo,
        bin::time_lib,
        helper::{check_args_len, check_exact_args_len, convert_list_map_to_table, get_string_ref},
        lazy_module::LazyModule,
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
        time,
        table,
        // 数据格式序列化
        toml, json, csv,
        highlighted, striped,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        str => "format an expression to a string", "<value>"
        int => "convert a float or string to an int", "<value>"
        float => "convert an int or string to a float", "<value>"
        boolean => "convert a value to a boolean", "<value>"
        filesize => "parse a string representing a file size into bytes", "<size_str>"
        time => "convert a string to a datetime", "<datetime_str> [datetime_template]"
        table => "convert third-party command output to a table", "<command_output> [regex|headers...]"
        // [FIX] "parse" → "serialize"
        toml => "serialize lumesh expression to TOML", "<expr>"
        json => "serialize lumesh expression to JSON", "<expr>"
        csv => "serialize lumesh expression to CSV", "<expr>"
        highlighted => "highlight script str with ANSI", "<script_string>"
        striped => "remove all ANSI escape codes from string", "<string>"
    })
}

pub fn time(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    time_lib::parse(args, env, ctx)
}

pub fn table(
    mut args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("table", &args, 1.., ctx)?;

    let opts = args.split_off(1);

    let data = match args.into_iter().next().unwrap() {
        Expression::String(s) => s,
        Expression::List(list) => {
            return Ok(Expression::Table(convert_list_map_to_table(&list)));
        }
        Expression::Table(t) => return Ok(Expression::Table(t)),
        e => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "String/List<Map>".into(),
                    found: e.type_name(),
                    sym: e.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }
    };

    let mut lines: Vec<&str> = data.lines().collect();
    if lines.is_empty() {
        return Ok(Expression::None);
    } else {
        // 检测已经是列表格式的：首行首尾都是相同的符号
        let mut c = lines.first().unwrap().chars();
        if let Some(first) = c.next()
            && first.is_ascii_punctuation()
            && Some(first) == c.last()
        {
            return Ok(Expression::String(data));
        }
    }

    let (headers, splitter): (Vec<String>, Option<Regex>) = match opts {
        s if s.is_empty() => (Vec::new(), None),
        s if s.len() == 1 => match s.first().unwrap() {
            Expression::List(list) => (list.as_ref().iter().map(|x| x.to_string()).collect(), None),
            Expression::BSet(list) => (list.as_ref().iter().map(|x| x.to_string()).collect(), None),
            Expression::Regex(r) => (Vec::new(), Some(r.regex.clone())),
            o => (vec![o.to_string()], None),
        },
        s if s.len() == 2 && matches!(s.first(), Some(Expression::Regex(_))) => {
            match (s.first().unwrap(), s.last().unwrap()) {
                (Expression::Regex(r), Expression::List(list)) => (
                    list.as_ref().iter().map(|x| x.to_string()).collect(),
                    Some(r.regex.clone()),
                ),
                (Expression::Regex(r), Expression::BSet(list)) => (
                    list.as_ref().iter().map(|x| x.to_string()).collect(),
                    Some(r.regex.clone()),
                ),
                (Expression::Regex(r), o) => (vec![o.to_string()], Some(r.regex.clone())),
                _ => (s.iter().map(|x| x.to_string()).collect(), None),
            }
        }
        s if matches!(s.first(), Some(Expression::Regex(_))) => (
            s.iter().skip(1).map(|x| x.to_string()).collect(),
            if let Some(Expression::Regex(r)) = s.first() {
                Some(r.regex.clone())
            } else {
                None
            },
        ),
        s => (s.iter().map(|x| x.to_string()).collect(), None),
    };

    if lines.len() > 2 {
        let first_line_cols = split_line(lines[0], &splitter);
        let second_line_cols = split_line(lines[1], &splitter);
        if first_line_cols.len() < second_line_cols.len() {
            lines.remove(0);
        }
        let last_line_cols = split_line(lines.last().unwrap(), &splitter);
        let second_last_line_cols = split_line(lines[lines.len() - 2], &splitter);
        if last_line_cols.len() < second_last_line_cols.len() {
            lines.pop();
        }
    }

    let (data_lines, detected_headers) = if headers.is_empty() {
        let maybe_header = lines[0];
        let first_line_cols = split_line(maybe_header, &splitter);
        let looks_like_header = first_line_cols
            .iter()
            .all(|s| s.chars().any(|c| c.is_uppercase() || !c.is_ascii()));

        if looks_like_header {
            let detected = first_line_cols
                .iter()
                .map(|s| {
                    s.replace(":", "_")
                        .replace("\"", "")
                        .replace("%", "")
                        .replace("(", "_")
                        .replace(")", "")
                        .replace("$", "")
                })
                .collect();
            (lines.split_off(1), detected)
        } else {
            let cols = first_line_cols
                .iter()
                .enumerate()
                .map(|(i, _)| format!("C{i}"))
                .collect();
            (lines, cols)
        }
    } else {
        (lines, headers)
    };

    let mut rows = Vec::with_capacity(data_lines.len());
    for line in data_lines {
        if line.trim().is_empty() {
            continue;
        }
        let slist: Vec<&str> = split_line(line, &splitter);
        let mut row = Vec::with_capacity(detected_headers.len());
        for (i, _header) in detected_headers.iter().enumerate() {
            if let Some(value) = slist.get(i) {
                row.push(Expression::String(value.to_string()));
            } else {
                row.push(Expression::None);
            }
        }
        if !row.is_empty() {
            rows.push(row);
        }
    }

    Ok(Expression::Table(TableData::new(detected_headers, rows)))
}

fn split_line<'a>(line: &'a str, regex: &Option<Regex>) -> Vec<&'a str> {
    match regex {
        Some(re) => re.split(line).collect(),
        None => line.split_whitespace().collect(),
    }
}

fn boolean(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("boolean", &args, 1, ctx)?;
    Ok(Expression::Boolean(args[0].is_truthy()))
}

pub fn str(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("str", &args, 1, ctx)?;
    Ok(Expression::String(args[0].to_string()))
}

pub fn int(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("int", &args, 1, ctx)?;
    match &args[0] {
        Expression::Integer(x) => Ok(Expression::Integer(*x)),
        Expression::Float(x) => Ok(Expression::Integer(*x as Int)),
        // [FIX] 新增 Boolean 处理
        Expression::Boolean(b) => Ok(Expression::Integer(if *b { 1 } else { 0 })),
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("float", &args, 1, ctx)?;
    match &args[0] {
        Expression::Integer(x) => Ok(Expression::Float(*x as f64)),
        Expression::Float(x) => Ok(Expression::Float(*x)),
        // [FIX] 新增 Boolean 处理
        Expression::Boolean(b) => Ok(Expression::Float(if *b { 1.0 } else { 0.0 })),
        Expression::String(x) => {
            let xt = x.trim();
            let r = match xt.ends_with("%") {
                true => xt.trim_end_matches('%').parse::<f64>().map(|f| f * 0.01),
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filesize", &args, 1, ctx)?;
    match args.into_iter().next().unwrap() {
        Expression::Integer(x) => Ok(Expression::FileSize(FileSize::from_bytes(x as u64))),
        Expression::Float(x) => Ok(Expression::FileSize(FileSize::from_bytes(x as u64))),
        Expression::FileSize(x) => Ok(Expression::FileSize(x)),
        Expression::String(x) => {
            if let Ok(n) = x.parse::<u64>() {
                Ok(Expression::FileSize(FileSize::from_bytes(n)))
            } else if let Some((num, unit)) = split_file_size(&x) {
                Ok(Expression::FileSize(FileSize::from_float(num, unit)))
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
    let trimmed = size_str.trim();

    // 找到最后一个数字字符（含小数点）的位置，作为数字/单位分界
    let split_pos = trimmed.rfind(|c: char| c.is_ascii_digit() || c == '.')?;
    let (number_part, unit_part) = trimmed.split_at(split_pos + 1);

    let unit = unit_part.trim().to_uppercase();

    // 天然支持 K/KB/KiB、大小写混用等多种格式
    let canonical_unit: &'static str = match unit.as_str() {
        "" | "B" => "B",
        "K" | "KB" | "KIB" => "K",
        "M" | "MB" | "MIB" => "M",
        "G" | "GB" | "GIB" => "G",
        "T" | "TB" | "TIB" => "T",
        "P" | "PB" | "PIB" => "P",
        _ => return None,
    };

    let number: f64 = number_part.trim().parse().ok()?;

    Some((number, canonical_unit))
}

// ===========serializers==============

// [NEW] 提取 TOML 字符串转义为独立函数
fn escape_toml_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0008}' => escaped.push_str("\\b"),
            '\u{000C}' => escaped.push_str("\\f"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

// [NEW] 提取 JSON 字符串转义为独立函数（供键和值共用）
fn escape_json_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0008}' => escaped.push_str("\\b"),
            '\u{000C}' => escaped.push_str("\\f"),
            _ if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn toml(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("toml", &args, 1, ctx)?;
    let expr = &args[0];
    let toml_str = expr_to_toml_string(expr, None);
    Ok(Expression::String(toml_str))
}

fn needs_quotes(key: &str) -> bool {
    if key.is_empty() {
        return true;
    }
    for ch in key.chars() {
        if !ch.is_alphanumeric() && ch != '_' && ch != '-' {
            return true;
        }
    }
    matches!(key, "true" | "false" | "null" | "inf" | "nan")
}

fn expr_to_toml_string(expr: &Expression, table_prefix: Option<&str>) -> String {
    match expr {
        Expression::None => "".to_string(),
        Expression::Boolean(b) => b.to_string(),
        Expression::Integer(i) => i.to_string(),
        Expression::Float(f) => {
            if f.is_infinite() {
                if f.is_sign_positive() {
                    "inf".to_string()
                } else {
                    "-inf".to_string()
                }
            } else if f.is_nan() {
                "nan".to_string()
            } else {
                f.to_string()
            }
        }
        Expression::DateTime(dt) => dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
        // [FIX] 使用完整转义，不再只转义双引号
        Expression::String(s) => format!("\"{}\"", escape_toml_string(s)),

        Expression::List(list) => {
            if list
                .iter()
                .all(|item| matches!(item, Expression::Map(_) | Expression::HMap(_)))
            {
                let mut output = Vec::new();
                for item in list.iter() {
                    output.push(format!("[[{}]]", table_prefix.unwrap_or("item")));
                    let table_content = expr_to_toml_string(item, table_prefix);
                    output.push(table_content);
                }
                output.join("\n")
            } else {
                let items: Vec<String> =
                    list.iter().map(|e| expr_to_toml_string(e, None)).collect();
                format!("[{}]", items.join(", "))
            }
        }

        Expression::BSet(set) => {
            let items: Vec<String> = set.iter().map(|e| expr_to_toml_string(e, None)).collect();
            format!("[{}]", items.join(", "))
        }

        Expression::Map(map) => {
            let mut output = Vec::new();
            let mut tables = BTreeMap::new();
            let mut simple_keys = BTreeMap::new();

            for (key, value) in map.as_ref() {
                if matches!(value, Expression::Map(_) | Expression::HMap(_)) {
                    tables.insert(key.clone(), value);
                } else {
                    simple_keys.insert(key.clone(), value);
                }
            }

            for (key, value) in &simple_keys {
                let formatted_key = if needs_quotes(key) {
                    format!("\"{}\"", escape_toml_string(key))
                } else {
                    key.clone()
                };
                output.push(format!(
                    "{} = {}",
                    formatted_key,
                    expr_to_toml_string(value, None)
                ));
            }

            for (table_name, table_expr) in &tables {
                let formatted_table_name = if needs_quotes(table_name) {
                    format!("\"{}\"", escape_toml_string(table_name))
                } else {
                    table_name.clone()
                };
                let full_table_name = match table_prefix {
                    Some(prefix) => format!("{prefix}.{formatted_table_name}"),
                    None => formatted_table_name,
                };
                output.push(format!("\n[{full_table_name}]"));
                let table_content = expr_to_toml_string(table_expr, Some(&full_table_name));
                for line in table_content.lines() {
                    output.push(line.to_string());
                }
            }

            output.join("\n")
        }

        // [FIX] HMap 分支现在与 Map 分支完全一致，使用 needs_quotes 和 escape_toml_string
        Expression::HMap(map) => {
            let mut output = Vec::new();
            let mut tables = BTreeMap::new();
            let mut simple_keys = BTreeMap::new();

            for (key, value) in map.as_ref() {
                if matches!(value, Expression::Map(_) | Expression::HMap(_)) {
                    tables.insert(key.clone(), value);
                } else {
                    simple_keys.insert(key.clone(), value);
                }
            }

            for (key, value) in &simple_keys {
                let formatted_key = if needs_quotes(key) {
                    format!("\"{}\"", escape_toml_string(key))
                } else {
                    key.clone()
                };
                output.push(format!(
                    "{} = {}",
                    formatted_key,
                    expr_to_toml_string(value, None)
                ));
            }

            for (table_name, table_expr) in &tables {
                let formatted_table_name = if needs_quotes(table_name) {
                    format!("\"{}\"", escape_toml_string(table_name))
                } else {
                    table_name.clone()
                };
                let full_table_name = match table_prefix {
                    Some(prefix) => format!("{prefix}.{formatted_table_name}"),
                    None => formatted_table_name,
                };
                output.push(format!("\n[{full_table_name}]"));
                let table_content = expr_to_toml_string(table_expr, Some(&full_table_name));
                for line in table_content.lines() {
                    output.push(line.to_string());
                }
            }

            output.join("\n")
        }

        Expression::Table(table_data) => {
            let mut output = Vec::new();
            for row in table_data.rows() {
                output.push(format!("[[{}]]", table_prefix.unwrap_or("item")));
                for (j, header) in table_data.headers().iter().enumerate() {
                    let value = row.get(j).cloned().unwrap_or(Expression::None);
                    output.push(format!(
                        "{} = {}",
                        header,
                        expr_to_toml_string(&value, None)
                    ));
                }
            }
            output.join("\n")
        }

        // [FIX] other 分支也使用转义
        other => format!("\"{}\"", escape_toml_string(&other.to_string())),
    }
}

pub fn json(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("json", &args, 1, ctx)?;
    let expr = &args[0];
    let json_str = expr_to_json_string(expr);
    Ok(Expression::String(json_str))
}

fn expr_to_json_string(expr: &Expression) -> String {
    match expr {
        Expression::None => "null".to_string(),
        Expression::Boolean(b) => b.to_string(),
        Expression::Integer(i) => i.to_string(),
        Expression::Float(f) => {
            if f.is_infinite() || f.is_nan() {
                "null".to_string()
            } else {
                f.to_string()
            }
        }
        Expression::DateTime(dt) => {
            format!("\"{}\"", dt.format("%Y-%m-%dT%H:%M:%S%.fZ"))
        }
        // 使用提取出的 escape_json_string 函数
        Expression::String(s) => format!("\"{}\"", escape_json_string(s)),

        Expression::List(list) => {
            let items: Vec<String> = list.iter().map(expr_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        Expression::BSet(set) => {
            let items: Vec<String> = set.iter().map(expr_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        // [FIX] Map 键也需要转义
        Expression::Map(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", escape_json_string(k), expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        // [FIX] HMap 键也需要转义
        Expression::HMap(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", escape_json_string(k), expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Expression::Table(table_data) => {
            let mut items = Vec::new();
            for row in table_data.rows() {
                let mut pairs = Vec::new();
                for (j, header) in table_data.headers().iter().enumerate() {
                    let value = row.get(j).cloned().unwrap_or(Expression::None);
                    pairs.push(format!(
                        "\"{}\":{}",
                        escape_json_string(header),
                        expr_to_json_string(&value)
                    ));
                }
                items.push(format!("{{{}}}", pairs.join(",")));
            }
            format!("[{}]", items.join(","))
        }
        // [FIX] other 分支也使用转义
        other => format!("\"{}\"", escape_json_string(&other.to_string())),
    }
}

pub fn csv(
    mut args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("csv", &args, 1, ctx)?;
    let expr = args.pop().unwrap();

    let ifs = env.get("IFS");
    // [FIX] 添加 !fs.is_empty() 防止空字符串 panic
    let delimiter = match (ifs_contains(IFS_CSV, env), &ifs) {
        (true, Some(Expression::String(fs))) if !fs.is_empty() && fs != "\n" => fs.as_bytes()[0],
        _ => b',',
    };

    // [NEW] 辅助闭包：将 csv crate 错误转换为 RuntimeError
    let csv_err = |msg: String| RuntimeError::common(msg.into(), ctx.clone(), 0);

    let result = match expr {
        Expression::List(rows) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(vec![]);

            let mut all_keys = BTreeMap::new();
            for row in rows.as_ref() {
                if let Expression::Map(map) = row {
                    for key in map.keys() {
                        all_keys.insert(key.clone(), ());
                    }
                }
            }
            let sorted_keys: Vec<_> = all_keys.keys().collect();

            // [FIX] unwrap → map_err + ?
            writer
                .write_record(&sorted_keys)
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;
            for row in rows.as_ref() {
                if let Expression::Map(map) = row {
                    let mut record = Vec::new();
                    for key in &sorted_keys {
                        let value = map.get(*key).map(expr_to_json_string).unwrap_or_default();
                        record.push(value);
                    }
                    writer
                        .write_record(&record)
                        .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;
                }
            }

            let inner = writer
                .into_inner()
                .map_err(|e| csv_err(format!("CSV flush failed: {e}")))?;
            String::from_utf8(inner).map_err(|e| csv_err(format!("CSV write failed: {e}")))
        }

        Expression::Map(map) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(vec![]);

            let sorted_keys: Vec<_> = map.keys().collect();
            writer
                .write_record(&sorted_keys)
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;

            let record: Vec<_> = sorted_keys
                .iter()
                .map(|k| expr_to_json_string(map.get(*k).unwrap()))
                .collect();
            writer
                .write_record(&record)
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;

            let inner = writer
                .into_inner()
                .map_err(|e| csv_err(format!("CSV flush failed: {e}")))?;
            String::from_utf8(inner).map_err(|e| csv_err(format!("CSV write failed: {e}")))
        }

        Expression::HMap(map) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(vec![]);

            let sorted_keys: Vec<_> = map.keys().collect();
            writer
                .write_record(&sorted_keys)
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;

            let record: Vec<_> = sorted_keys
                .iter()
                .map(|k| expr_to_json_string(map.get(*k).unwrap()))
                .collect();
            writer
                .write_record(&record)
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;

            let inner = writer
                .into_inner()
                .map_err(|e| csv_err(format!("CSV flush failed: {e}")))?;
            String::from_utf8(inner).map_err(|e| csv_err(format!("CSV write failed: {e}")))
        }

        Expression::String(ct) => Ok(ct),

        Expression::Table(table_data) => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(vec![]);

            writer
                .write_record(table_data.headers())
                .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;

            for row in table_data.rows() {
                let record: Vec<String> = row.iter().map(|v| v.to_string()).collect();
                writer
                    .write_record(&record)
                    .map_err(|e| csv_err(format!("CSV write failed: {e}")))?;
            }

            let inner = writer
                .into_inner()
                .map_err(|e| csv_err(format!("CSV flush failed: {e}")))?;
            String::from_utf8(inner).map_err(|e| csv_err(format!("CSV write failed: {e}")))
        }

        o => Ok(o.to_string()),
    };
    result.map(Expression::from)
    // Ok(Expression::String(result))
}

fn highlighted(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("highlighted", &args, 1, ctx)?;
    let script = get_string_ref(&args[0], ctx)?;
    if script.is_empty() {
        return Ok(Expression::None);
    }
    let hi = highlight_dark_theme(script);
    Ok(Expression::String(hi))
}

pub fn striped(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("striped", &args, 1, ctx)?;
    let p = get_string_ref(&args[0], ctx)?;
    Ok(strip_ansi_escapes(p).into())
}
