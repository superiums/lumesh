use crate::expression::cmd_excutor::expand_home;
use crate::modules::bin::get_string_arg;
use crate::{Environment, Int};
use crate::{Expression, LmError};
use common_macros::hash_map;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

// #[cfg(unix)]
// use std::os::unix::fs::MetadataExt;

use super::fs_ls::list_directory_wrapper;

pub fn get() -> Expression {
    let fs_module = hash_map! {
        // get
               String::from("dirs") => Expression::builtin("dirs", get_system_dirs, "get system directories", ""),
               String::from("ls") => Expression::builtin("ls", list_directory_wrapper, "list directory contents", "[path]"),
               String::from("glob") => Expression::builtin("glob", glob_pattern, "match files with pattern", "<pattern>"),
               String::from("tree") => Expression::builtin("tree", get_directory_tree, "get directory tree as nested map", "[path]"),
               String::from("canon") => Expression::builtin("canon", canonicalize_path, "canonicalize path", "<path>"),

               // modify
               String::from("mkdir") => Expression::builtin("mkdir", make_directory, "create directory", "<path>"),
               String::from("rmdir") => Expression::builtin("rmdir", remove_directory, "remove empty directory", "<path>"),
               String::from("mv") => Expression::builtin("mv", move_path_wrapper, "move path", "<source> <destination>"),
               String::from("cp") => Expression::builtin("cp", copy_path_wrapper, "copy path", "<source> <destination>"),
               String::from("rm") => Expression::builtin("rm", remove_path_wrapper, "remove path", "<path>"),

               // check
               String::from("exists") => Expression::builtin("exists", path_exists, "check if path exists", "<path>"),
               String::from("is_dir") => Expression::builtin("is_dir", is_directory, "check if path is directory", "<path>"),
               String::from("is_file") => Expression::builtin("is_file", is_file, "check if path is file", "<path>"),

               // read/write
               String::from("head") => Expression::builtin("head", read_file_head, "read first N lines of file", "[n] <file>"),
               String::from("tail") => Expression::builtin("tail", read_file_tail, "read last N lines of file", "[n] <file>"),
               String::from("read") => Expression::builtin("read", read_file, "read file contents", "<file>"),
               String::from("write") => Expression::builtin("write", write_file, "create/write to file", "<file> [content]"),
               String::from("append") => Expression::builtin("append", append_to_file, "append to file", "<file> <content>"),
               // assist
               String::from("base_name") => Expression::builtin("base_name", extract_filename, "extract base_name from path", "[split_ext?] <path>"),
               String::from("dir_name") => Expression::builtin("dir_name", extract_parent, "extract dir_name from path", "<path>"),
               String::from("join") => Expression::builtin("join", join_path, "join paths", "<path>..."),
    };
    Expression::from(fs_module)
}

// Helper functions

fn get_current_path() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn join_current_path(path: &str) -> PathBuf {
    get_current_path().join(path)
}

fn get_system_dirs(_args: &[Expression], _env: &mut Environment) -> Result<Expression, LmError> {
    let mut dir_tree = BTreeMap::<String, String>::new();

    if let Some(home_dir) = dirs::home_dir() {
        dir_tree.insert("home".into(), home_dir.to_string_lossy().into());
    }
    if let Some(config_dir) = dirs::config_dir() {
        dir_tree.insert("config".into(), config_dir.to_string_lossy().into());
    }
    if let Some(cache_dir) = dirs::cache_dir() {
        dir_tree.insert("cache".into(), cache_dir.to_string_lossy().into());
    }
    if let Some(data_dir) = dirs::data_dir() {
        dir_tree.insert("data".into(), data_dir.to_string_lossy().into());
    }
    if let Some(picture_dir) = dirs::picture_dir() {
        dir_tree.insert("pic".into(), picture_dir.to_string_lossy().into());
    }
    if let Some(desktop_dir) = dirs::desktop_dir() {
        dir_tree.insert("desk".into(), desktop_dir.to_string_lossy().into());
    }
    if let Some(document_dir) = dirs::document_dir() {
        dir_tree.insert("docs".into(), document_dir.to_string_lossy().into());
    }
    if let Some(download_dir) = dirs::download_dir() {
        dir_tree.insert("down".into(), download_dir.to_string_lossy().into());
    }

    dir_tree.insert(
        "current".into(),
        get_current_path().to_string_lossy().into(),
    );

    Ok(Expression::from(dir_tree))
}

fn get_directory_tree(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("tree", args, 1..=2)?;

    let mut cwd = get_current_path();
    let mut max_depth = None;

    match args.first().unwrap().eval(env)? {
        Expression::Integer(n) => {
            max_depth = Some(n);
            if let Some(path_expr) = args.get(1) {
                let path = path_expr.eval(env)?.to_string();
                cwd = cwd.join(path);
            }
        }
        Expression::String(path) | Expression::Symbol(path) => {
            cwd = cwd.join(path);
        }
        _ => (),
    }

    Ok(Expression::from(build_directory_tree(&cwd, max_depth)))
}

fn build_directory_tree(path: &Path, max_depth: Option<Int>) -> BTreeMap<String, Expression> {
    let mut tree = BTreeMap::new();

    tree.insert(
        ".".into(),
        Expression::String(path.to_string_lossy().to_string()),
    );
    if path.parent().is_some() {
        tree.insert(
            "..".into(),
            Expression::String(path.to_string_lossy().to_string()),
        );
    }

    if let Some(0) = max_depth {
        return tree;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let child_path = entry.path();
            if let Ok(name) = entry.file_name().into_string() {
                if child_path.is_dir() {
                    let new_depth = max_depth.map(|d| d - 1);
                    tree.insert(
                        name,
                        Expression::from(build_directory_tree(&child_path, new_depth)),
                    );
                } else {
                    tree.insert(name, Expression::String(path.to_string_lossy().to_string()));
                }
            }
        }
    }

    tree
}

fn read_file_portion(path: &Path, n: i64, from_start: bool) -> Result<String, LmError> {
    let contents = std::fs::read_to_string(path)?;
    // .map_err(|e| LmError::CustomError(format!("Could not read file: {}", path.display())))?;

    let mut lines: Vec<&str> = contents.lines().collect();
    if !from_start {
        lines.reverse();
    }

    let portion = lines
        .into_iter()
        .take(n.max(0) as usize)
        .collect::<Vec<&str>>()
        .join("\n");

    Ok(portion)
}

fn read_file_head(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("head", args, 1..=2)?;
    let p = args.last().unwrap().eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    let n = match args.len() {
        2 => match args[0].eval(env)? {
            Expression::Integer(n) => n,
            _ => {
                return Err(LmError::CustomError(
                    "First argument must be an integer".into(),
                ));
            }
        },
        1 => 10,
        _ => unreachable!(),
    };

    let result = read_file_portion(&path, n, true)?;
    Ok(Expression::String(result))
}

fn read_file_tail(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("head", args, 1..=2)?;
    let p = args.last().unwrap().eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    let n = match args.len() {
        2 => match args[0].eval(env)? {
            Expression::Integer(n) => n,
            _ => {
                return Err(LmError::CustomError(
                    "First argument must be an integer".into(),
                ));
            }
        },
        1 => 10,
        _ => unreachable!(),
    };

    let result = read_file_portion(&path, n, false)?;
    Ok(Expression::String(result))
}

fn canonicalize_path(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("canon", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    let canon_path = dunce::canonicalize(&path)?;
    //     .map_err(|_| {
    //     LmError::CustomError(format!("Could not canonicalize path: {}", path.display()))
    // })?;

    Ok(Expression::String(canon_path.to_string_lossy().into()))
}

fn make_directory(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("mkdir", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    std::fs::create_dir_all(&path)?;
    // .map_err(|_| {
    //     LmError::CustomError(format!("Could not create directory: {}", path.display()))
    // })?;

    Ok(Expression::None)
}

fn remove_directory(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("rmdir", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    std::fs::remove_dir(&path)?;
    //     .map_err(|_| {
    //     LmError::CustomError(format!(
    //         "Could not remove directory (is it empty?): {}",
    //         path.display()
    //     ))
    // })?;

    Ok(Expression::None)
}

fn move_path_wrapper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("mv", args, 2)?;
    let p = args[0].eval(env)?.to_string();
    let src = join_current_path(expand_home(p.as_str()).as_ref());
    let dst_str = args[1].eval(env)?.to_string();
    let dst = if dst_str.ends_with("/") {
        let mut dpath = join_current_path(&dst_str);
        dpath.push(src.file_name().unwrap_or(OsStr::new("")));
        dpath
    } else {
        join_current_path(&dst_str)
    };

    move_path(&src, &dst)?;
    Ok(Expression::None)
}

fn copy_path_wrapper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cp", args, 2)?;
    let p = args[0].eval(env)?.to_string();
    let src = join_current_path(expand_home(p.as_str()).as_ref());

    let dst_str = args[1].eval(env)?.to_string();
    let dst = if dst_str.ends_with("/") {
        let mut dpath = join_current_path(&dst_str);
        dpath.push(src.file_name().unwrap_or(OsStr::new("")));
        dpath
    } else {
        join_current_path(&dst_str)
    };

    copy_path(&src, &dst)?;
    Ok(Expression::None)
}

fn remove_path_wrapper(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("rm", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    remove_path(&path)?;
    Ok(Expression::None)
}

fn path_exists(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("exists", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    Ok(Expression::Boolean(path.exists()))
}

fn is_directory(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("isdir", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    Ok(Expression::Boolean(path.is_dir()))
}

fn is_file(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("isfile", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    Ok(Expression::Boolean(path.is_file()))
}

fn read_file(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("read", args, 1)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());

    // First try to read as text
    if let Ok(contents) = std::fs::read_to_string(&path) {
        return Ok(Expression::String(contents));
    }

    // Fall back to reading as bytes
    let bytes = std::fs::read(&path)?;
    // .map_err(|_| LmError::CustomError(format!("Could not read file: {}", path.display())))?;

    Ok(Expression::Bytes(bytes))
}

fn write_file(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("write", args, 1..=2)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());

    match args.len() {
        1 => {
            // 只有一个参数时，创建空白文件（如果不存在）
            if !path.exists() {
                std::fs::File::create(&path)?;
            }
        }
        2 => {
            // 两个参数时，正常写入内容
            let contents = args[1].eval(env)?;
            match contents {
                Expression::Bytes(bytes) => std::fs::write(&path, bytes),
                _ => std::fs::write(&path, contents.to_string()),
            }?;
        }
        _ => unreachable!(),
    }

    Ok(Expression::None)
}

fn append_to_file(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("append", args, 2)?;
    let p = args[0].eval(env)?.to_string();
    let path = join_current_path(expand_home(p.as_str()).as_ref());
    let contents = args[1].eval(env)?;

    let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
    // .map_err(|e| {
    //     LmError::CustomError(format!("Could not open file: {} - {}", path.display(), e))
    // })?;

    match contents {
        Expression::Bytes(bytes) => file.write_all(&bytes),
        _ => file.write_all(contents.to_string().as_bytes()),
    }?;
    // .map_err(|e| {
    //     LmError::CustomError(format!(
    //         "Could not append to file: {} - {}",
    //         path.display(),
    //         e
    //     ))
    // })?;

    Ok(Expression::None)
}

fn glob_pattern(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("glob", args, 1)?;

    let pattern = args[0].eval(env)?.to_string();
    let cwd = get_current_path();
    let mut results = Vec::new();

    for entry in glob::glob(&pattern)
        .map_err(|e| LmError::CustomError(format!("Invalid glob pattern: {pattern} - {e}")))?
    {
        let path = entry.map_err(|e| LmError::CustomError(format!("Glob error: {e}")))?;
        let display_path = path
            .strip_prefix(&cwd)
            .unwrap_or(&path)
            .display()
            .to_string();
        results.push(Expression::String(display_path));
    }

    Ok(Expression::from(results))
}

fn extract_filename(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("base_name", args, 1..=2)?;
    let path = args.last().unwrap().eval(env)?.to_string();
    let split_extension = args.len() > 1
        && match args[0] {
            Expression::Boolean(b) => b,
            _ => false,
        };

    let path = Path::new(path.as_str());

    // 获取文件名
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => String::from(""),
    };

    // 如果需要分割扩展名
    if split_extension {
        let parts = file_name.split_once('.');
        Ok(Expression::from(match parts {
            Some(ps) => vec![
                Expression::String(ps.0.to_string()),
                Expression::String(ps.1.to_string()),
            ],
            _ => vec![Expression::None, Expression::None],
        }))
    } else {
        Ok(Expression::String(file_name))
    }
}
fn extract_parent(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("dir_name", args, 1)?;
    let pathstr = get_string_arg(args[0].eval(env)?)?;

    let pathsep = if cfg!(windows) { "\\" } else { "/" };
    if pathstr.ends_with(pathsep) {
        return Ok(Expression::String(pathstr));
    }
    let path = Path::new(pathstr.as_str());

    // 获取文件名
    let dir_name = match path.parent() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => String::from(""),
    };

    Ok(Expression::String(dir_name))
}

fn join_path(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    // 检查至少有一个参数
    super::check_args_len("join", args, 1..)?;

    let mut final_path = PathBuf::new();

    for arg in args {
        let path_str = arg.eval(env)?.to_string();

        // 处理 ~ 符号
        let expanded_path = if path_str.starts_with('~') {
            let home_dir = dirs::home_dir().ok_or_else(|| {
                LmError::CustomError("Could not retrieve home directory.".to_string())
            })?;
            let start = if path_str.len() == 1 { 1 } else { 2 };
            home_dir.join(&path_str[start..]) // 去掉 ~ 并与主目录连接
        } else {
            Path::new(&path_str).to_path_buf()
        };
        final_path = final_path.join(expanded_path);
    }

    // 返回合并后的路径作为 Expression
    Ok(Expression::String(
        final_path.to_string_lossy().into_owned(),
    ))
}

// File operation implementations (unchanged from previous version)
fn move_path(src: &Path, dst: &Path) -> Result<(), LmError> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(LmError::CustomError(format!(
            "Destination exists: {}",
            dst.display()
        )));
    }
    std::fs::rename(src, dst).map_err(|e| {
        LmError::CustomError(format!(
            "Could not move {} to {}: {}",
            src.display(),
            dst.display(),
            e
        ))
    })
}

fn copy_path(src: &Path, dst: &Path) -> Result<(), LmError> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(LmError::CustomError(format!(
            "Destination exists: {}",
            dst.display()
        )));
    }

    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dst_path = dst.join(entry.file_name());
            copy_path(&entry.path(), &dst_path)?;
        }
    } else {
        std::fs::copy(src, dst)?;
        //     .map_err(|e| {
        //     LmError::CustomError(format!(
        //         "Could not copy {} to {}: {}",
        //         src.display(),
        //         dst.display(),
        //         e
        //     ))
        // })?;
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), LmError> {
    if path.is_dir() {
        Ok(std::fs::remove_dir_all(path)?)
    } else {
        Ok(std::fs::remove_file(path)?)
    }
    // .map_err(|e| LmError::CustomError(format!("Could not remove {}: {}", path.display(), e)))
}
