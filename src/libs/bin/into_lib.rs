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
        // 时间解析（time库）
        time,
        table,
        // 数据格式序列化
        toml, json, csv,
        highlighted, striped,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
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
        table => "convert third-party command output to a table", "<command_output> [regex|headers...]"

        // 数据格式序列化
        toml => "parse lumesh expression into TOML", "<expr>"
        json => "parse lumesh expression into JSON", "<expr>"
        csv => "parse lumesh expression into CSV", "<expr>"

        highlighted =>   "highlight script str with ANSI", "<script_string>"
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
        // 转换 List<Map> 到 TableData
        Expression::List(list) => {
            return Ok(Expression::Table(convert_list_map_to_table(&list)));
        }
        // 如果已经是表格格式
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

    // ---convert string---
    // lines
    let mut lines: Vec<&str> = data.lines().collect();
    if lines.is_empty() {
        return Ok(Expression::None);
    } else {
        // 检测已经是列表格式的：首行首尾都是相同的符号
        let mut c = lines.first().unwrap().chars();
        if let Some(first) = c.next() {
            if first.is_ascii_punctuation() && Some(first) == c.last() {
                return Ok(Expression::String(data));
            }
        }
    }

    // headers
    let (headers, splitter): (Vec<String>, Option<Regex>) = match opts {
        s if s.is_empty() => (Vec::new(), None),
        s if s.len() == 1 => match s.first().unwrap() {
            Expression::List(list) => (list.as_ref().iter().map(|x| x.to_string()).collect(), None),
            Expression::BSet(list) => (list.as_ref().iter().map(|x| x.to_string()).collect(), None),
            Expression::Regex(r) => (Vec::new(), Some(r.regex.clone())),
            _ => (s.iter().map(|x| x.to_string()).collect(), None),
        },
        s => (s.iter().map(|x| x.to_string()).collect(), None),
    };

    // Filter short tip lines
    if lines.len() > 2 {
        let first_line_cols = split_line(&lines[0], &splitter);
        let second_line_cols = split_line(&lines[1], &splitter);

        if first_line_cols.len() < second_line_cols.len() {
            lines.remove(0);
        }

        let last_line_cols = split_line(lines.last().unwrap(), &splitter);
        let second_last_line_cols = split_line(&lines[lines.len() - 2], &splitter);

        if last_line_cols.len() < second_last_line_cols.len() {
            lines.pop();
        }
    }

    // Try to detect headers
    let (data_lines, detected_headers) = if headers.is_empty() {
        let maybe_header = lines[0];
        let first_line_cols = split_line(&maybe_header, &splitter);
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
            // Use column numbers
            let cols = first_line_cols
                .iter()
                .enumerate()
                .map(|(i, _)| format!("C{i}"))
                .collect();
            (lines, cols)
        }
    } else {
        // Use provided headers
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

// Helper function to split line using regex or whitespace
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("toml", &args, 1, ctx)?;
    let expr = &args[0];
    let toml_str = expr_to_toml_string(expr, None);
    Ok(Expression::String(toml_str))
}

// 添加辅助函数判断键是否需要引号
fn needs_quotes(key: &str) -> bool {
    // 空字符串
    if key.is_empty() {
        return true;
    }

    // 检查是否只允许的字符
    for ch in key.chars() {
        if !ch.is_alphanumeric() && ch != '_' && ch != '-' {
            return true;
        }
    }

    // 检查是否是保留字
    matches!(key, "true" | "false" | "null" | "inf" | "nan")
}

// 递归序列化函数（新增表名前缀参数）
fn expr_to_toml_string(expr: &Expression, table_prefix: Option<&str>) -> String {
    match expr {
        // 基本类型处理
        Expression::None => "".to_string(),
        Expression::Boolean(b) => b.to_string(),
        Expression::Integer(i) => i.to_string(),
        Expression::Float(f) => {
            // 处理特殊值
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
        // never include space
        Expression::DateTime(dt) => dt.format("%Y-%m-%dT%H:%M:%S").to_string(),

        // 字符串处理（禁用Unicode转义）
        Expression::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),

        // 数组处理
        Expression::List(list) => {
            // 检查是否是数组 of tables
            if list
                .iter()
                .all(|item| matches!(item, Expression::Map(_) | Expression::HMap(_)))
            {
                // 格式化为数组 of tables
                let mut output = Vec::new();
                for item in list.iter() {
                    output.push(format!("[[{}]]", table_prefix.unwrap_or("item")));
                    let table_content = expr_to_toml_string(item, table_prefix);
                    output.push(table_content);
                }
                output.join("\n")
            } else {
                // 普通数组
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

            // 分离简单键和嵌套表
            for (key, value) in map.as_ref() {
                if let Expression::Map(_) = value {
                    tables.insert(key.clone(), value);
                } else if let Expression::HMap(_) = value {
                    tables.insert(key.clone(), value);
                } else {
                    simple_keys.insert(key.clone(), value);
                }
            }

            // 处理当前层简单键值对
            for (key, value) in &simple_keys {
                let formatted_key = if needs_quotes(key) {
                    format!("\"{}\"", key.replace("\"", "\\\""))
                } else {
                    key.clone()
                };
                let line = format!("{} = {}", formatted_key, expr_to_toml_string(value, None));
                output.push(line);
            }

            // 处理嵌套表
            for (table_name, table_expr) in &tables {
                let formatted_table_name = if needs_quotes(table_name) {
                    format!("\"{}\"", table_name.replace("\"", "\\\""))
                } else {
                    table_name.clone()
                };

                let full_table_name = match table_prefix {
                    Some(prefix) => format!("{prefix}.{}", formatted_table_name),
                    None => formatted_table_name,
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

        Expression::HMap(map) => {
            let mut output = Vec::new();
            let mut tables = BTreeMap::new();
            let mut simple_keys = BTreeMap::new();

            // 分离简单键和嵌套表
            for (key, value) in map.as_ref() {
                if let Expression::Map(_) = value {
                    tables.insert(key.clone(), value);
                } else if let Expression::HMap(_) = value {
                    tables.insert(key.clone(), value);
                } else {
                    simple_keys.insert(key.clone(), value);
                }
            }

            // 处理当前层简单键值对
            for (key, value) in &simple_keys {
                let formatted_key = if needs_quotes(key) {
                    format!("\"{}\"", key.replace("\"", "\\\""))
                } else {
                    key.clone()
                };
                let line = format!("{} = {}", formatted_key, expr_to_toml_string(value, None));
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
        // Expression::Table(table_data) => {
        //     // Convert table to list of maps, then serialize
        //     let list_map = table_data.to_list_map();
        //     expr_to_toml_string(&list_map, table_prefix)
        // }
        Expression::Table(table_data) => {
            // 直接转换为 TOML 数组 of tables
            let mut output = Vec::new();

            for row in table_data.rows() {
                output.push(format!("[[{}]]", table_prefix.unwrap_or("item")));

                for (j, header) in table_data.headers().iter().enumerate() {
                    let value = row.get(j).cloned().unwrap_or(Expression::None);
                    let value_str = expr_to_toml_string(&value, None);
                    output.push(format!("{} = {}", header, value_str));
                }
            }

            output.join("\n")
        }
        // 其他类型保持原样
        other => format!("\"{}\"", other.to_string()),
    }
}

// Expression to JSON Conversion (优化版)
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
            // 处理特殊值
            if f.is_infinite() {
                if f.is_sign_positive() {
                    "null".to_string() // JSON 不支持 inf，用 null 代替
                } else {
                    "null".to_string()
                }
            } else if f.is_nan() {
                "null".to_string() // JSON 不支持 nan，用 null 代替
            } else {
                f.to_string()
            }
        }
        Expression::DateTime(dt) => {
            // JSON 没有日期类型，转换为 ISO 8601 字符串
            format!("\"{}\"", dt.format("%Y-%m-%dT%H:%M:%S%.fZ"))
        }
        Expression::String(s) => {
            // 需要正确转义特殊字符
            let mut escaped = String::new();
            for ch in s.chars() {
                match ch {
                    '"' => escaped.push_str("\\\""),
                    '\\' => escaped.push_str("\\\\"),
                    '\n' => escaped.push_str("\\n"),
                    '\r' => escaped.push_str("\\r"),
                    '\t' => escaped.push_str("\\t"),
                    '\u{0008}' => escaped.push_str("\\b"),
                    '\u{000C}' => escaped.push_str("\\f"),
                    _ if ch.is_control() => {
                        escaped.push_str(&format!("\\u{:04x}", ch as u32));
                    }
                    _ => escaped.push(ch),
                }
            }
            format!("\"{}\"", escaped)
        }
        Expression::List(list) => {
            let items: Vec<String> = list.iter().map(expr_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        Expression::BSet(set) => {
            let items: Vec<String> = set.iter().map(expr_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        Expression::Map(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Expression::HMap(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, expr_to_json_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        // Expression::Table(table_data) => {
        //     // Convert table to list of maps, then serialize
        //     let list_map = table_data.to_list_map();
        //     expr_to_json_string(&list_map)
        // }
        Expression::Table(table_data) => {
            // 直接转换为 JSON 数组 of objects
            let mut items = Vec::new();

            for row in table_data.rows() {
                let mut pairs = Vec::new();
                for (j, header) in table_data.headers().iter().enumerate() {
                    let value = row.get(j).cloned().unwrap_or(Expression::None);
                    let value_str = expr_to_json_string(&value);
                    pairs.push(format!("\"{}\":{}", header, value_str));
                }
                items.push(format!("{{{}}}", pairs.join(",")));
            }

            format!("[{}]", items.join(","))
        }
        other => format!("\"{}\"", other.to_string()),
    }
}

// Expression to CSV
pub fn csv(
    mut args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("csv", &args, 1, ctx)?;
    let expr = args.pop().unwrap();

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
        Expression::HMap(map) => {
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
        Expression::String(ct) => ct,
        Expression::Table(table_data) => {
            // 新的 Table 类型处理
            let mut writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(vec![]);

            // 写入标题行
            writer.write_record(table_data.headers()).unwrap();

            // 写入数据行
            for row in table_data.rows() {
                let record: Vec<String> = row.iter().map(|v| v.to_string()).collect();
                writer.write_record(&record).unwrap();
            }

            String::from_utf8(writer.into_inner().unwrap()).unwrap()
        }
        o => o.to_string(),
    };

    Ok(Expression::String(result))
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

// 单参数函数（字符串作为最后一个参数）
pub fn striped(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("striped", &args, 1, ctx)?;
    let p = get_string_ref(&args[0], ctx)?;

    Ok(strip_ansi_escapes(&p).into())
}
