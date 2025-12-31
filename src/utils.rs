use crate::expression::cmd_excutor::expand_home;
use crate::{Expression, RuntimeError};
use std::path::PathBuf;

// Helper functions

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
