use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind,
    libs::{
        BuiltinInfo,
        helper::{
            check_args_len, check_exact_args_len, get_exact_string_arg, get_integer_arg,
            get_string_arg, get_string_args,
        },
        lazy_module::LazyModule,
    },
    reg_info, reg_lazy,
    utils::expand_home,
};

use crate::utils::{self, get_current_path, join_current_path};
use std::io::Write;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};
use std::{collections::HashMap, ffi::OsStr};
// use super::fs_ls::list_directory_wrapper;
use super::fs_ls::ls;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        dirs, ls, glob, tree, abs, canon,
        // modify
        mkdir, rmdir, mv, cp, rm,
        // check
        exists, is_dir, is_file,
        // read/write
        head, tail, read, write, append,
        // assist
        base_name, dir_name, join,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        dirs => "get system directories", ""
        ls => "list directory contents", "[path]"
        glob => "match files with pattern", "<pattern>"
        tree => "get directory tree as nested map", "[path]"
        abs => "absolute path", "<path>"
        canon => "canonicalize path", "<path>"

        // modify
        mkdir => "create directory", "<path>"
        rmdir => "remove empty directory", "<path>"
        mv => "move path", "<source> <destination>"
        cp => "copy path", "<source> <destination>"
        rm => "remove path", "<path>"

        // check
        exists => "check if path exists", "<path>"
        is_dir => "check if path is directory", "<path>"
        is_file => "check if path is file", "<path>"

        // read/write
        head => "read first N lines of file", "[n] <file>"
        tail => "read last N lines of file", "[n] <file>"
        read => "read file contents", "<file>"
        write => "create/write to file", "<file> [content]"
        append => "append to file", "<file> <content>"
        // assist
        base_name => "extract base_name from path", "[split_ext?] <path>"
        dir_name => "extract dir_name from path", "<path>"
        join => "join paths", "<path>..."
    })
}
// Helper Functions
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
// File operation implementations (unchanged from previous version)
fn move_path(src: &Path, dst: &Path, ctx: &Expression) -> Result<(), RuntimeError> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(RuntimeError::common(
            format!("Destination exists: {}", dst.display()).into(),
            ctx.clone(),
            0,
        ));
    }
    std::fs::rename(src, dst)
        .map_err(|e| RuntimeError::from_io_error(e, "move".into(), ctx.clone(), 0))
}

fn copy_path(src: &Path, dst: &Path, ctx: &Expression) -> Result<(), RuntimeError> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(RuntimeError::common(
            format!("Destination exists: {}", dst.display()).into(),
            ctx.clone(),
            0,
        ));
    }

    if src.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| RuntimeError::from_io_error(e, "create dirs".into(), ctx.clone(), 0))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| RuntimeError::from_io_error(e, "read dir".into(), ctx.clone(), 0))?
        {
            let entry = entry
                .map_err(|e| RuntimeError::from_io_error(e, "read entry".into(), ctx.clone(), 0))?;
            let dst_path = dst.join(entry.file_name());
            copy_path(&entry.path(), &dst_path, ctx)?;
        }
    } else {
        std::fs::copy(src, dst)
            .map_err(|e| RuntimeError::from_io_error(e, "copy".into(), ctx.clone(), 0))?;
    }
    return Ok(());
}

// System Directory Functions
fn dirs(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
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

    Ok(Expression::from(dir_tree))
}
// Directory Tree Functions
fn tree(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("tree", args, 1..=2, ctx)?;

    let mut cwd = get_current_path();
    let mut max_depth = None;

    match args[0].eval(env)? {
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
// File Reading Functions
fn head(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("head", args, 1..=2, ctx)?;
    let p = args.last().unwrap().eval(env)?.to_string();
    let path = utils::canon(&p)?;
    let n = match args.len() {
        2 => match args[0].eval(env)? {
            Expression::Integer(n) => n,
            _ => {
                return Err(RuntimeError::common(
                    "First argument must be an integer".into(),
                    ctx.clone(),
                    0,
                ));
            }
        },
        1 => 10,
        _ => unreachable!(),
    };

    let result = read_file_portion(&path, n, true)?;
    Ok(Expression::String(result))
}

fn tail(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("tail", args, 1..=2, ctx)?;
    let p = args.last().unwrap().eval(env)?.to_string();
    let path = utils::canon(&p)?;
    let n = match args.len() {
        2 => match args[0].eval(env)? {
            Expression::Integer(n) => n,
            _ => {
                return Err(RuntimeError::common(
                    "First argument must be an integer".into(),
                    ctx.clone(),
                    0,
                ));
            }
        },
        1 => 10,
        _ => unreachable!(),
    };

    let result = read_file_portion(&path, n, false)?;
    Ok(Expression::String(result))
}

fn read_file_portion(path: &Path, n: i64, from_start: bool) -> Result<String, RuntimeError> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        RuntimeError::from_io_error(e, "read file portion".into(), Expression::None, 0)
    })?;

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
// Path Operations
fn canon(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("canon", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let canon_path = utils::canon(&p)?;
    Ok(Expression::String(canon_path.to_string_lossy().into()))
}

fn abs(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("abs", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let abs_path = utils::abs(&p);
    Ok(Expression::String(abs_path.to_string_lossy().into()))
}
// Directory Operations
fn mkdir(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("mkdir", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    std::fs::create_dir_all(&path).map_err(|e| {
        RuntimeError::from_io_error(e, "create directory".into(), args[0].clone(), 0)
    })?;
    Ok(Expression::None)
}

fn rmdir(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("rmdir", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    std::fs::remove_dir(&path).map_err(|e| {
        RuntimeError::from_io_error(e, "remove directory".into(), args[0].clone(), 0)
    })?;
    Ok(Expression::None)
}
// File Operations
fn mv(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("mv", args, 2, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let src = utils::abs(&p);
    let dst_str = args[1].eval(env)?.to_string();
    let dst = if is_a_dir(&dst_str) {
        let mut dpath = join_current_path(&dst_str);
        dpath.push(src.file_name().unwrap_or(OsStr::new("")));
        dpath
    } else {
        join_current_path(&dst_str)
    };

    move_path(&src, &dst, ctx)?;
    Ok(Expression::None)
}

fn cp(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cp", args, 2, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let src = utils::abs(&p);

    let dst_str = args[1].eval(env)?.to_string();
    let dst = if is_a_dir(&dst_str) {
        let mut dpath = join_current_path(&dst_str);
        dpath.push(src.file_name().unwrap_or(OsStr::new("")));
        dpath
    } else {
        join_current_path(&dst_str)
    };

    copy_path(&src, &dst, ctx)?;
    Ok(Expression::None)
}

fn rm(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("rm", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    remove_path(&path)?;
    Ok(Expression::None)
}

fn remove_path(path: &Path) -> Result<(), RuntimeError> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| {
            RuntimeError::from_io_error(e, "remove directory all".into(), Expression::None, 0)
        })?;
    } else {
        std::fs::remove_file(path).map_err(|e| {
            RuntimeError::from_io_error(e, "remove file".into(), Expression::None, 0)
        })?;
    }
    Ok(())
}
// Path Check Functions
fn exists(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exists", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    Ok(Expression::Boolean(path.exists()))
}

fn is_dir(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_dir", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    Ok(Expression::Boolean(path.is_dir()))
}

fn is_file(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_file", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    Ok(Expression::Boolean(path.is_file()))
}
// File Content Operations
fn read(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("read", args, 1, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::canon(&p)?;

    // First try to read as text
    if let Ok(contents) = std::fs::read_to_string(&path) {
        return Ok(Expression::String(contents));
    }

    // Fall back to reading as bytes
    let bytes = std::fs::read(&path)
        .map_err(|e| RuntimeError::from_io_error(e, "read file".into(), args[0].clone(), 0))?;
    Ok(Expression::Bytes(bytes))
}

fn write(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("write", args, 1..=2, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);

    // 只有一个参数时，创建空白文件（如果不存在）
    if !path.exists() {
        std::fs::File::create(&path).map_err(|e| {
            RuntimeError::from_io_error(e, "create file".into(), args[0].clone(), 0)
        })?;
    }

    // 两个参数时，正常写入内容
    if args.len() == 2 {
        let contents = args[1].eval(env)?;
        match contents {
            Expression::Bytes(bytes) => std::fs::write(&path, bytes),
            _ => std::fs::write(&path, contents.to_string()),
        }
        .map_err(|e| RuntimeError::from_io_error(e, "write file".into(), args[0].clone(), 0))?;
    }

    Ok(Expression::None)
}

fn append(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("append", args, 2, ctx)?;
    let p = args[0].eval(env)?.to_string();
    let path = utils::abs(&p);
    let contents = args[1].eval(env)?;

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| RuntimeError::from_io_error(e, "open file".into(), args[0].clone(), 0))?;

    match contents {
        Expression::Bytes(bytes) => file.write_all(&bytes),
        _ => file.write_all(contents.to_string().as_bytes()),
    }
    .map_err(|e| RuntimeError::from_io_error(e, "write file".into(), args[0].clone(), 0))?;

    Ok(Expression::None)
}
// Pattern Matching
fn glob(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("glob", args, 1, ctx)?;

    let pattern = args[0].eval(env)?.to_string();
    let cwd = get_current_path();
    let mut results = Vec::new();

    for entry in glob::glob(&pattern).map_err(|e| {
        RuntimeError::common(
            format!("Invalid glob pattern: {pattern} - {e}").into(),
            ctx.clone(),
            0,
        )
    })? {
        let path = entry
            .map_err(|e| RuntimeError::common(format!("Glob error: {e}").into(), ctx.clone(), 0))?;
        let display_path = path
            .strip_prefix(&cwd)
            .unwrap_or(&path)
            .display()
            .to_string();
        results.push(Expression::String(display_path));
    }

    Ok(Expression::from(results))
}
// Path Extraction
fn base_name(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("base_name", args, 1..=2, ctx)?;
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
fn dir_name(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("dir_name", args, 1, ctx)?;
    let pathstr = get_string_arg(args[0].eval(env)?, ctx)?;

    if is_a_dir(&pathstr) {
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

fn join(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // 检查至少有一个参数
    check_args_len("join", args, 1.., ctx)?;

    let mut final_path = PathBuf::new();

    for arg in args {
        let path_str = arg.eval(env)?.to_string();
        final_path = final_path.join(path_str);
    }
    let p = expand_home(final_path.to_str().unwrap_or("."));
    // 返回合并后的路径作为 Expression
    Ok(Expression::String(p.into()))
}
fn is_a_dir(path: &str) -> bool {
    let path = utils::abs(path);
    path.is_dir()
}
