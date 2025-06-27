pub mod error_runtime;
pub mod error_syntax;
use crate::{Expression, Int};
use common_macros::b_tree_map;
use error_runtime::RuntimeError;
use error_syntax::SyntaxError;
use thiserror::Error;

// ============== 顶级错误类型 ==============

#[derive(Debug, Error)]
pub enum LmError {
    #[error(transparent)]
    Syntax(#[from] SyntaxError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    CustomError(String),
    #[error("type error, expected {expected}, found {sym}: {found}")]
    TypeError {
        expected: String,
        sym: String,
        found: String,
    },
}

impl LmError {
    pub const ERROR_CODE_RUNTIME_ERROR: Int = 100;
    pub const ERROR_CODE_SYNTAX_ERROR: Int = 101;
    pub const ERROR_CODE_IO_ERROR: Int = 102;
    pub const ERROR_CODE_CS_ERROR: Int = 103;
    pub const ERROR_CODE_TYPE_ERROR: Int = 104;
    pub fn codes() -> Expression {
        Expression::from(b_tree_map! {
            String::from("runtime_error") => Expression::Integer(Self::ERROR_CODE_RUNTIME_ERROR),
            String::from("syntax_error") => Expression::Integer(Self::ERROR_CODE_SYNTAX_ERROR),
            String::from("io_error") => Expression::Integer(Self::ERROR_CODE_IO_ERROR),
            String::from("custom_error") => Expression::Integer(Self::ERROR_CODE_CS_ERROR),
            String::from("type_error") => Expression::Integer(Self::ERROR_CODE_TYPE_ERROR),
        })
    }
    pub fn code(&self) -> Int {
        match self {
            Self::Syntax(_) => Self::ERROR_CODE_SYNTAX_ERROR,
            Self::Runtime(err) => err.code(),
            Self::Io(_) => Self::ERROR_CODE_IO_ERROR,
            Self::CustomError(_) => Self::ERROR_CODE_CS_ERROR,
            Self::TypeError { .. } => Self::ERROR_CODE_TYPE_ERROR,
        }
    }
}
