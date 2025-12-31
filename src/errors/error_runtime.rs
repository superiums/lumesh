use std::borrow::Cow;

// ============== 运行时错误部分 ==============
use crate::{Expression, Int};
use common_macros::b_tree_map;
use thiserror::Error;

#[derive(Debug)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub context: Expression,
    pub depth: usize,
}
#[derive(Debug, Error)]
pub enum RuntimeErrorKind {
    #[error("type `{0}` is not appliable: {1:?}")]
    CannotApply(String, Expression),
    #[error("symbol `{0}` not defined")]
    SymbolNotDefined(String),
    #[error("command `{0}` failed with args:\n  {1:?}")]
    CommandFailed(String, Vec<Expression>),
    #[error("command `{0}` failed:\n  {1}")]
    CommandFailed2(String, String),
    #[error("attempted to iterate over non-list `{0:?}`")]
    ForNonList(Expression),
    #[error("recursion depth exceeded while evaluating `{0:?}`")]
    RecursionDepth(Expression),
    #[error("permission denied while spawn `{0}`")]
    PermissionDenied(String),
    #[error("program `{0}` not found")]
    ProgramNotFound(String),
    #[error("{0}")]
    CustomError(Cow<'static, str>),
    #[error("redeclaration of `{0}`")]
    Redeclaration(String),
    #[error("undeclared variable: `{0}`")]
    UndeclaredVariable(String),
    #[error("no matching branch while evaluating `{0}`")]
    NoMatchingBranch(String),
    #[error("too many arguments for function `{name}`: max {max}, found {received}")]
    TooManyArguments {
        name: String,
        max: usize,
        received: usize,
    },
    #[error("arguments mismatch for function `{name}`: expected {expected}, found {received}")]
    ArgumentMismatch {
        name: String,
        expected: usize,
        received: usize,
    },
    #[error("invalid default value `{2}` for argument `{1}` in function `{0}`")]
    InvalidDefaultValue(String, String, Expression),
    #[error("invalid operator `{0}`")]
    InvalidOperator(String),
    #[error("index {index} out of bounds (length {len})")]
    IndexOutOfBounds { index: Int, len: usize },
    #[error("key `{0}` not found in map")]
    KeyNotFound(String),
    #[error("method `{0}` not found in module `{1}`")]
    MethodNotFound(Cow<'static, str>, Cow<'static, str>),
    // #[error("module `{0}` not found")]
    // ModuleNotFound(Cow<'static, str>),
    #[error("no module defined for `{0}`:{1}")]
    NoModuleDefined(String, Cow<'static, str>),
    #[error("not a callable function: `{0}`")]
    NotAFunction(String),
    #[error("type error, expected `{expected}`, found `{found}`:\n  {sym}")]
    TypeError {
        expected: String,
        sym: String,
        found: String,
    },
    #[error("illegal return outside function")]
    EarlyReturn(Expression),
    #[error("illegal break outside loop")]
    EarlyBreak(Expression),
    #[error("overflowed when: `{0}`")]
    Overflow(String),
    #[error("wildcard not matched: `{0}`")]
    WildcardNotMatched(String),
    #[error("builtin func `{0}` failed:\n  {1}")]
    BuiltinFailed(String, String),
    #[error("terminated")]
    Terminated,
    #[error("IO Error during {operation}:\n  {kind}: {message}")]
    IoDetailed {
        operation: Cow<'static, str>,
        message: String,
        kind: std::io::ErrorKind,
        os_error: Option<i32>,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, context: Expression, depth: usize) -> Self {
        Self {
            kind,
            context,
            depth,
        }
    }
    // pub fn from_io_error(io_err: std::io::Error, context: Expression, depth: usize) -> Self {
    //     Self::new(RuntimeErrorKind::Io(io_err), context, depth)
    // }
    pub fn from_io_error(
        io_err: std::io::Error,
        operation: Cow<'static, str>,
        context: Expression,
        depth: usize,
    ) -> Self {
        Self::new(
            RuntimeErrorKind::IoDetailed {
                operation,
                message: io_err.to_string(),
                kind: io_err.kind(),
                os_error: io_err.raw_os_error(),
            },
            context,
            depth,
        )
    }
    pub fn common(msg: Cow<'static, str>, context: Expression, depth: usize) -> Self {
        Self {
            kind: RuntimeErrorKind::CustomError(msg),
            context,
            depth,
        }
    }
}
const BLUE_START: &str = "\x1b[34m";
const DIM_START: &str = "\x1b[2m";
const RESET: &str = "\x1b[m\x1b[0m";
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 使用 RuntimeErrorKind 的 Display 实现
        writeln!(
            f,
            "{}Message   [{}]{}: {}",
            BLUE_START, self.depth, RESET, self.kind
        )?;
        writeln!(
            f,
            "{}Expression[{}]{}: {}",
            BLUE_START, self.depth, RESET, self.context,
        )?;
        writeln!(
            f,
            "{}SyntaxTree[{}]{}: {}{:?}{}",
            BLUE_START, self.depth, RESET, DIM_START, self.context, RESET
        )
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl RuntimeError {
    pub const ERROR_CODE_CANNOT_APPLY: Int = 1;
    pub const ERROR_CODE_SYMBOL_NOT_DEFINED: Int = 2;
    pub const ERROR_CODE_COMMAND_FAILED: Int = 3;
    pub const ERROR_CODE_FOR_NON_LIST: Int = 5;
    pub const ERROR_CODE_RECURSION_DEPTH: Int = 6;
    pub const ERROR_CODE_PERMISSION_DENIED: Int = 7;
    pub const ERROR_CODE_PROGRAM_NOT_FOUND: Int = 8;
    pub const ERROR_CODE_CUSTOM_ERROR: Int = 9;
    pub const ERROR_CODE_REDECLARATION: Int = 10; // Added for Redeclaration
    pub const ERROR_CODE_UNDECLARED_VARIABLE: Int = 11; // Added for UndeclaredVariable
    pub const ERROR_CODE_NO_MATCHING_BRANCH: Int = 12; // Added for NoMatchingBranch
    pub const ERROR_CODE_TOO_MANY_ARGUMENTS: Int = 13; // Added for TooManyArguments
    pub const ERROR_CODE_ARGUMENT_MISMATCH: Int = 14; // Added for ArgumentMismatch
    pub const ERROR_CODE_INVALID_DEFAULT_VALUE: Int = 15; // Added for InvalidDefaultValue
    pub const ERROR_CODE_INVALID_OPERATOR: Int = 16; // Added for InvalidOperator
    pub const ERROR_CODE_INDEX_OUT_OF_BOUNDS: Int = 17; // Added for IndexOutOfBounds
    pub const ERROR_CODE_KEY_NOT_FOUND: Int = 18; // Added for KeyNotFound
    pub const ERROR_CODE_TYPE_ERROR: Int = 19; // Added for TypeError
    pub const ERROR_CODE_EARLY_RETURN: Int = 20; // Added for EarlyReturn

    pub fn codes() -> Expression {
        Expression::from(b_tree_map! {
            String::from("cannot_apply") => Expression::Integer(Self::ERROR_CODE_CANNOT_APPLY),
            String::from("symbol_not_defined") => Expression::Integer(Self::ERROR_CODE_SYMBOL_NOT_DEFINED),
            String::from("command_failed") => Expression::Integer(Self::ERROR_CODE_COMMAND_FAILED),
            String::from("for_non_list") => Expression::Integer(Self::ERROR_CODE_FOR_NON_LIST),
            String::from("recursion_depth") => Expression::Integer(Self::ERROR_CODE_RECURSION_DEPTH),
            String::from("permission_denied") => Expression::Integer(Self::ERROR_CODE_PERMISSION_DENIED),
            String::from("program_not_found") => Expression::Integer(Self::ERROR_CODE_PROGRAM_NOT_FOUND),
            String::from("custom_error") => Expression::Integer(Self::ERROR_CODE_CUSTOM_ERROR),
            String::from("redeclaration") => Expression::Integer(Self::ERROR_CODE_REDECLARATION),
            String::from("undeclared_variable") => Expression::Integer(Self::ERROR_CODE_UNDECLARED_VARIABLE),
            String::from("no_matching_branch") => Expression::Integer(Self::ERROR_CODE_NO_MATCHING_BRANCH),
            String::from("too_many_arguments") => Expression::Integer(Self::ERROR_CODE_TOO_MANY_ARGUMENTS),
            String::from("argument_mismatch") => Expression::Integer(Self::ERROR_CODE_ARGUMENT_MISMATCH),
            String::from("invalid_default_value") => Expression::Integer(Self::ERROR_CODE_INVALID_DEFAULT_VALUE),
            String::from("invalid_operator") => Expression::Integer(Self::ERROR_CODE_INVALID_OPERATOR),
            String::from("index_out_of_bounds") => Expression::Integer(Self::ERROR_CODE_INDEX_OUT_OF_BOUNDS),
            String::from("key_not_found") => Expression::Integer(Self::ERROR_CODE_KEY_NOT_FOUND),
            String::from("type_error") => Expression::Integer(Self::ERROR_CODE_TYPE_ERROR),
            String::from("early_return") => Expression::Integer(Self::ERROR_CODE_EARLY_RETURN),
        })
    }

    pub fn code(&self) -> Int {
        match self.kind {
            RuntimeErrorKind::CannotApply(..) => Self::ERROR_CODE_CANNOT_APPLY,
            RuntimeErrorKind::SymbolNotDefined(..) => Self::ERROR_CODE_SYMBOL_NOT_DEFINED,
            RuntimeErrorKind::CommandFailed(..) | RuntimeErrorKind::CommandFailed2(..) => {
                Self::ERROR_CODE_COMMAND_FAILED
            }
            RuntimeErrorKind::ForNonList(..) => Self::ERROR_CODE_FOR_NON_LIST,
            RuntimeErrorKind::RecursionDepth(..) => Self::ERROR_CODE_RECURSION_DEPTH,
            RuntimeErrorKind::PermissionDenied(..) => Self::ERROR_CODE_PERMISSION_DENIED,
            RuntimeErrorKind::ProgramNotFound(..) => Self::ERROR_CODE_PROGRAM_NOT_FOUND,
            RuntimeErrorKind::Redeclaration(..) => Self::ERROR_CODE_REDECLARATION,
            RuntimeErrorKind::UndeclaredVariable(..) => Self::ERROR_CODE_UNDECLARED_VARIABLE,
            RuntimeErrorKind::NoMatchingBranch(..) => Self::ERROR_CODE_NO_MATCHING_BRANCH,
            RuntimeErrorKind::TooManyArguments { .. } => Self::ERROR_CODE_TOO_MANY_ARGUMENTS,
            RuntimeErrorKind::ArgumentMismatch { .. } => Self::ERROR_CODE_ARGUMENT_MISMATCH,
            RuntimeErrorKind::InvalidDefaultValue(..) => Self::ERROR_CODE_INVALID_DEFAULT_VALUE,
            RuntimeErrorKind::InvalidOperator(..) => Self::ERROR_CODE_INVALID_OPERATOR,
            RuntimeErrorKind::IndexOutOfBounds { .. } => Self::ERROR_CODE_INDEX_OUT_OF_BOUNDS,
            RuntimeErrorKind::KeyNotFound(..) => Self::ERROR_CODE_KEY_NOT_FOUND,
            RuntimeErrorKind::TypeError { .. } => Self::ERROR_CODE_TYPE_ERROR,
            RuntimeErrorKind::EarlyReturn(..) => Self::ERROR_CODE_EARLY_RETURN,
            _ => Self::ERROR_CODE_CUSTOM_ERROR,
        }
    }
}
