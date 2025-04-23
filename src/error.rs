use common_macros::b_tree_map;
use detached_str::{Str, StrSlice};
use nom::error::{ErrorKind, ParseError};
use thiserror::Error;

use core::{cmp::max, fmt};

use crate::{Diagnostic, tokens::Tokens};

use super::{Expression, Int};

#[derive(Debug, Error)]
pub enum LmError {
    #[error("cannot apply `{0:?}` to the arguments {1:?}")]
    CannotApply(Expression, Vec<Expression>),
    #[error("symbol \"{0}\" not defined")]
    SymbolNotDefined(String),
    #[error("command `{0}` failed with args {1:?}")]
    CommandFailed(String, Vec<Expression>),
    #[error("command `{0}` failed: {1:?}")]
    CommandFailed2(String, String),
    #[error("attempted to iterate over non-list `{0:?}`")]
    ForNonList(Expression),
    #[error("recursion depth exceeded while evaluating `{0:?}`")]
    RecursionDepth(Expression),
    #[error("permission denied while evaluating `{0:?}`")]
    PermissionDenied(Expression),
    #[error("program \"{0}\" not found")]
    ProgramNotFound(String),
    #[error("{1}")]
    SyntaxError(Str, #[source] SyntaxError),
    #[error("{0}")]
    CustomError(String),
    #[error("redeclaration of `{0}`")]
    Redeclaration(String),
    #[error("undeclared variable: {0}")]
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
    IndexOutOfBounds { index: usize, len: usize },
    #[error("key `{0}` not found in map")]
    KeyNotFound(String),
    #[error("type error: expected {expected}, found {found}")]
    TypeError { expected: String, found: String },
    #[error("illegal return outside function")]
    EarlyReturn(Expression),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("tokenization errors")]
    TokenizationErrors(Box<[Diagnostic]>),
    #[error("syntax error")]
    Expected {
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    },
    #[error("expected character `{expected}`")]
    ExpectedChar {
        expected: char,
        at: Option<StrSlice>,
    },
    #[error("parse error: `{kind:?}`")]
    NomError {
        kind: nom::error::ErrorKind,
        at: Option<StrSlice>,
        #[source]
        cause: Option<Box<SyntaxError>>,
    },
    #[error("internal parser error")]
    InternalError,
    #[error("no expression recognized")]
    NoExpression,
}

impl LmError {
    /// Error code constant integers for error handlers to handle.
    pub const ERROR_CODE_CANNOT_APPLY: Int = 1;
    pub const ERROR_CODE_SYMBOL_NOT_DEFINED: Int = 2;
    pub const ERROR_CODE_COMMAND_FAILED: Int = 3;
    pub const ERROR_CODE_FOR_NON_LIST: Int = 4;
    pub const ERROR_CODE_RECURSION_DEPTH: Int = 5;
    pub const ERROR_CODE_PERMISSION_DENIED: Int = 6;
    pub const ERROR_CODE_PROGRAM_NOT_FOUND: Int = 7;
    pub const ERROR_CODE_SYNTAX_ERROR: Int = 8;
    pub const ERROR_CODE_CUSTOM_ERROR: Int = 9;

    pub fn codes() -> Expression {
        Expression::Map(b_tree_map! {
            String::from("cannot_apply") => Expression::Integer(Self::ERROR_CODE_CANNOT_APPLY),
            String::from("symbol_not_defined") => Expression::Integer(Self::ERROR_CODE_SYMBOL_NOT_DEFINED),
            String::from("command_failed") => Expression::Integer(Self::ERROR_CODE_COMMAND_FAILED),
            String::from("for_non_list") => Expression::Integer(Self::ERROR_CODE_FOR_NON_LIST),
            String::from("recursion_depth") => Expression::Integer(Self::ERROR_CODE_RECURSION_DEPTH),
            String::from("permission_denied") => Expression::Integer(Self::ERROR_CODE_PERMISSION_DENIED),
            String::from("program_not_found") => Expression::Integer(Self::ERROR_CODE_PROGRAM_NOT_FOUND),
            String::from("syntax_error") => Expression::Integer(Self::ERROR_CODE_SYNTAX_ERROR),
            String::from("custom_error") => Expression::Integer(Self::ERROR_CODE_CUSTOM_ERROR),
        })
    }

    /// Convert the error into a code for an error handler to handle.
    pub fn code(&self) -> Int {
        match self {
            Self::CannotApply(..) => Self::ERROR_CODE_CANNOT_APPLY,
            Self::SymbolNotDefined(..) => Self::ERROR_CODE_SYMBOL_NOT_DEFINED,
            Self::CommandFailed(..) => Self::ERROR_CODE_COMMAND_FAILED,
            Self::CommandFailed2(..) => Self::ERROR_CODE_COMMAND_FAILED,
            Self::ForNonList(..) => Self::ERROR_CODE_FOR_NON_LIST,
            Self::RecursionDepth(..) => Self::ERROR_CODE_RECURSION_DEPTH,
            Self::PermissionDenied(..) => Self::ERROR_CODE_PERMISSION_DENIED,
            Self::ProgramNotFound(..) => Self::ERROR_CODE_PROGRAM_NOT_FOUND,
            Self::SyntaxError(..) => Self::ERROR_CODE_SYNTAX_ERROR,
            Self::CustomError(..)
            | Self::Redeclaration(..)
            | Self::UndeclaredVariable(..)
            | Self::NoMatchingBranch(..)
            | Self::TooManyArguments { .. }
            | Self::ArgumentMismatch { .. }
            | Self::InvalidDefaultValue(..)
            | Self::EarlyReturn(..)
            | Self::InvalidOperator(..)
            | Self::IndexOutOfBounds { .. }
            | Self::KeyNotFound(..)
            | Self::TypeError { .. } => Self::ERROR_CODE_CUSTOM_ERROR,
            Self::IoError(_) => Self::ERROR_CODE_CUSTOM_ERROR,
        }
    }
}

impl SyntaxError {
    pub(crate) fn unrecoverable(
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    ) -> nom::Err<Self> {
        nom::Err::Failure(Self::Expected {
            input,
            expected,
            found,
            hint,
        })
    }

    pub(crate) fn expected(
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    ) -> nom::Err<Self> {
        nom::Err::Error(Self::Expected {
            input,
            expected,
            found,
            hint,
        })
    }

    pub fn unclosed_delimiter(start: StrSlice, delim: &'static str) -> Self {
        Self::Expected {
            input: start,
            expected: delim,
            found: None,
            hint: Some("检查括号/引号是否匹配"),
        }
    }
}

impl ParseError<Tokens<'_>> for SyntaxError {
    fn from_error_kind(input: Tokens<'_>, kind: ErrorKind) -> Self {
        Self::NomError {
            kind,
            at: input.first().map(|t| t.range),
            cause: None,
        }
    }

    fn append(input: Tokens<'_>, kind: ErrorKind, other: Self) -> Self {
        Self::NomError {
            kind,
            at: input.first().map(|t| t.range),
            cause: Some(Box::new(other)),
        }
    }

    fn from_char(input: Tokens<'_>, expected: char) -> Self {
        Self::ExpectedChar {
            expected,
            at: input.first().map(|t| t.range),
        }
    }

    fn or(self, other: Self) -> Self {
        match self {
            Self::InternalError => other,
            _ => self,
        }
    }
}

fn fmt_syntax_error(string: &Str, err: &SyntaxError, f: &mut fmt::Formatter) -> fmt::Result {
    match err {
        SyntaxError::Expected {
            input,
            expected,
            found,
            hint,
        } => {
            write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
            write!(f, "expected {}", expected)?;
            if let Some(found) = found {
                write!(f, ", found {}", found)?;
            }
            writeln!(f)?;
            print_error_lines(string, *input, f, 72)?;
            if let Some(hint) = *hint {
                writeln!(f, "    hint: {}", hint)?;
            }
            Ok(())
        }
        SyntaxError::TokenizationErrors(errors) => {
            for err in errors.iter() {
                fmt_token_error(string, err, f)?;
            }
            Ok(())
        }
        SyntaxError::ExpectedChar { expected, at } => {
            write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
            writeln!(f, "expected {:?}", expected)?;
            if let Some(at) = *at {
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
        SyntaxError::NomError { kind, at, cause } => {
            write!(f, "{}{}unexpected syntax error{}: ", RED_START, BOLD, RESET)?;
            writeln!(f, "`{:?}`", kind)?;
            if let Some(at) = *at {
                print_error_lines(string, at, f, 72)?;
            }
            if let Some(cause) = cause {
                fmt_syntax_error(string, cause, f)?;
            }
            Ok(())
        }
        SyntaxError::InternalError => {
            writeln!(f, "{}{}unexpected syntax error{}", RED_START, BOLD, RESET)
        }
        SyntaxError::NoExpression => {
            writeln!(f, "{}{}no expression recognized{}", RED_START, BOLD, RESET)
        }
    }
}

fn fmt_token_error(string: &Str, err: &Diagnostic, f: &mut fmt::Formatter) -> fmt::Result {
    match err {
        Diagnostic::Valid => Ok(()),
        Diagnostic::InvalidStringEscapes(ranges) => {
            for &at in ranges.iter() {
                write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
                let escape = at.to_str(string).trim();
                writeln!(f, "invalid string escape sequence `{}`", escape)?;
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
        &Diagnostic::InvalidNumber(at) => {
            write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
            let num = at.to_str(string).trim();
            writeln!(f, "invalid number `{}`", num)?;
            print_error_lines(string, at, f, 72)
        }
        &Diagnostic::IllegalChar(at) => {
            write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
            writeln!(f, "invalid token {:?}", at.to_str(string))?;
            print_error_lines(string, at, f, 72)
        }
        &Diagnostic::NotTokenized(at) => {
            write!(f, "{}{}error{}: ", RED_START, BOLD, RESET)?;
            writeln!(
                f,
                "there are leftover tokens after tokenizing: {}",
                at.to_str(string)
            )?;
            print_error_lines(string, at, f, 72)
        }
    }
}

fn print_error_lines(
    string: &Str,
    at: StrSlice,
    f: &mut fmt::Formatter,
    max_width: usize,
) -> fmt::Result {
    let mut lines = at.to_str(string).lines().collect::<Vec<&str>>();
    if lines.is_empty() {
        lines.push("");
    }
    let singleline = lines.len() == 1;

    let before = &string[..at.start()];
    let after = &string[at.end()..];

    let line_before = before.lines().next_back().unwrap_or_default();
    let line_after = after.lines().next().unwrap_or_default();

    let first_line_number = max(before.lines().count(), 1);

    writeln!(f, "      |")?;

    if singleline {
        let before_len = line_before.chars().take(max_width).count().min(max_width);

        let line = line_before
            .chars()
            .take(max_width)
            .chain(RED_START.chars())
            .chain(lines[0].chars())
            .chain(RESET.chars())
            .chain(line_after.chars().take(max_width - before_len))
            .collect::<String>();

        writeln!(f, "{:>5} | {}", first_line_number, line)?;
    } else {
        let first_line = line_before
            .chars()
            .chain(RED_START.chars())
            .chain(lines[0].chars())
            .take(max_width)
            .chain(RESET.chars())
            .collect::<String>();
        write!(f, "{:>5} | {}", first_line_number, first_line)?;

        for (i, line) in lines.iter().copied().enumerate().skip(1) {
            let line = RED_START
                .chars()
                .chain(line.chars().take(max_width))
                .chain(RESET.chars())
                .collect::<String>();
            write!(f, "\n{:>5} | {}", first_line_number + i, line)?;
        }

        let last_len = lines.last().unwrap().chars().count();
        let suffix = line_after
            .chars()
            .take(max_width - last_len)
            .chain(RESET.chars())
            .collect::<String>();
        writeln!(f, "\n{}", suffix)?;
    }

    writeln!(f, "      |")?;

    Ok(())
}

const RED_START: &str = "\x1b[38;5;9m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[m\x1b[0m";
