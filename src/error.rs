use super::{Expression, Int};
use crate::{Diagnostic, tokens::Tokens};
use common_macros::b_tree_map;
use core::{cmp::max, fmt};
use detached_str::{Str, StrSlice};
use nom::error::{ErrorKind, ParseError};
use smallstr::SmallString;
use std::error::Error as StdError;
use thiserror::Error;

// ============== 语法错误部分 ==============

#[derive(Debug)]
pub struct SyntaxError {
    pub source: Str,
    pub kind: SyntaxErrorKind,
}

#[derive(Debug)]
pub enum SyntaxErrorKind {
    Expected {
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    },
    TokenizationErrors(Box<[Diagnostic]>),
    ExpectedChar {
        expected: char,
        at: Option<StrSlice>,
    },
    NomError {
        kind: ErrorKind,
        at: Option<StrSlice>,
        cause: Option<Box<SyntaxError>>,
    },
    InternalError,
    UnknownOperator(String),
    PrecedenceTooLow,
    NoExpression,
    ArgumentMismatch {
        name: String,
        expected: usize,
        received: usize,
    },
    RecursionDepth {
        input: StrSlice,
        depth: u8,
    },
}

// impl StdError for SyntaxError {
//     fn source(&self) -> Option<&(dyn StdError + 'static)> {
//         match &self.kind {
//             SyntaxErrorKind::NomError { cause, .. } => cause.as_deref(),
//             _ => None,
//         }
//     }
// }

impl StdError for SyntaxError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.kind {
            SyntaxErrorKind::NomError { cause, .. } => {
                // Box the cause to convert it to a trait object
                cause
                    .as_ref()
                    .map(|c| c.as_ref() as &(dyn StdError + 'static))
            }
            _ => None,
        }
    }
}
impl SyntaxError {
    pub fn new(source: Str, kind: SyntaxErrorKind) -> Self {
        Self { source, kind }
    }

    // pub fn expected(
    //     source: Str,
    //     input: StrSlice,
    //     expected: &'static str,
    //     found: Option<String>,
    //     hint: Option<&'static str>,
    // ) -> nom::Err<Self> {
    //     nom::Err::Error(Self::new(
    //         source,
    //         SyntaxErrorKind::Expected {
    //             input,
    //             expected,
    //             found,
    //             hint,
    //         },
    //     ))
    // }

    // pub fn unclosed_delimiter(source: Str, start: StrSlice, delim: &'static str) -> Self {
    //     Self::new(
    //         source,
    //         SyntaxErrorKind::Expected {
    //             input: start,
    //             expected: delim,
    //             found: None,
    //             hint: Some("检查括号/引号是否匹配"),
    //         },
    //     )
    // }
}
impl SyntaxErrorKind {
    pub fn failure(
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    ) -> nom::Err<Self> {
        nom::Err::Failure(SyntaxErrorKind::Expected {
            input,
            expected,
            found,
            hint,
        })
    }
    /// return Fail to stop all parse. use this **carefully**!
    pub fn empty_fail(input: Tokens<'_>) -> Result<(), nom::Err<Self>> {
        if input.is_empty() {
            Err(nom::Err::Failure(SyntaxErrorKind::Expected {
                input: input.get_str_slice(),
                expected: "Some Expression",
                found: Some("Nothing".into()),
                hint: None,
            }))
        } else {
            Ok(())
        }
    }
    /// return an Error to stop process.
    pub fn empty_back(input: Tokens<'_>) -> Result<(), nom::Err<Self>> {
        if input.is_empty() {
            Err(nom::Err::Error(SyntaxErrorKind::Expected {
                input: input.get_str_slice(),
                expected: "Some Expression to parse",
                found: Some("Nothing".into()),
                hint: None,
            }))
        } else {
            Ok(())
        }
    }

    pub fn expected(
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    ) -> nom::Err<Self> {
        nom::Err::Error(SyntaxErrorKind::Expected {
            input,
            expected,
            found,
            hint,
        })
    }

    pub fn unclosed_delimiter(start: StrSlice, delim: &'static str) -> nom::Err<Self> {
        nom::Err::Error(SyntaxErrorKind::Expected {
            input: start,
            expected: delim,
            found: None,
            hint: Some("Check if parentheses/quotes are matched"),
        })
    }
}

impl ParseError<Tokens<'_>> for SyntaxErrorKind {
    fn from_error_kind(input: Tokens<'_>, kind: ErrorKind) -> Self {
        SyntaxErrorKind::NomError {
            kind,
            at: input.first().map(|t| t.range),
            cause: None,
        }
    }

    fn append(input: Tokens<'_>, kind: ErrorKind, _: Self) -> Self {
        SyntaxErrorKind::NomError {
            kind,
            at: input.first().map(|t| t.range),
            cause: None,
        }
    }

    fn from_char(input: Tokens<'_>, expected: char) -> Self {
        SyntaxErrorKind::ExpectedChar {
            expected,
            at: input.first().map(|t| t.range),
        }
    }

    fn or(self, other: Self) -> Self {
        match self {
            SyntaxErrorKind::InternalError => other,
            _ => self,
        }
    }
}
impl ParseError<Tokens<'_>> for SyntaxError {
    fn from_error_kind(input: Tokens<'_>, kind: ErrorKind) -> Self {
        Self::new(
            input.str.clone(),
            SyntaxErrorKind::NomError {
                kind,
                at: input.first().map(|t| t.range),
                cause: None,
            },
        )
    }

    fn append(input: Tokens<'_>, kind: ErrorKind, other: Self) -> Self {
        Self::new(
            input.str.clone(),
            SyntaxErrorKind::NomError {
                kind,
                at: input.first().map(|t| t.range),
                cause: Some(Box::new(other)),
            },
        )
    }

    fn from_char(input: Tokens<'_>, expected: char) -> Self {
        Self::new(
            input.str.clone(),
            SyntaxErrorKind::ExpectedChar {
                expected,
                at: input.first().map(|t| t.range),
            },
        )
    }

    fn or(self, other: Self) -> Self {
        match self.kind {
            SyntaxErrorKind::InternalError => other,
            _ => self,
        }
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            SyntaxErrorKind::Expected {
                input,
                expected,
                found,
                hint,
            } => {
                write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
                write!(f, "expect {}{}{}", YELLOW_START, expected, RESET)?;
                if let Some(found) = found {
                    write!(f, ", found {}{}{}", RED2_START, found, RESET)?;
                }
                writeln!(f)?;
                print_error_lines(&self.source, *input, f, 72)?;
                if let Some(hint) = hint {
                    writeln!(f, "    hint: {}", hint)?;
                }
                Ok(())
            }
            SyntaxErrorKind::TokenizationErrors(errors) => {
                for err in errors.iter() {
                    fmt_token_error(&self.source, err, f)?;
                }
                Ok(())
            }
            SyntaxErrorKind::ExpectedChar { expected, at } => {
                write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
                writeln!(f, "expect {:?}", expected)?;
                if let Some(at) = at {
                    print_error_lines(&self.source, *at, f, 72)?;
                }
                Ok(())
            }
            SyntaxErrorKind::NomError { kind, at, cause } => {
                write!(f, "{}{}unexpected syntax error{}: ", RED_START, BOLD, RESET)?;
                writeln!(f, "`{:?}`", kind)?;
                if let Some(at) = at {
                    print_error_lines(&self.source, *at, f, 72)?;
                }
                if let Some(cause) = cause {
                    writeln!(f, "Caused by: {}", cause)?;
                }
                Ok(())
            }
            SyntaxErrorKind::InternalError => {
                writeln!(f, "{}{}unexpected syntax error{}", RED_START, BOLD, RESET)
            }
            SyntaxErrorKind::NoExpression => {
                writeln!(f, "{}{}no expression recognized{}", RED_START, BOLD, RESET)
            }
            SyntaxErrorKind::UnknownOperator(op) => {
                writeln!(f, "{}{}unknown operator {op:?}{}", RED_START, BOLD, RESET)
            }
            SyntaxErrorKind::PrecedenceTooLow => {
                writeln!(f, "{}{}precedence too low {}", RED_START, BOLD, RESET)
            }
            SyntaxErrorKind::ArgumentMismatch {
                name,
                expected,
                received,
            } => {
                writeln!(
                    f,
                    "{}{}arguments mismatch for function `{name}`: expected {expected}, found {received} {}",
                    RED_START, BOLD, RESET
                )
            }
            SyntaxErrorKind::RecursionDepth { input, depth } => {
                write!(f, "{}{}max recursion reached{}: ", RED_START, BOLD, RESET)?;
                write!(f, "depth: {}{}{}", YELLOW_START, depth, RESET)?;

                writeln!(f)?;
                print_error_lines(&self.source, *input, f, 72)?;
                writeln!(
                    f,
                    "    hint: simplify your script, or config LUME_MAX_PARSE_RECURSION larger."
                )?;
                Ok(())
            }
        }
    }
}

// ============== 运行时错误部分 ==============

#[derive(Debug, Error)]
pub enum RuntimeError {
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
    #[error("{0}")]
    CustomError(String),
    #[error("redeclaration of `{0}`")]
    Redeclaration(SmallString<[u8; 16]>),
    #[error("undeclared variable: {0}")]
    UndeclaredVariable(SmallString<[u8; 16]>),
    #[error("no matching branch while evaluating `{0}`")]
    NoMatchingBranch(String),
    #[error("too many arguments for function `{name}`: max {max}, found {received}")]
    TooManyArguments {
        name: SmallString<[u8; 16]>,
        max: usize,
        received: usize,
    },
    #[error("arguments mismatch for function `{name}`: expected {expected}, found {received}")]
    ArgumentMismatch {
        name: SmallString<[u8; 16]>,
        expected: usize,
        received: usize,
    },
    #[error("invalid default value `{2}` for argument `{1}` in function `{0}`")]
    InvalidDefaultValue(SmallString<[u8; 16]>, String, Expression),
    #[error("invalid operator `{0}`")]
    InvalidOperator(SmallString<[u8; 3]>),
    #[error("index {index} out of bounds (length {len})")]
    IndexOutOfBounds { index: Int, len: usize },
    #[error("key `{0}` not found in map")]
    KeyNotFound(SmallString<[u8; 16]>),
    #[error("type error: expected {expected}, found {found}:{found_type}")]
    TypeError {
        expected: String,
        found: String,
        found_type: String,
    },
    #[error("illegal return outside function")]
    EarlyReturn(Expression),
    #[error("illegal break outside loop")]
    EarlyBreak(Expression),
    #[error("overflowed when: `{0}`")]
    Overflow(String),
    #[error("wildcard not matched: `{0}`")]
    WildcardNotMatched(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

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
            SmallString::from("cannot_apply") => Expression::Integer(Self::ERROR_CODE_CANNOT_APPLY),
            SmallString::from("symbol_not_defined") => Expression::Integer(Self::ERROR_CODE_SYMBOL_NOT_DEFINED),
            SmallString::from("command_failed") => Expression::Integer(Self::ERROR_CODE_COMMAND_FAILED),
            SmallString::from("for_non_list") => Expression::Integer(Self::ERROR_CODE_FOR_NON_LIST),
            SmallString::from("recursion_depth") => Expression::Integer(Self::ERROR_CODE_RECURSION_DEPTH),
            SmallString::from("permission_denied") => Expression::Integer(Self::ERROR_CODE_PERMISSION_DENIED),
            SmallString::from("program_not_found") => Expression::Integer(Self::ERROR_CODE_PROGRAM_NOT_FOUND),
            SmallString::from("custom_error") => Expression::Integer(Self::ERROR_CODE_CUSTOM_ERROR),
            SmallString::from("redeclaration") => Expression::Integer(Self::ERROR_CODE_REDECLARATION),
            SmallString::from("undeclared_variable") => Expression::Integer(Self::ERROR_CODE_UNDECLARED_VARIABLE),
            SmallString::from("no_matching_branch") => Expression::Integer(Self::ERROR_CODE_NO_MATCHING_BRANCH),
            SmallString::from("too_many_arguments") => Expression::Integer(Self::ERROR_CODE_TOO_MANY_ARGUMENTS),
            SmallString::from("argument_mismatch") => Expression::Integer(Self::ERROR_CODE_ARGUMENT_MISMATCH),
            SmallString::from("invalid_default_value") => Expression::Integer(Self::ERROR_CODE_INVALID_DEFAULT_VALUE),
            SmallString::from("invalid_operator") => Expression::Integer(Self::ERROR_CODE_INVALID_OPERATOR),
            SmallString::from("index_out_of_bounds") => Expression::Integer(Self::ERROR_CODE_INDEX_OUT_OF_BOUNDS),
            SmallString::from("key_not_found") => Expression::Integer(Self::ERROR_CODE_KEY_NOT_FOUND),
            SmallString::from("type_error") => Expression::Integer(Self::ERROR_CODE_TYPE_ERROR),
            SmallString::from("early_return") => Expression::Integer(Self::ERROR_CODE_EARLY_RETURN),
        })
    }

    pub fn code(&self) -> Int {
        match self {
            RuntimeError::CannotApply(..) => Self::ERROR_CODE_CANNOT_APPLY,
            RuntimeError::SymbolNotDefined(..) => Self::ERROR_CODE_SYMBOL_NOT_DEFINED,
            RuntimeError::CommandFailed(..) | RuntimeError::CommandFailed2(..) => {
                Self::ERROR_CODE_COMMAND_FAILED
            }
            RuntimeError::ForNonList(..) => Self::ERROR_CODE_FOR_NON_LIST,
            RuntimeError::RecursionDepth(..) => Self::ERROR_CODE_RECURSION_DEPTH,
            RuntimeError::PermissionDenied(..) => Self::ERROR_CODE_PERMISSION_DENIED,
            RuntimeError::ProgramNotFound(..) => Self::ERROR_CODE_PROGRAM_NOT_FOUND,
            RuntimeError::Redeclaration(..) => Self::ERROR_CODE_REDECLARATION,
            RuntimeError::UndeclaredVariable(..) => Self::ERROR_CODE_UNDECLARED_VARIABLE,
            RuntimeError::NoMatchingBranch(..) => Self::ERROR_CODE_NO_MATCHING_BRANCH,
            RuntimeError::TooManyArguments { .. } => Self::ERROR_CODE_TOO_MANY_ARGUMENTS,
            RuntimeError::ArgumentMismatch { .. } => Self::ERROR_CODE_ARGUMENT_MISMATCH,
            RuntimeError::InvalidDefaultValue(..) => Self::ERROR_CODE_INVALID_DEFAULT_VALUE,
            RuntimeError::InvalidOperator(..) => Self::ERROR_CODE_INVALID_OPERATOR,
            RuntimeError::IndexOutOfBounds { .. } => Self::ERROR_CODE_INDEX_OUT_OF_BOUNDS,
            RuntimeError::KeyNotFound(..) => Self::ERROR_CODE_KEY_NOT_FOUND,
            RuntimeError::TypeError { .. } => Self::ERROR_CODE_TYPE_ERROR,
            RuntimeError::EarlyReturn(..) => Self::ERROR_CODE_EARLY_RETURN,
            _ => Self::ERROR_CODE_CUSTOM_ERROR,
        }
    }
}

impl LmError {
    pub const ERROR_CODE_RUNTIME_ERROR: Int = 100;
    pub const ERROR_CODE_SYNTAX_ERROR: Int = 101;
    pub const ERROR_CODE_IO_ERROR: Int = 102;
    pub const ERROR_CODE_CS_ERROR: Int = 103;
    pub fn codes() -> Expression {
        Expression::from(b_tree_map! {
            SmallString::from("runtime_error") => Expression::Integer(Self::ERROR_CODE_RUNTIME_ERROR),
            SmallString::from("syntax_error") => Expression::Integer(Self::ERROR_CODE_SYNTAX_ERROR),
            SmallString::from("io_error") => Expression::Integer(Self::ERROR_CODE_IO_ERROR),
            SmallString::from("custom_error") => Expression::Integer(Self::ERROR_CODE_CS_ERROR),
        })
    }
    pub fn code(&self) -> Int {
        match self {
            Self::Syntax(_) => Self::ERROR_CODE_SYNTAX_ERROR,
            Self::Runtime(err) => err.code(),
            Self::Io(_) => Self::ERROR_CODE_IO_ERROR,
            Self::CustomError(_) => Self::ERROR_CODE_CS_ERROR,
        }
    }
}

// ============== 彩色显示辅助函数 ==============

fn fmt_token_error(string: &Str, err: &Diagnostic, f: &mut fmt::Formatter) -> fmt::Result {
    match err {
        Diagnostic::Valid => Ok(()),
        Diagnostic::InvalidUnicode(ranges) => {
            for &at in ranges.iter() {
                write!(f, "{}{}syntax error{}: ", RED_START, BOLD, RESET)?;
                let escape = at.to_str(string).trim();
                writeln!(f, "invalid unicode sequence `{}`", escape)?;
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
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
    // dbg!(&lines, line_before, line_after, first_line_number);
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

const YELLOW_START: &str = "\x1b[38;5;230m";
const RED2_START: &str = "\x1b[38;5;210m";
const RED_START: &str = "\x1b[38;5;9m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[m\x1b[0m";
