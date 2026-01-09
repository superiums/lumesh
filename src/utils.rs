use crate::{Expression, RuntimeError};
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

pub fn get_current_path() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn join_current_path(path: &str) -> PathBuf {
    get_current_path().join(path)
}
pub fn abs(path: &str) -> PathBuf {
    get_current_path().join(expand_home(path).as_ref())
}
pub fn canon(p: &str) -> Result<PathBuf, RuntimeError> {
    let path = abs(p);
    dunce::canonicalize(&path).map_err(|e| {
        RuntimeError::from_io_error(e, "canon".into(), Expression::String(p.to_string()), 0)
    })
}
