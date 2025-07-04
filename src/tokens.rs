use core::option::Option::None;
use std::{fmt, ops::Deref};

use detached_str::{Str, StrSlice};
use nom::{InputLength, InputTake};

#[derive(Copy, Clone, Debug)]
pub(crate) struct Input<'a> {
    str: &'a Str,
    offset: usize,
}

impl Deref for Input<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.str[self.offset..]
    }
}

impl AsRef<str> for Input<'_> {
    fn as_ref(&self) -> &str {
        &self.str[self.offset..]
    }
}

impl<'a> Input<'a> {
    pub fn new(str: &'a Str) -> Self {
        Input { str, offset: 0 }
    }

    pub fn is_empty(self) -> bool {
        self.offset == self.str.len()
    }

    pub fn as_original_str(self) -> &'a str {
        self.str
    }

    pub fn as_str_slice(self) -> StrSlice {
        self.str.get(self.offset..)
    }

    pub fn split_empty(self) -> StrSlice {
        self.str.get(self.offset..self.offset)
    }

    pub fn split_at(self, n: usize) -> (Self, StrSlice) {
        let start = self.offset;
        let offset = start + n;

        let new_input = Input { offset, ..self };
        let slice = self.str.get(start..offset);
        (new_input, slice)
    }

    pub fn split_saturating(self, n: usize) -> (Self, StrSlice) {
        self.split_at(n.min(self.len()))
    }

    pub fn split_until(self, other: Input<'_>) -> (Self, StrSlice) {
        assert!(std::ptr::eq(self.str, other.str));
        let diff = self.len() - other.len();
        self.split_at(diff)
    }

    pub fn strip_prefix(self, prefix: &str) -> Option<(Self, StrSlice)> {
        if self.starts_with(prefix) {
            Some(self.split_at(prefix.len()))
        } else {
            None
        }
    }
    // 新增方法：获取当前 offset 的前一个字符
    pub fn previous_char(&self) -> Option<char> {
        if self.offset == 0 {
            return None; // 已经是字符串开头
        }

        let s = &self.str[..self.offset];
        let mut iter = s.char_indices().rev();

        // 找到前一个字符的起始位置
        match iter.next() {
            Some((idx, _)) => s[idx..].chars().next(),
            None => None,
        }
    }
    // 新增方法：获取当前 offset 的前n个字符
    // pub fn previous_n_char(&self, n: usize) -> Option<char> {
    //     let off_d = self.offset - n + 1;
    //     if off_d <= 0 || off_d > self.len() {
    //         return None; // 已经是字符串开头
    //     }

    //     let s = &self.str[..off_d];
    //     let mut iter = s.char_indices().rev();

    //     // 找到前一个字符的起始位置
    //     match iter.next() {
    //         Some((idx, _)) => s[idx..].chars().next(),
    //         None => None,
    //     }
    // }
}

impl nom::InputLength for Input<'_> {
    fn input_len(&self) -> usize {
        self.len()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub range: StrSlice,
}

impl Token {
    pub fn new(kind: TokenKind, range: StrSlice) -> Self {
        Token { kind, range }
    }

    pub fn text(self, tokens: Tokens<'_>) -> &str {
        self.range.to_str(tokens.str)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Punctuation,
    Operator,
    Keyword,
    StringLiteral,
    StringRaw,
    StringTemplate,
    IntegerLiteral,
    FloatLiteral,
    ValueSymbol,
    Symbol,
    Whitespace,
    LineBreak, //add newline
    OperatorInfix,
    OperatorPrefix,
    OperatorPostfix,
    // LineContinuation, //add linecontinue
    Comment,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}({}..{})",
            self.kind,
            self.range.start(),
            self.range.end(),
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Tokens<'a> {
    pub str: &'a Str,
    pub slice: &'a [Token],
}

impl std::ops::Deref for Tokens<'_> {
    type Target = [Token];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl InputLength for Tokens<'_> {
    fn input_len(&self) -> usize {
        self.slice.len()
    }
}

impl InputTake for Tokens<'_> {
    fn take(&self, count: usize) -> Self {
        Tokens {
            slice: &self.slice[count..],
            ..*self
        }
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let (a, b) = self.slice.split_at(count);
        (Tokens { slice: a, ..*self }, Tokens { slice: b, ..*self })
    }
}

impl Tokens<'_> {
    pub fn skip_n(self, count: usize) -> Self {
        let slice = &self.slice[count..];
        Tokens { slice, ..self }
    }

    pub fn get_str_slice(self) -> StrSlice {
        match self.slice.first() {
            Some(t) => t.range,
            None => {
                let len = self.str.len();
                self.str.get(len..len)
            }
        }
    }
}
