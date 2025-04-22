use crate::STRICT;
use detached_str::StrSlice;
use nom::{
    AsChar, IResult,
    branch::alt,
    combinator::{eof, map},
    error::ParseError,
    multi::fold_many_m_n,
    sequence::tuple,
};
use std::convert::TryFrom;

use crate::tokens::{Input, Token, TokenKind};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NotFoundError;

const NOT_FOUND: nom::Err<NotFoundError> = nom::Err::Error(NotFoundError);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    Valid,
    InvalidStringEscapes(Box<[StrSlice]>),
    InvalidNumber(StrSlice),
    IllegalChar(StrSlice),
    NotTokenized(StrSlice),
}

impl<I> ParseError<I> for NotFoundError {
    fn from_error_kind(_: I, _: nom::error::ErrorKind) -> Self {
        NotFoundError
    }

    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

type TokenizationResult<'a, T = StrSlice> = IResult<Input<'a>, T, NotFoundError>;

fn parse_token(input: Input) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if input.is_empty() {
        Err(NOT_FOUND)
    } else {
        Ok(alt((
            // 优先处理续航、换行符（新增）
            map_valid_token(line_continuation, TokenKind::Whitespace),
            // triple_quote_string,
            map_valid_token(linebreak, TokenKind::LineBreak),
            map_valid_token(argument_symbol, TokenKind::StringLiteral), //argument first to allow args such as = -
            map_valid_token(long_operator, TokenKind::Operator),
            map_valid_token(custome_operator, TokenKind::Operator), //before short_operator
            map_valid_token(any_punctuation, TokenKind::Punctuation),
            map_valid_token(any_keyword, TokenKind::Keyword),
            map_valid_token(bool_literal, TokenKind::BooleanLiteral),
            map_valid_token(comment, TokenKind::Comment),
            number_literal,
            map_valid_token(short_operator, TokenKind::Operator), //atfter number to avoid -4.
            string_literal,
            map_valid_token(symbol, TokenKind::Symbol),
            map_valid_token(whitespace, TokenKind::Whitespace),
        ))(input)
        .unwrap_or_else(|_| {
            let next = input.chars().next().unwrap();
            let (rest, range) = input.split_at(next.len_utf8());
            let token = Token::new(TokenKind::Symbol, range);
            (rest, (token, Diagnostic::IllegalChar(range)))
        }))
    }
}

fn map_valid_token(
    mut parser: impl FnMut(Input<'_>) -> TokenizationResult<'_>,
    kind: TokenKind,
) -> impl FnMut(Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    move |input| {
        let (input, s) = parser(input)?;
        Ok((input, (Token::new(kind, s), Diagnostic::Valid)))
    }
}

fn any_punctuation(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag(":="),
        punctuation_tag("("),
        punctuation_tag(")"),
        punctuation_tag("["),
        punctuation_tag("]"),
        punctuation_tag("{"),
        punctuation_tag("}"),
        // punctuation_tag("\'"),
        punctuation_tag(","),
        punctuation_tag(";"),
        // punctuation_tag("="),
        punctuation_tag("->"), // `->foo` is also a valid symbol
        punctuation_tag("~>"), // `~>foo` is also a valid symbol
    ))(input)
}

fn long_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        // keyword_tag("to"),
        operator_tag(".."),
        keyword_tag("=>"),  //for match
        operator_tag("=="), //to allow a==b
        operator_tag("!="),
        operator_tag(">="),
        operator_tag("<="),
        keyword_tag("~~"), //string contains
        keyword_tag("~="), //regex match
        keyword_tag("&&"),
        keyword_tag("||"),
        keyword_tag("|>"), //param pipe
        keyword_tag("<<"),
        keyword_tag(">>>"),
        keyword_tag(">>"),
        operator_tag("**"), //pow
        operator_tag("++"),
        operator_tag("--"),
        operator_tag("+="),
        operator_tag("-="),
        operator_tag("*="),
        operator_tag("/="),
    ))(input)
}

fn short_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        operator_tag("++"), // for op overload use.
        operator_tag("--"), // for op overload use.
        operator_tag("**"), // for op overload use.
        operator_tag("<"),
        operator_tag(">"),
        operator_tag("+"),    // to allow a<b insteadof mustbe a < b
        keyword_tag("-"),     // to allow a<b insteadof mustbe a < b
        operator_tag("*"),    // to allow a<b insteadof mustbe a < b
        keyword_tag("/"),     // to allow a<b insteadof mustbe a < b
        operator_tag("%"),    // to allow a<b insteadof mustbe a < b
        operator_tag("="),    // 新增赋值运算符
        operator_tag("?"),    // 新增条件赋值运算符
        punctuation_tag(":"), // ?:, {k:v}, arry[a:b:c], allow arr[b:]
        keyword_tag("|"),     //standard io stream pipe
        punctuation_tag("@"),
        punctuation_tag("."),
        punctuation_tag("!"),
    ))(input)
}

fn any_keyword(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("None"),
        keyword_alone_tag("fn"),
        keyword_alone_tag("return"),
        keyword_tag("then"),
        keyword_tag("else"),
        keyword_tag("export"), //set global env
        keyword_tag("let"),
        keyword_tag("for"),
        keyword_tag("while"),
        keyword_tag("if"),
        keyword_tag("in"),
        keyword_tag("del"),
        keyword_tag("match"),
    ))(input)
}
// custrom operator for op overload, such as _*+ , must around with space.
fn custome_operator(input: Input<'_>) -> TokenizationResult<'_> {
    if input.starts_with("_") {
        // 检查前一个字符是否为空格或行首
        if input.previous_char().map_or(true, |c| c.is_whitespace()) {
            let places = input.chars().take_while(char::is_ascii_punctuation).count();
            if places > 1 {
                return Ok(input.split_at(places));
            }
        }
    }
    Err(NOT_FOUND)
}

// parse argument such as ls -l --color=auto ./
fn argument_symbol(input: Input<'_>) -> TokenizationResult<'_> {
    // begin with -+./
    let mut it = input.chars();
    let first_char = it.next().ok_or(NOT_FOUND)?;
    if !matches!(first_char, '-' | '.' | '/') {
        return Err(NOT_FOUND);
    }

    // followed by letter/num
    let next_char = it.next().ok_or(NOT_FOUND)?;
    // dbg!(first_char, next_char);
    let valid = match (first_char, next_char) {
        ('-', '-') => it.next().ok_or(NOT_FOUND)?.is_ascii_alphabetic(),
        ('-', c) => c.is_ascii_alphabetic(),
        ('/', c) => c.is_ascii_alphanumeric(),
        ('.', '/') => true,
        ('.', '.') => true,
        _ => false,
    };
    if valid {
        // prev_char must be blank
        let prev_char = input.previous_char().ok_or(NOT_FOUND)?;
        if prev_char.is_ascii_whitespace() {
            // differ `ls --color` and `a + --b`
            // let prev_prev_char = input.previous_n_char(2).ok_or(NOT_FOUND)?;
            // if prev_prev_char.is_alpha() {
            let len = input
                .chars()
                .take_while(|&c| !c.is_whitespace())
                .map(char::len_utf8)
                .sum();

            // dbg!(len);
            return Ok(input.split_at(len));
            //     }
        }
    }
    Err(NOT_FOUND)
}
// fn string_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
//     // 解析开始引号
//     let (rest_after_start_quote, start_quote_range) = punctuation_tag("\"")(input)?;
//     // 解析内容部分
//     let (rest_after_content, diagnostics) = parse_string_inner(rest_after_start_quote)?;
//     // 解析结束引号或处理EOF
//     let (rest_after_end_quote, end_quote_range) = alt((
//         map(punctuation_tag("\""), |(rest, range)| (rest, range)),
//         map(eof, |_| (input.split_empty(), input.split_empty())),
//     ))(rest_after_content)?;

//     // 计算内容的起始和结束位置
//     let content_start = start_quote_range.end();
//     let content_end = end_quote_range.start();

//     // 生成内容范围，确保有效性
//     let content_range = if content_start <= content_end {
//         // 使用input的方法来分割内容范围
//         let (_, range) = input.split_at(content_start);
//         let (_, range) = range.split_at(content_end - content_start);
//         range
//     } else {
//         // 处理未闭合的情况，取到输入末尾
//         let (_, range) = input.split_at(content_start);
//         range
//     };

//     // 创建Token
//     let token = Token::new(TokenKind::StringLiteral, content_range);
//     Ok((rest_after_end_quote, (token, diagnostics)))
// }
fn string_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // 1. 解析开始引号
    let (rest_after_start, start_quote_range) =
        alt((punctuation_tag("\""), punctuation_tag("'")))(input)?;
    let quote_char = start_quote_range.to_str(input.as_original_str());

    // 2. 解析字符串内容（含转义处理）
    let is_double = quote_char == "\"";
    let (rest_after_content, diagnostics) = parse_string_inner(rest_after_start, is_double)?;

    // 3. 解析结束引号（或EOF）
    let (rest_after_end, _) = alt((
        punctuation_tag(quote_char),
        map(eof, |_| input.split_empty()),
    ))(rest_after_content)?;
    // 4.split
    let (_, content_range) = input.split_until(rest_after_end);
    // 4. 计算内容范围
    // let content_start = start_quote_range.end();
    // let content_end = end_quote_range.start();
    // let (_, content_range) = rest_after_start.split_until(rest_after_content);

    // // 5. 处理未闭合字符串（当end_quote_range为空时）
    // let content_range = if content_start < content_end {
    //     content_range
    // } else {
    //     // 若未闭合，取到输入末尾
    //     let (_, full_range) = input.split_until(rest_after_start);
    //     full_range
    // };

    // 6. 根据引号类型生成TokenKind
    let kind = if is_double {
        TokenKind::StringLiteral
    } else {
        TokenKind::StringRaw
    };

    let token = Token::new(kind, content_range);
    Ok((rest_after_end, (token, diagnostics)))
}

fn number_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // 检查负号 `-` 是否合法（前面是空格或行首）
    let is_negative = input.starts_with("-");
    if is_negative {
        // 检查前一个字符是否为空格或行首
        if !input.previous_char().map_or(false, |c| c.is_whitespace()) {
            return Err(NOT_FOUND); // 前面有非空格字符，不解析为负数
        }
    }

    // skip sign
    let (rest, _) = input.strip_prefix("-").unwrap_or_else(|| input.split_at(0));

    // skip places before the dot
    let (rest, _) = rest
        .strip_prefix("0")
        .or_else(|| {
            let places = rest.chars().take_while(char::is_ascii_digit).count();
            if places > 0 {
                Some(rest.split_at(places))
            } else {
                None
            }
        })
        .ok_or(NOT_FOUND)?;

    // skip the dot, if present
    let (rest, _) = match rest.strip_prefix(".") {
        Some(s) => s,
        None => {
            let (rest, number) = input.split_until(rest);
            let token = Token::new(TokenKind::IntegerLiteral, number);
            return Ok((rest, (token, Diagnostic::Valid)));
        }
    };

    // skip places after the dot
    let places = rest.chars().take_while(char::is_ascii_digit).count();
    if places == 0 {
        let (rest, range) = input.split_until(rest);
        let token = Token::new(TokenKind::FloatLiteral, range);
        return Ok((rest, (token, Diagnostic::InvalidNumber(range))));
    }
    let (rest, _) = rest.split_at(places);

    let (rest, range) = input.split_until(rest);
    let token = Token::new(TokenKind::FloatLiteral, range);
    Ok((rest, (token, Diagnostic::Valid)))
}

fn bool_literal(input: Input<'_>) -> TokenizationResult<'_> {
    alt((keyword_tag("True"), keyword_tag("False")))(input)
}

fn symbol(input: Input<'_>) -> TokenizationResult<'_> {
    let len = input
        .chars()
        .take_while(|&c| is_symbol_char(c))
        .map(char::len_utf8)
        .sum();

    if len == 0 {
        return Err(NOT_FOUND);
    }

    Ok(input.split_at(len))
}

fn whitespace(input: Input<'_>) -> TokenizationResult<'_> {
    let ws_chars = input.chars().take_while(char::is_ascii_whitespace).count();

    if ws_chars == 0 {
        return Err(NOT_FOUND);
    }

    Ok(input.split_at(ws_chars))
}

fn find_prev_char(original_str: &str, current_offset: usize) -> Option<char> {
    // let current_offset = original_str.len() - rest.len();
    let first_c = original_str.get(..current_offset);
    match first_c {
        Some(c) => {
            if !c.is_empty() {
                return c.chars().last();
            }
            // 2. 反向计算前导空白字节长度
            let ws_len = c
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .map(|c| c.len_utf8())
                .sum();

            // 3. 安全切割空白部分
            let ws_start = current_offset.saturating_sub(ws_len);
            let before_nl = original_str.get(..ws_start).unwrap_or("");

            // 4. 获取最后一个非空白字符
            return before_nl.chars().last();
        }
        None => return None,
    }
}

fn linebreak(input: Input<'_>) -> TokenizationResult<'_> {
    // dbg!("--->", input.as_str_slice());

    if let Some((rest, nl_slice)) = input.strip_prefix("\n") {
        // dbg!(nl_slice);
        let original_str = input.as_original_str();

        // 1. 计算换行符的字节位置
        let current_offset = original_str.len().saturating_sub(rest.len() + 1);

        match find_prev_char(original_str, current_offset) {
            Some(c) => {
                // dbg!(c);
                if matches!(c, '{' | '(' | '[' | ',' | '>' | '=' | ';' | '\n' | '\\') {
                    // skip ; and \n because there's already a linebreak parsed.
                    // > is for ->
                    // dbg!("=== skip ");
                    return Err(NOT_FOUND);
                }
            }
            // 读取前面字符失败，跳过
            None => return Err(NOT_FOUND),
        }
        // dbg!("---> LineBreak ");

        Ok((rest, nl_slice))
    } else if let Some((rest, matched)) = input.strip_prefix(";") {
        Ok((rest, matched))
    } else {
        Err(NOT_FOUND)
    }
}
// 新增续行符解析函数
fn line_continuation(input: Input<'_>) -> TokenizationResult<'_> {
    if let Some((rest, matched)) = input.strip_prefix("\\\n") {
        // println!("rest={},matched=", rest, matched);
        // // dbg!(rest, matched);
        Ok((rest, matched))
    } else {
        Err(NOT_FOUND)
    }
}
// 新增行继续符识别逻辑
// fn line_continuation(input: Input<'_>) -> TokenizationResult<'_> {
//     if let Some((rest, _)) = input.strip_prefix("\\") {
//         // 消费后续所有空白（包括换行符）
//         let ws = rest.chars().take_while(char::is_ascii_digit).count();
//         let (rest, _) = rest.split_at(ws);
//         Ok((rest, input.split_at(1).1))
//     } else {
//         Err(NOT_FOUND)
//     }
// }
fn comment(input: Input<'_>) -> TokenizationResult<'_> {
    if input.starts_with('#') {
        let len = input
            .chars()
            .take_while(|&c| !matches!(c, '\r' | '\n'))
            .map(char::len_utf8)
            .sum();

        Ok(input.split_at(len))
    } else {
        Err(NOT_FOUND)
    }
}

fn parse_string_inner(
    input: Input<'_>,
    is_double_quote: bool,
) -> TokenizationResult<'_, Diagnostic> {
    let mut rest = input;
    let mut errors = Vec::new();
    if is_double_quote {
        loop {
            match rest.chars().next() {
                Some('"') | None => break,
                Some('\\') => {
                    let (r, diagnostic) = parse_escape(rest)?;
                    rest = r;
                    if let Diagnostic::InvalidStringEscapes(ranges) = diagnostic {
                        errors.push(ranges[0]);
                    }
                }
                Some(ch) => rest = rest.split_at(ch.len_utf8()).0,
            }
        }
    } else {
        loop {
            match rest.chars().next() {
                Some('\'') | None => break,
                Some(ch) => rest = rest.split_at(ch.len_utf8()).0,
            }
        }
    }

    let diagnostic = match errors.is_empty() {
        true => Diagnostic::Valid,
        false => Diagnostic::InvalidStringEscapes(errors.into_boxed_slice()),
    };

    Ok((rest, diagnostic))
}

fn parse_escape(input: Input<'_>) -> TokenizationResult<'_, Diagnostic> {
    fn parse_hex_digit(input: Input<'_>) -> TokenizationResult<'_> {
        input
            .chars()
            .next()
            .filter(char::is_ascii_hexdigit)
            .ok_or(NOT_FOUND)?;

        Ok(input.split_at(1))
    }

    let (rest, _) = punctuation_tag("\\")(input)?;

    let mut parser1 = alt((
        punctuation_tag("\""),
        punctuation_tag("\\"),
        punctuation_tag("b"),
        punctuation_tag("f"),
        punctuation_tag("n"),
        punctuation_tag("r"),
        punctuation_tag("t"),
    ));
    if let Ok((rest, _)) = parser1(rest) {
        return Ok((rest, Diagnostic::Valid));
    }

    let mut parser2 = tuple((
        punctuation_tag("u{"),
        fold_many_m_n(
            1,
            5,
            parse_hex_digit,
            || None::<StrSlice>,
            |a, b| match a {
                Some(a) => Some(a.join(b)),
                None => Some(b),
            },
        ),
    ));

    let rest = match parser2(rest) {
        Ok((rest, (_, range))) => {
            let range = range.unwrap();
            let hex = range.to_str(input.as_original_str());
            let code_point = u32::from_str_radix(hex, 16).unwrap();
            if char::try_from(code_point).is_ok() {
                rest
            } else {
                let ranges = vec![range].into_boxed_slice();
                return Ok((rest, Diagnostic::InvalidStringEscapes(ranges)));
            }
        }
        Err(_) => {
            let (rest, range) = input.split_saturating(2);
            let ranges = vec![range].into_boxed_slice();
            return Ok((rest, Diagnostic::InvalidStringEscapes(ranges)));
        }
    };

    match punctuation_tag("}")(rest) {
        Ok((rest, _)) => Ok((rest, Diagnostic::Valid)),
        Err(_) => {
            let (rest, range) = input.split_until(rest);
            let ranges = vec![range].into_boxed_slice();
            Ok((rest, Diagnostic::InvalidStringEscapes(ranges)))
        }
    }
}

/// Parses a word that contains characters which can also appear in a symbol.
///
/// This parser ensures that the word is *not* immediately followed by symbol characters.
fn keyword_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| !rest.starts_with(is_symbol_char))
            .ok_or(NOT_FOUND)
    }
}
/// This parser ensures that the word is *not* immediately followed by whitespace.
fn keyword_alone_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}
/// This parser ensures that the word is *not* immediately followed by punctuation.
fn operator_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| unsafe {
                if STRICT {
                    !rest.starts_with(is_symbol_char)
                } else {
                    // match keyword {
                    //     "-" => rest.starts_with(|c: char| c.is_whitespace() || c.is_numeric()),
                    // _ =>
                    rest.starts_with(|c: char| c.is_whitespace() || !c.is_ascii_punctuation())
                    // }
                }
            })
            .ok_or(NOT_FOUND)
    }
}

/// Parses a word that is allowed to be immediately followed by symbol characters.
///
/// This is essentially the same as `nom::bytes::complete::tag`, but with different lifetimes:
/// If the provided string has a 'static lifetime, so does the returned string.
fn punctuation_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| input.strip_prefix(punct).ok_or(NOT_FOUND)
}

/// Checks whether the character is allowed in a symbol.
fn is_symbol_char(c: char) -> bool {
    macro_rules! special_char_pattern {
        () => {
            '_' | '~' | '?' | '&' | '#' | '^' | '$' | '-' | '/'
        };
        // add - / back because it's used so offen in cmd string. "connman-gtk"
        // remove + - /  %  > < to allow non space operator such as a+1
        // remove : to use in dict
        // remove . for dict use. but filename ?
        // $ to use as var prefix, compatil with bash
    }

    static ASCII_SYMBOL_CHARS: [bool; 128] = {
        let mut array = [false; 128];
        let mut i = 0u8;

        while i < 128 {
            array[i as usize] = matches!(
                i as char,
                'a'..='z' | 'A'..='Z' | '0'..='9' | special_char_pattern!()
            );
            i += 1;
        }

        array
    };

    if c.is_ascii() {
        ASCII_SYMBOL_CHARS[c as usize]
    } else {
        false
        // currently only ASCII identifiers are supported :/
    }
}

pub(crate) fn parse_tokens(mut input: Input<'_>) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    loop {
        match parse_token(input) {
            Err(_) => break,
            Ok((new_input, (token, diagnostic))) => {
                input = new_input;
                tokens.push(token);
                diagnostics.push(diagnostic);
            }
        }
    }
    if !input.is_empty() {
        diagnostics.push(Diagnostic::NotTokenized(input.as_str_slice()))
    }
    //dbg!(input, &tokens);
    (tokens, diagnostics)
}

pub fn tokenize(input: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let str = input.into();
    let input = Input::new(&str);
    parse_tokens(input)
}
