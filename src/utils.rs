use crate::{Environment, Expression, RuntimeError};
use std::{borrow::Cow, path::PathBuf};

// Helper functions

pub fn expand_home(path: &'_ str) -> Cow<'_, str> {
    if path.starts_with("~") {
        if let Some(home_dir) = dirs::home_dir() {
            return Cow::Owned(path.replace("~", home_dir.to_string_lossy().as_ref()));
        }
    }
    Cow::Borrowed(path)
}

pub fn get_std_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
pub fn get_current_path(env: &mut Environment) -> PathBuf {
    env.get("PWD").map_or(get_std_cwd(), |v| match v {
        Expression::String(s) => PathBuf::from(s),
        s => PathBuf::from(s.to_string()),
    })
}

pub fn join_current_path(path: &str, env: &mut Environment) -> PathBuf {
    get_current_path(env).join(path)
}
pub fn abs(path: &str, env: &mut Environment) -> PathBuf {
    if path.starts_with("./") || path == "." {
        return join_current_path(path, env);
    }
    PathBuf::from(expand_home(path).as_ref())
}
pub fn abs_check(path: &str, env: &mut Environment) -> Result<PathBuf, RuntimeError> {
    let abs = abs(path, env);
    if abs.exists() {
        return Ok(abs);
    }
    Err(RuntimeError::common(
        "File not found".into(),
        Expression::String(path.to_string()),
        0,
    ))
}
pub fn canon(p: &str, env: &mut Environment) -> Result<PathBuf, RuntimeError> {
    let path = abs(p, env);
    dunce::canonicalize(&path).map_err(|e| {
        RuntimeError::from_io_error(e, "canon".into(), Expression::String(p.to_string()), 0)
    })
}
