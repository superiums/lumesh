use crate::Diagnostic;
use crate::tokens::Tokens;
use detached_str::StrSlice;
use nom::error::{ErrorKind, ParseError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxError {
    TokenizationErrors(Box<[Diagnostic]>),
    Expected {
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    },
    ExpectedChar {
        expected: char,
        at: Option<StrSlice>,
    },
    NomError {
        kind: nom::error::ErrorKind,
        at: Option<StrSlice>,
        cause: Option<Box<SyntaxError>>,
    },
    InternalError,
    NoExpression,
}

impl SyntaxError {
    pub(crate) fn unrecoverable(
        input: StrSlice,
        expected: &'static str,
        found: Option<String>,
        hint: Option<&'static str>,
    ) -> nom::Err<SyntaxError> {
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
    ) -> nom::Err<SyntaxError> {
        nom::Err::Error(Self::Expected {
            input,
            expected,
            found,
            hint,
        })
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

// enhanced
impl SyntaxError {
    pub fn unclosed_delimiter(start: StrSlice, delim: &'static str) -> Self {
        Self::Expected {
            input: start,
            expected: delim,
            found: None,
            hint: Some("检查括号/引号是否匹配"),
        }
    }

    // pub fn with_context(self, context: &str) -> Self {
    //     match self {
    //         Self::Expected { hint, .. } => Self::Expected {
    //             hint: Some(context),
    //             ..self
    //         },
    //         _ => self,
    //     }
    // }
}
