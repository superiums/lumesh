#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDateTime};

use crate::expression::FileSize;
use crate::expression::table::TableData;
use crate::utils::abs;
use crate::{Environment, Expression, RuntimeError};

#[derive(Default)]
pub struct LsOptions {
    pub detailed: bool,
    pub show_hidden: bool,
    pub follow_links: bool,
    // pub human_readable: bool,
    pub unix_time: bool,
    pub size_in_kb: bool,
    pub show_user: bool,
    pub show_mode: bool,
    pub show_path: bool,
}

pub fn parse_ls_args(
    args: Vec<Expression>,
    env: &mut Environment,
) -> Result<(PathBuf, LsOptions), RuntimeError> {
    let mut options = LsOptions::default();
    let mut path = PathBuf::from(".");
    // dbg!(args);
    for arg in args {
        if let Expression::Symbol(s) | Expression::String(s) = arg {
            match s.as_str() {
                "-l" => options.detailed = true,
                "-a" => options.show_hidden = true,
                "-L" => options.follow_links = true,
                // "-h" => options.human_readable = true,
                "-U" => options.unix_time = true,
                "-k" => options.size_in_kb = true,
                "-u" => options.show_user = true,
                "-m" => options.show_mode = true,
                "-p" => options.show_path = true,
                arg if !arg.starts_with('-') => path = abs(arg, env),
                _ => continue,
            }
        }
    }
    // dbg!(&path);
    Ok((path, options))
}

pub fn get_file_expression(
    entry: &std::fs::DirEntry,
    options: &LsOptions,
    base_path: Option<&Path>,
    ctx: &Expression,
) -> Result<Vec<Expression>, RuntimeError> {
    let p = entry.path();
    let metadata = if options.follow_links {
        entry.metadata().map_err(|e| {
            RuntimeError::from_io_error(e, "read file meta".into(), Expression::None, 0)
        })?
    } else {
        p.symlink_metadata().map_err(|e| {
            RuntimeError::from_io_error(e, "read symlink".into(), Expression::None, 0)
        })?
    };

    let name = entry.file_name().to_string_lossy().into_owned();
    let mut row = Vec::new();

    // 基础字段：name (总是第一列)
    row.push(Expression::String(name.clone()));

    if options.detailed {
        // 惰性检测字段：type
        let file_type = detect_file_type(&metadata);
        row.push(Expression::String(file_type.to_string()));

        // 动态计算大小表达式
        let size_expr = if options.size_in_kb {
            Expression::Integer(metadata.len().div_ceil(1024) as i64)
        } else {
            Expression::FileSize(FileSize::from_bytes(metadata.len()))
        };
        row.push(size_expr);

        // 时间表达式
        let modified = metadata.modified().map_err(|e| {
            RuntimeError::from_io_error(e, "read mtime".into(), Expression::None, 0)
        })?;
        let time_expr = if options.unix_time {
            Expression::Integer(system_time_to_unix_duration(modified, ctx)?.as_secs() as i64)
        } else {
            Expression::DateTime(system_time_to_naive_datetime(modified, ctx)?)
        };
        row.push(time_expr);

        // 符号链接目标（惰性检测）
        #[cfg(unix)]
        if options.follow_links {
            if file_type == "symlink" {
                if let Ok(target) = std::fs::read_link(entry.path()) {
                    row.push(Expression::String(target.to_string_lossy().into_owned()));
                } else {
                    row.push(Expression::None);
                }
            } else {
                row.push(Expression::None);
            }
        }
    }

    // Unix特有字段
    #[cfg(unix)]
    {
        if options.show_user {
            row.push(Expression::Integer(metadata.uid() as i64));
        }
        if options.detailed || options.show_mode {
            let mode = metadata.permissions().mode() & 0o777;
            row.push(Expression::Integer(mode as i64));
        }
    }

    // 可选路径字段
    if options.show_path {
        if let Some(p) = base_path {
            let full_path = p.join(&name);
            row.push(Expression::String(full_path.to_string_lossy().into_owned()));
        } else {
            row.push(Expression::None);
        }
    }

    Ok(row)
}

pub fn ls(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let (full_path, options) = parse_ls_args(args, env)?;

    if !full_path.exists() {
        return Err(RuntimeError::common(
            format!("Path does not exist: {}", full_path.display()).into(),
            ctx.clone(),
            0,
        ));
    }

    // 构建表头
    let mut headers = vec!["name".to_string()];

    if options.detailed {
        headers.extend_from_slice(&[
            "type".to_string(),
            "size".to_string(),
            "modified".to_string(),
        ]);

        #[cfg(unix)]
        if options.follow_links {
            headers.push("target".to_string());
        }
    }

    #[cfg(unix)]
    {
        if options.show_user {
            headers.push("user".to_string());
        }
        if options.detailed || options.show_mode {
            headers.push("mode".to_string());
        }
    }

    if options.show_path {
        headers.push("path".to_string());
    }

    // 收集行数据
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(&full_path)
        .map_err(|e| RuntimeError::from_io_error(e, "read dir".into(), Expression::None, 0))?
    {
        let entry = entry.map_err(|e| {
            RuntimeError::from_io_error(e, "read entry".into(), Expression::None, 0)
        })?;
        let file_name = entry.file_name();

        if !options.show_hidden && file_name.to_string_lossy().starts_with('.') {
            continue;
        }

        let row = get_file_expression(&entry, &options, Some(&full_path), ctx)?;
        rows.push(row);
    }

    // 创建 TableData 并包装为 Expression
    let table_data = TableData::new(headers, rows);
    Ok(Expression::Table(table_data))
}

#[cfg(unix)]
fn detect_file_type(metadata: &std::fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        "directory"
    } else if file_type.is_file() {
        "file"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_block_device() {
        "block_device"
    } else if file_type.is_char_device() {
        "char_device"
    } else if file_type.is_fifo() {
        "fifo"
    } else {
        "unknown"
    }
}

#[cfg(windows)]
fn detect_file_type(metadata: &std::fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        "directory"
    } else if file_type.is_file() {
        "file"
    } else if file_type.is_symlink() {
        "symlink"
    } else {
        "unknown"
    }
}

// 辅助函数：将 SystemTime 转换为 UNIX 时间戳的 Duration
fn system_time_to_unix_duration(
    st: SystemTime,
    ctx: &Expression,
) -> Result<std::time::Duration, RuntimeError> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| RuntimeError::common("SystemTime before UNIX EPOCH".into(), ctx.clone(), 0))
}

// 辅助函数：将 SystemTime 转换为 NaiveDateTime
fn system_time_to_naive_datetime(
    st: SystemTime,
    ctx: &Expression,
) -> Result<NaiveDateTime, RuntimeError> {
    let duration = system_time_to_unix_duration(st, ctx)?;
    Ok(
        DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
            .unwrap_or_default()
            .naive_local(), // NaiveDateTime::from_timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
                            // .unwrap_or_default(),
    ) // 提供默认值以防转换失败
}

// fn format_system_time(time: SystemTime) -> String {
//     let datetime: chrono::DateTime<chrono::Local> =
//         (UNIX_EPOCH + time.duration_since(UNIX_EPOCH).unwrap()).into();
//     datetime.format("%Y-%m-%d %H:%M:%S").to_string()
// }

// fn human_readable_size(size: u64) -> String {
//     const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
//     let mut size = size as f64;
//     let mut unit_idx = 0;

//     while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
//         size /= 1024.0;
//         unit_idx += 1;
//     }

//     if unit_idx == 0 {
//         format!("{}", size)
//     } else {
//         format!("{:.1}{}", size, UNITS[unit_idx])
//     }
// }
