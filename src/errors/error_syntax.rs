use crate::{Diagnostic, tokens::Tokens};

use core::fmt;
use detached_str::{Str, StrSlice};
use nom::error::{ErrorKind, ParseError};
use std::error::Error as StdError;

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
    InternalError(String),
    CustomError(String, StrSlice),
    UnknownOperator(String, StrSlice),
    UnExpectedToken(String, StrSlice),
    InvalidEscapeSequence(String, StrSlice),
    PrecedenceTooLow(StrSlice),
    NoExpression,
    ArgumentMismatch {
        name: String,
        expected: u8,
        received: u8,
    },
    RecursionDepth {
        input: StrSlice,
        depth: usize,
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
        use SyntaxErrorKind::*;

        match (&self, &other) {
            // InternalError 优先级最低，总是被其他错误替换
            (InternalError(_), _) => other,
            (_, InternalError(_)) => self,

            // Expected 错误优先级较高，包含具体的期望信息
            (Expected { .. }, NomError { .. }) => self,
            (NomError { .. }, Expected { .. }) => other,

            // TokenizationErrors 是致命错误，优先级最高
            (TokenizationErrors(_), _) => self,
            (_, TokenizationErrors(_)) => other,

            // RecursionDepth 是严重错误，优先级很高
            (RecursionDepth { .. }, _) => self,
            (_, RecursionDepth { .. }) => other,

            // ArgumentMismatch 比一般错误更具体
            (ArgumentMismatch { .. }, NomError { .. }) => self,
            (NomError { .. }, ArgumentMismatch { .. }) => other,

            // UnknownOperator 比 NoExpression 更具体
            (UnknownOperator(..), NoExpression) => self,
            (NoExpression, UnknownOperator(..)) => other,

            // 对于相同类型的错误，选择包含更多上下文信息的
            (
                Expected {
                    input: input1,
                    hint: hint1,
                    ..
                },
                Expected {
                    input: input2,
                    hint: hint2,
                    ..
                },
            ) => {
                // 优先选择有 hint 的错误
                if hint1.is_some() && hint2.is_none() {
                    self
                } else if hint1.is_none() && hint2.is_some() {
                    other
                } else {
                    // 选择输入位置更靠前的错误（通常更相关）
                    if input1.start() <= input2.start() {
                        self
                    } else {
                        other
                    }
                }
            }

            // 默认情况：保留第一个错误
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
            SyntaxErrorKind::InternalError(_) => other,
            SyntaxErrorKind::TokenizationErrors(..) => self,
            //    ExpectedChar { /* … */ }=>
            // Expected { /* … */ },
            //    NomError { /* … */ },
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
                write!(f, "{RED_START}{BOLD}syntax error{RESET}: ")?;
                write!(f, "expect {YELLOW_START}{expected}{RESET}")?;
                if let Some(found) = found {
                    write!(f, ", found {RED2_START}{found}{RESET}")?;
                }
                writeln!(f)?;
                // 使用增强的错误显示
                print_error_lines(&self.source, *input, f, 72)?;

                // print_error_lines(&self.source, *input, f, 72)?;
                if let Some(hint) = hint {
                    writeln!(f, "    hint: {hint}")?;
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
                write!(f, "{RED_START}{BOLD}syntax error{RESET}: ")?;
                write!(f, "expect character {YELLOW_START}{expected:?}{RESET}")?;
                writeln!(f)?;
                if let Some(at) = at {
                    print_error_lines(&self.source, *at, f, 72)?;
                    // writeln!(f, "    hint: check if quotes or brackets are properly closed")?;
                }
                Ok(())
            }
            SyntaxErrorKind::NomError { kind, at, cause } => {
                write!(f, "{RED_START}{BOLD}nom syntax error{RESET}: ")?;
                writeln!(f, "`{kind:?}`")?;
                if let Some(at) = at {
                    print_error_lines(&self.source, *at, f, 72)?;
                }
                if let Some(cause) = cause {
                    writeln!(f, "Caused by: {cause}")?;
                }
                Ok(())
            }
            SyntaxErrorKind::InternalError(s) => {
                writeln!(f, "{RED_START}{BOLD}internal syntax error: {s}{RESET}")
            }
            SyntaxErrorKind::CustomError(s, at) => {
                writeln!(f, "{RED_START}{BOLD}syntax error: {s}{RESET}")?;
                print_error_lines(&self.source, *at, f, 72)?;
                Ok(())
            }
            SyntaxErrorKind::NoExpression => {
                writeln!(f, "{RED_START}{BOLD}no expression recognized{RESET}")
            }
            SyntaxErrorKind::UnknownOperator(op, at) => {
                writeln!(f, "{RED_START}{BOLD}unknown operator {op:?}{RESET}")?;
                print_error_lines(&self.source, *at, f, 72)?;
                Ok(())
            }
            SyntaxErrorKind::UnExpectedToken(op, at) => {
                writeln!(f, "{RED_START}{BOLD}unexpected token {op:?}{RESET}")?;
                print_error_lines(&self.source, *at, f, 72)?;
                Ok(())
            }
            SyntaxErrorKind::InvalidEscapeSequence(op, at) => {
                writeln!(f, "{RED_START}{BOLD}invalid escape sequence {op:?}{RESET}")?;
                print_error_lines(&self.source, *at, f, 72)?;
                Ok(())
            }
            SyntaxErrorKind::PrecedenceTooLow(at) => {
                writeln!(f, "{RED_START}{BOLD}precedence too low {RESET}")?;
                print_error_lines(&self.source, *at, f, 72)?;
                Ok(())
            }
            SyntaxErrorKind::ArgumentMismatch {
                name,
                expected,
                received,
            } => {
                writeln!(
                    f,
                    "{RED_START}{BOLD}arguments mismatch for function `{name}`: expected {expected}, found {received} {RESET}"
                )
            }
            SyntaxErrorKind::RecursionDepth { input, depth } => {
                write!(f, "{RED_START}{BOLD}max recursion reached{RESET}: ")?;
                write!(f, "depth: {YELLOW_START}{depth}{RESET}")?;

                writeln!(f)?;
                print_error_lines(&self.source, *input, f, 72)?;
                writeln!(
                    f,
                    "    hint: simplify your script, or config LUME_MAX_SYNTAX_RECURSION larger."
                )?;
                Ok(())
            }
        }
    }
}

// ============== 彩色显示辅助函数 ==============

fn fmt_token_error(string: &Str, err: &Diagnostic, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{RED_START}{BOLD}token error{RESET}: ")?;
    match err {
        Diagnostic::Valid => Ok(()),
        Diagnostic::InvalidUnicode(ranges) => {
            for &at in ranges.iter() {
                let escape = at.to_str(string).trim();
                writeln!(f, "invalid unicode sequence `{escape}`")?;
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
        Diagnostic::InvalidStringEscapes(ranges) => {
            for &at in ranges.iter() {
                let escape = at.to_str(string).trim();
                writeln!(f, "invalid string escape sequence `{escape}`")?;
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
        Diagnostic::InvalidColorCode(ranges) => {
            for &at in ranges.iter() {
                let escape = at.to_str(string).trim();
                writeln!(f, "invalid color code sequence `{escape}`")?;
                print_error_lines(string, at, f, 72)?;
            }
            Ok(())
        }
        &Diagnostic::InvalidNumber(at) => {
            let num = at.to_str(string).trim();
            writeln!(f, "invalid number `{num}`")?;
            print_error_lines(string, at, f, 72)
        }
        &Diagnostic::IllegalChar(at) => {
            writeln!(f, "invalid char {:?}", at.to_str(string))?;
            print_error_lines(string, at, f, 72)
        }
        &Diagnostic::NotTokenized(at) => {
            writeln!(
                f,
                "there are leftover tokens after tokenizing:\n{}",
                at.to_str(string)
            )?;
            print_error_lines(string, at, f, 72)
        }
    }
}

// 添加新的颜色常量
const DIM_START: &str = "\x1b[2m";
// const GREEN_START: &str = "\x1b[32m";
const BLUE_START: &str = "\x1b[34m";
const YELLOW_START: &str = "\x1b[38;5;230m";
const RED2_START: &str = "\x1b[38;5;210m";
const RED_START: &str = "\x1b[38;5;9m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[m\x1b[0m";

fn print_error_lines(
    string: &Str,
    at: StrSlice,
    f: &mut fmt::Formatter,
    _max_width: usize,
) -> fmt::Result {
    let error_start = at.start();
    let error_end = at.end();

    // 计算错误所在的行号和列号
    let before_text = &string[..error_start];
    let lines_before: Vec<&str> = before_text.lines().collect();
    let error_line_num = lines_before.len();
    let error_col = lines_before.last().map(|line| line.len()).unwrap_or(0);

    // 获取错误周围的上下文行（前后各3行）
    let all_lines: Vec<&str> = string.lines().collect();
    let context_start = error_line_num.saturating_sub(3);
    let context_end = (error_line_num + 3).min(all_lines.len());

    writeln!(f, "     {BLUE_START} ▏{RESET}")?;

    // 显示上下文行
    for (i, line) in all_lines[context_start..context_end].iter().enumerate() {
        let line_num = context_start + i + 1;
        let is_error_line = line_num == error_line_num;

        // dbg!(is_error_line, line_num, error_line_num, i, line);
        if is_error_line {
            // 安全地计算行内位置
            let line_start = before_text.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
            let error_start_in_line = error_start.saturating_sub(line_start);
            let error_end_in_line = (error_end.saturating_sub(line_start)).min(line.len());

            // 确保索引不超出行的范围
            let safe_start = error_start_in_line.min(line.len());
            let safe_end = error_end_in_line.min(line.len()).max(safe_start);
            // dbg!(error_start_in_line, error_end_in_line, safe_start, safe_end);

            write!(f, "{RED_START}{line_num:>5}{RESET} {BLUE_START}▏{RESET} ")?;
            if safe_start > 0 {
                write!(f, "{}", &line[..safe_start])?;
            }
            if safe_end > safe_start {
                write!(f, "{}{}{}", RED_START, &line[safe_start..safe_end], RESET)?;
            }
            if safe_end < line.len() {
                writeln!(f, "{}", &line[safe_end..])?;
            } else {
                writeln!(f)?;
            }

            // 添加指示箭头（只有在有错误内容时才显示）
            if safe_end >= safe_start {
                write!(f, "      {BLUE_START}▏{RESET} ")?;
                for _ in 0..safe_start {
                    write!(f, " ")?;
                }
                write!(f, "{RED_START}{BOLD}\x1b[5m^")?;
                for _ in 1..(safe_end - safe_start) {
                    write!(f, "~")?;
                }
                writeln!(f, "{RESET}")?;
            }
        } else {
            // 普通上下文行
            writeln!(
                f,
                "{BLUE_START}{line_num:>5} ▏{RESET} {DIM_START}{line}{RESET}"
            )?;
        }
    }

    writeln!(f, "     {BLUE_START} ▏{RESET}")?;

    // 显示错误位置信息
    writeln!(
        f,
        "      ↳ at line {}, column {}",
        error_line_num,
        error_col + 1
    )?;

    Ok(())
}
