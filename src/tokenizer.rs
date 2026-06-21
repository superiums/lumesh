use crate::tokens::{Input, Token, TokenKind};
use crate::with_cfm_enabled;
use detached_str::StrSlice;
use nom::{IResult, branch::alt, error::ParseError};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NotFoundError;

const NOT_FOUND: nom::Err<NotFoundError> = nom::Err::Error(NotFoundError);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    Valid,
    InvalidNumber(StrSlice),
    IllegalChar(StrSlice),
    NotTokenized(StrSlice),
    UnterminatedString(StrSlice),
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

/// Context based on the previous token's last character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Start,
    Space,
    Word,
    Open,
}

impl Ctx {
    fn after_token(token: &Token, original: &str) -> Self {
        let last_char = token.range.to_str(original).chars().next_back();
        match token.kind {
            TokenKind::Whitespace | TokenKind::LineBreak | TokenKind::Comment => Ctx::Space,
            _ => match last_char {
                Some(c) if c.is_ascii_whitespace() => Ctx::Space,
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => Ctx::Word,
                Some(c) if matches!(c, ')' | ']' | '}' | '\'' | '"' | '`') => Ctx::Word,
                _ => Ctx::Open,
            },
        }
    }
}

fn parse_token_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    let first = match input.chars().next() {
        Some(c) => c,
        None => return Err(NOT_FOUND),
    };

    macro_rules! m {
        ($p:expr, $k:expr) => {
            map_valid_token($p, $k)(input)
        };
    }

    match first {
        ' ' | '\t' => m!(whitespace, TokenKind::Whitespace),

        '\n' | ';' => m!(linebreak, TokenKind::LineBreak),

        '\\' => {
            if let Ok(r) = m!(line_continuation, TokenKind::Whitespace) {
                return Ok(r);
            }
            m!(symbol, TokenKind::Symbol)
        }

        '#' => m!(comment, TokenKind::Comment),

        '"' | '\'' | '`' => string_literal(input),

        '0'..='9' => number_literal(input),

        '.' => dot_dispatch(input, ctx),

        '-' => minus_dispatch(input, ctx),

        '(' | ')' | '[' | ']' | '{' | '}' | ',' => dispatch_paren(input, ctx, first),

        'H' | 'M' | 'S' => try_map_or_symbol(input, ctx, first),

        '%' if input.len() > 1 => {
            let next = input.chars().nth(1).unwrap();
            if next == '{' {
                m!(punctuation_tag("%{"), TokenKind::Punctuation)
            } else {
                operator_or_symbol(input, ctx, first)
            }
        }

        '+' | '=' => operator_or_symbol(input, ctx, first),

        '!' => bang_dispatch(input, ctx),

        '?' => question_dispatch(input, ctx),

        '$' => alt((
            map_valid_token(prefix_tag("$"), TokenKind::OperatorPrefix),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),

        '|' | '&' | '^' | '*' | '/' | '<' | '>' | ':' | '~' | '@' => {
            operator_or_symbol(input, ctx, first)
        }

        '_' => underscore_dispatch(input, ctx),

        'a'..='z' | 'A'..='Z' => alpha_dispatch(input, ctx),

        c if !c.is_ascii() => m!(non_ascii, TokenKind::StringRaw),

        _ => {
            let (rest, range) = input.split_at(first.len_utf8());
            Ok((
                rest,
                (
                    Token::new(TokenKind::Symbol, range),
                    Diagnostic::IllegalChar(range),
                ),
            ))
        }
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

// ========== Context-aware dispatch helpers ==========

fn dispatch_paren(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word if matches!(first, '(' | '[') => alt((
            map_valid_token(punctuation_tag("("), TokenKind::OperatorPostfix),
            map_valid_token(punctuation_tag("["), TokenKind::OperatorPostfix),
            map_valid_token(any_punctuation, TokenKind::Punctuation),
        ))(input),
        _ => map_valid_token(any_punctuation, TokenKind::Punctuation)(input),
    }
}

fn dot_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(infix_tag("...="), TokenKind::OperatorInfix),
            map_valid_token(infix_tag("..."), TokenKind::OperatorInfix),
            map_valid_token(infix_tag("..="), TokenKind::OperatorInfix),
            map_valid_token(infix_tag(".."), TokenKind::OperatorInfix),
            map_valid_token(punctuation_tag("."), TokenKind::OperatorPostfix),
        ))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punct_seq_tag(".."), TokenKind::Operator),
            map_valid_token(prefix_tag("."), TokenKind::OperatorPrefix),
            map_valid_token(argument_symbol, TokenKind::StringRaw),
            number_literal,
            map_valid_token(keyword_alone_or_end("."), TokenKind::ValueSymbol),
            map_valid_token(keyword_alone_or_end(".."), TokenKind::ValueSymbol),
        ))(input),
    }
}

fn minus_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(keyword_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            map_valid_token(keyword_tag("-"), TokenKind::Operator),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        Ctx::Start | Ctx::Space => alt((
            map_valid_token(keyword_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            map_valid_token(argument_symbol, TokenKind::StringRaw),
            map_valid_token(prefix_tag("-"), TokenKind::OperatorPrefix),
            number_literal,
            map_valid_token(keyword_tag("-"), TokenKind::Operator),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        Ctx::Open => alt((
            map_valid_token(prefix_tag("-"), TokenKind::OperatorPrefix),
            number_literal,
            map_valid_token(keyword_tag("-"), TokenKind::Operator),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
    }
}

fn bang_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(punctuation_tag("!=="), TokenKind::Operator),
            map_valid_token(punctuation_tag("!="), TokenKind::Operator),
            map_valid_token(keyword_tag("!~:"), TokenKind::Operator),
            map_valid_token(postfix_break_tag("!"), TokenKind::OperatorPostfix),
            map_valid_token(any_punctuation, TokenKind::Punctuation),
        ))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punctuation_tag("!=="), TokenKind::Operator),
            map_valid_token(punctuation_tag("!="), TokenKind::Operator),
            map_valid_token(keyword_tag("!~:"), TokenKind::Operator),
            map_valid_token(prefix_tag("!"), TokenKind::OperatorPrefix),
            map_valid_token(any_punctuation, TokenKind::Punctuation),
        ))(input),
    }
}

fn question_dispatch(input: Input<'_>, _ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    alt((
        map_valid_token(question_operator, TokenKind::Operator),
        map_valid_token(operator_tag("?"), TokenKind::Operator),
        map_valid_token(symbol, TokenKind::Symbol),
    ))(input)
}

fn underscore_dispatch(input: Input<'_>, _ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    alt((
        map_valid_token(among_punc_tag("_"), TokenKind::ValueSymbol),
        map_valid_token(symbol, TokenKind::Symbol),
    ))(input)
}

fn alpha_dispatch(input: Input<'_>, _ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    alt((
        map_valid_token(any_keyword, TokenKind::Keyword),
        map_valid_token(value_symbol, TokenKind::ValueSymbol),
        string_literal, // r'...' or t'...'
        map_valid_token(symbol, TokenKind::Symbol),
    ))(input)
}

fn operator_or_symbol(
    input: Input<'_>,
    ctx: Ctx,
    _first: char,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(long_operator, TokenKind::Operator),
            map_valid_token(short_operator, TokenKind::Operator),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        _ => alt((
            map_valid_token(long_operator, TokenKind::Operator),
            map_valid_token(argument_symbol, TokenKind::StringRaw),
            map_valid_token(short_operator, TokenKind::Operator),
            map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
    }
}

fn try_map_or_symbol(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // H{, M{, S{ — check if followed by {
    let peek = if input.len() > 1 {
        input.chars().nth(1)
    } else {
        None
    };
    match peek {
        Some('{') => map_valid_token(
            punctuation_tag(&format!("{first}{{")),
            TokenKind::Punctuation,
        )(input),
        _ => alpha_dispatch(input, ctx),
    }
}

// =====================================================

fn any_punctuation(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        punctuation_tag(","),
        punctuation_tag("("),
        punctuation_tag(")"),
        punctuation_tag("["),
        punctuation_tag("]"),
        punctuation_tag("{"),
        punctuation_tag("}"),
        punctuation_tag("H{"), //hashMap
        punctuation_tag("M{"), //bMap
        punctuation_tag("S{"), //bSet
        punctuation_tag("%{"), //explicit block
    ))(input)
}

fn long_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("=>"), //for match
        alt((
            punctuation_tag("!=="),
            punctuation_tag("==="),
            punctuation_tag("!="),
            punctuation_tag("=="), //to allow a==b
            punctuation_tag(">="),
            punctuation_tag("<="),
            keyword_tag("!~:"),
            keyword_tag("~:"),
        )),
        keyword_tag("&&"),
        keyword_tag("||"),
        keyword_tag("|>"), //dispatch pipe
        keyword_tag("|^"), //pty pipe
        keyword_tag("<<"),
        keyword_tag(">!"),
        keyword_tag(">>"),
        operator_tag("+="),
        operator_tag("-="),
        operator_tag("*="),
        operator_tag("/="),
        keyword_tag(":="),
        punctuation_tag("->"), // `->foo` is also a valid symbol
        question_operator,
    ))(input)
}

/// Matches `?`-prefixed multi-char operators (`?+`, `?.`, `??`, `?>`, `?!`, `?:`, `?~`).
fn question_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("?+"),
        keyword_tag("?."),
        keyword_tag("??"),
        keyword_tag("?>"),
        keyword_tag("?!"),
        keyword_tag("?:"),
        keyword_tag("?~"),
    ))(input)
}

fn short_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("-"), // not followed by symbol, allow punct follow.
        keyword_tag("/"),
        keyword_tag("|"),  //standard io stream pipe
        operator_tag("<"), // not followed by punct, allow space,symbol like.
        operator_tag(">"),
        operator_tag("*"),
        operator_tag("%"),
        operator_tag("^"), //math power
        punctuation_tag("+"),
        punctuation_tag("="), // allow all.
        operator_tag("?"),
        punctuation_tag(":"), // ?:, {k:v}, arry[a:b:c], allow arr[b:]
    ))(input)
}

fn any_keyword(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_alone_tag("let"),
        keyword_alone_tag("set"),
        keyword_alone_tag("alias"),
        keyword_alone_tag("export"),
        keyword_alone_tag("if"),
        // keyword_tag("then"),
        keyword_tag("else"),
        keyword_alone_tag("fn"),
        keyword_alone_tag("match"),
        keyword_alone_tag("for"),
        keyword_alone_tag("in"),
        keyword_alone_tag("while"),
        keyword_alone_tag("loop"),
        keyword_tag("break"),
        keyword_tag("return"),
        keyword_alone_tag("del"),
        keyword_alone_tag("use"),
    ))(input)
}

/// Matches a sequence of ASCII punctuation starting with `punct` (length ≥ 2).
///
/// Used for `..` range operator at expression-start/non-word positions.
fn punct_seq_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if input.starts_with(punct) {
            let places = input.chars().take_while(char::is_ascii_punctuation).count();
            if places > 1 {
                return Ok(input.split_at(places));
            }
        }
        Err(NOT_FOUND)
    }
}

/// Consumes a path-like argument sequence starting with `punct` (e.g., `--flag`, `-o`, `/path`).
///
/// Eats characters until whitespace or break chars, handling escape sequences.
/// Only returns match if consumed length > 1 (to distinguish from operators).
fn path_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if input.starts_with(punct) {
            let mut chars = input.chars();
            let mut places = 0;

            while let Some(c) = chars.next() {
                if c == '\\' {
                    // 检查转义空格
                    if let Some(next_c) = chars.next() {
                        #[cfg(windows)]
                        if matches!(&next_c, ' ') {
                            places += c.len_utf8() + next_c.len_utf8();
                            continue; // 跳过转义空格
                        }
                        #[cfg(unix)]
                        if matches!(&next_c, ' ' | '"' | '\'') {
                            places += c.len_utf8() + next_c.len_utf8();
                            continue; // 跳过转义空格
                        }
                        places += next_c.len_utf8();
                    }
                } else if c.is_ascii_whitespace() {
                    break; // 遇到普通空格，结束
                } else if matches!(&c, ';' | '`' | ')' | ']' | '}' | '|' | '>') {
                    break; // 遇到特殊字符，结束
                }

                places += c.len_utf8(); // 累加字符长度
            }

            if places > 1 {
                return Ok(input.split_at(places));
            }

            // 允许单字符路径，但仅在它们是输入的结尾时
            if places + punct.len() >= input.len() {
                return Ok(input.split_at(places));
            }
        }
        Err(NOT_FOUND)
    }
}

#[cfg(windows)]
fn win_abpath_tag(_: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        let mut it = input.chars();
        if input.len() > 1
            && it.next().map_or(false, |c| c.is_ascii_uppercase())
            && it.next().map_or(false, |c| c == ':')
        {
            let places = input
                .chars()
                .take_while(|&c| {
                    !c.is_whitespace() && !matches!(&c, ';' | '`' | ')' | ']' | '}' | '|' | '>')
                })
                .count();
            if places > 1 {
                return Ok(input.split_at(places));
            }
            if places >= input.len()
            {
                return Ok(input.split_at(input.len()));
            }
        }
        Err(NOT_FOUND)
    }
}

// parse argument such as ipconfig /all; C:\
#[cfg(windows)]
fn argument_symbol(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        // allow unix style
        path_tag("../"),
        path_tag("./"),
        path_tag("*/"),
        path_tag("**/"),
        // win style
        path_tag("--"),
        path_tag("-"),
        win_abpath_tag(":"),
        path_tag("..\\"),
        path_tag(".\\"),
        path_tag("~"),
        path_tag("*\\"),
        path_tag("**\\"),
        path_tag("*."),
        alt((
            path_tag("http:"),
            path_tag("https:"),
            path_tag("ftp:"),
            path_tag("ftps:"),
            path_tag("file:"),
        )),
        keyword_alone_or_end("."),
        keyword_alone_or_end(".."),
        keyword_alone_or_end("&-"),
        keyword_alone_or_end("&?"),
        keyword_alone_or_end("&+"),
        keyword_alone_or_end("&."),
    ))(input)
}
// parse argument such as ls -l --color=auto ./
#[cfg(unix)]
fn argument_symbol(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        path_tag("--"),
        path_tag("-"),
        path_tag("/"),
        path_tag("../"),
        path_tag("./"),
        path_tag("~"),
        path_tag("*/"),
        path_tag("**/"),
        path_tag("*."),
        path_tag("http:"),
        path_tag("https:"),
        path_tag("ftp:"),
        path_tag("ftps:"),
        path_tag("file:"),
        // keyword_alone_or_end("~"),
        // keyword_alone_or_end("/"),
        keyword_alone_or_end("."),
        keyword_alone_or_end(".."),
        keyword_alone_or_end("&-"),
        keyword_alone_or_end("&?"),
        keyword_alone_or_end("&+"),
        keyword_alone_or_end("&."),
    ))(input)
}

fn string_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // 1. 解析开始引号
    let (rest_after_start, start_quote_range) = alt((
        punctuation_tag("\""),
        punctuation_tag("'"),
        punctuation_tag("`"),
        punctuation_tag("r'"), //regex
        punctuation_tag("t'"), //time
    ))(input)?;
    let quote_char = start_quote_range.to_str(input.as_original_str());
    let q_char = match quote_char.len() {
        1 => quote_char,
        _ => "'",
    };

    // 2. 解析字符串内容（含转义处理）
    let (rest_after_content, diagnostics) =
        parse_string_inner(rest_after_start, q_char.chars().next().unwrap())?;

    let (rest, content_range) = match punctuation_tag(q_char)(rest_after_content) {
        Ok((rest_after_end, _)) => input.split_until(rest_after_end),
        Err(_) => input.split_until(rest_after_content),
    };

    let kind = match quote_char {
        "'" => TokenKind::StringRaw,
        "\"" => TokenKind::StringLiteral,
        "`" => TokenKind::StringTemplate,
        "r'" => TokenKind::Regex,
        "t'" => TokenKind::Time,
        _ => unreachable!(),
    };
    let token = Token::new(kind, content_range);
    Ok((rest, (token, diagnostics)))
}

fn number_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
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

    // skip .. only take number
    if rest.starts_with("..") {
        let (rest, number) = input.split_until(rest);
        let token = Token::new(TokenKind::IntegerLiteral, number);
        return Ok((rest, (token, Diagnostic::Valid)));
    }

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

fn value_symbol(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("true"),
        keyword_tag("false"),
        keyword_tag("none"),
        among_punc_tag("_"),
    ))(input)
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

fn non_ascii(input: Input<'_>) -> TokenizationResult<'_> {
    // 处理UTF-8连续非ASCII字符
    let len = input
        .chars()
        .take_while(|&c| !c.is_ascii())
        .map(char::len_utf8)
        .sum();

    if len == 0 {
        return Err(NOT_FOUND);
    }
    Ok(input.split_at(len))
}

fn linebreak(mut input: Input<'_>) -> TokenizationResult<'_> {
    // dbg!("--->", input.as_str_slice(),input.first());
    let ws_chars = input.chars().take_while(|c| *c == ' ').count();
    if ws_chars > 0 {
        (input, _) = input.split_at(ws_chars);
    }

    #[cfg(windows)]
    if let Some((rest, nl_slice)) = input.strip_prefix("\r\n") {
        return Ok((rest, nl_slice));
    }

    if let Some((rest, nl_slice)) = input.strip_prefix("\n") {
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
        Ok((rest, matched))
    } else {
        #[cfg(windows)]
        if let Some((rest, matched)) = input.strip_prefix("\\\r\n") {
            return Ok((rest, matched));
        }
        Err(NOT_FOUND)
    }
}

fn comment(input: Input<'_>) -> TokenizationResult<'_> {
    if input.starts_with("#") {
        let len = input
            .chars()
            .take_while(|&c| !matches!(c, '\n' | '\r'))
            .map(char::len_utf8)
            .sum();

        Ok(input.split_at(len))
    } else {
        Err(NOT_FOUND)
    }
}

fn parse_string_inner(input: Input<'_>, quote_char: char) -> TokenizationResult<'_, Diagnostic> {
    let mut rest = input;
    let start_range = input.as_str_slice();

    loop {
        let next_char = rest.chars().next();
        match next_char {
            Some(c) if c == quote_char => break,
            None => {
                return Ok((rest, Diagnostic::UnterminatedString(start_range)));
            }
            Some('\\') => {
                // 统一处理：消耗反斜杠及其后一个字符
                // 所有转义序列以原始文本保留在 content 中，由 parser 层处理
                let rest_after_bs = rest.split_at(1).0;
                match rest_after_bs.chars().next() {
                    Some(_) => {
                        rest = rest_after_bs.split_at(1).0;
                    }
                    None => {
                        return Ok((rest, Diagnostic::UnterminatedString(start_range)));
                    }
                }
            }
            Some(ch) => {
                rest = rest.split_at(ch.len_utf8()).0;
            }
        }
    }

    Ok((rest, Diagnostic::Valid))
}

/// Matches a literal string prefix without any continuation restrictions.
fn punctuation_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| input.strip_prefix(punct).ok_or(NOT_FOUND)
}

/// Matches a keyword/operator that must NOT be followed by symbol characters.
///
/// Used for multi-char operators (`&&`, `||`, `=>`, `::`, `<<`, etc.) where
/// the operator should not merge into a longer symbol.
fn keyword_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| !rest.starts_with(is_symbol_char))
            .ok_or(NOT_FOUND)
    }
}
/// Matches a keyword that must be followed by whitespace (not end-of-input).
fn keyword_alone_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}
/// Matches a keyword that must be followed by whitespace OR end-of-input.
fn keyword_alone_or_end(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.is_empty() || rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}
/// Matches an operator that must NOT be followed by ASCII punctuation.
///
/// Used for single-char operators (`+`, `=`, `<`, etc.) to prevent them from
/// merging into longer operator sequences (e.g. `+` vs `+=`).
fn operator_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.starts_with(|c: char| c.is_whitespace() || !c.is_ascii_punctuation())
            })
            .ok_or(NOT_FOUND)
    }
}
/// Matches a token that must be surrounded by whitespace or punctuation (not letters).
///
/// Used for `_` to distinguish standalone `_` value from `_` within a symbol.
fn among_punc_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.is_empty()
                    || rest.starts_with(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            })
            .ok_or(NOT_FOUND)
    }
}
/// Matches a prefix operator that must be followed by a value-start character.
///
/// After stripping the prefix, checks the rest starts with alphanumeric, `(`, `[`, `{`, or `$`.
fn prefix_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.starts_with(|c: char| {
                    c.is_ascii_alphanumeric() || matches!(&c, '(' | '[' | '{' | '$')
                })
            })
            .ok_or(NOT_FOUND)
    }
}
fn infix_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.starts_with(|c: char| {
                    c.is_ascii_alphanumeric() || matches!(&c, '(' | '_' | '-')
                })
            })
            .ok_or(NOT_FOUND)
    }
}
/// Matches a postfix operator that must be followed by whitespace or end-of-input.
///
/// Used for postfix `!` and `^` when they should not merge with following characters.
fn postfix_break_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_whitespace())
            })
            .ok_or(NOT_FOUND)
    }
}
/// Checks whether the character is allowed in a symbol.
fn is_symbol_char(c: char) -> bool {
    macro_rules! special_char_pattern {
        () => {
            '_' | '~' | '?' | '&' | '#' | '$' | '-' | '/' | '\\'
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
    // 检查是否为单行命令模式
    if is_cfm_mode(input) {
        return parse_command_tokens(input);
    }

    // 多行输入使用正常模式
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ctx = Ctx::Start;
    // skip multiline mode prefix
    if let Ok((new_input, (token, diagnostic))) =
        map_valid_token(punctuation_tag(":"), TokenKind::Comment)(input)
    {
        input = new_input;
        ctx = Ctx::after_token(&token, input.as_original_str());
        tokens.push(token);
        diagnostics.push(diagnostic);
    }

    // go
    loop {
        match parse_token_dispatch(input, ctx) {
            Err(_) => break,
            Ok((new_input, (token, diagnostic))) => {
                ctx = Ctx::after_token(&token, input.as_original_str());
                input = new_input;
                tokens.push(token);
                diagnostics.push(diagnostic);
            }
        }
    }
    if !input.is_empty() {
        diagnostics.push(Diagnostic::NotTokenized(input.as_str_slice()))
    }
    (tokens, diagnostics)
}

pub fn tokenize(input: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let str = input.into();
    let input = Input::new(&str);
    parse_tokens(input)
}

/// CFM: command first mode
fn is_cfm_mode(input: Input<'_>) -> bool {
    with_cfm_enabled(|cfm_enabled| match input.starts_with(">") {
        true => true,
        false => match input.starts_with(":") {
            true => false,
            false => !input.contains("\n") && cfm_enabled,
        },
    })
}

fn parse_command_tokens(mut input: Input<'_>) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ctx = Ctx::Start;

    if let Ok((new_input, (token, diagnostic))) =
        map_valid_token(punctuation_tag(">"), TokenKind::Comment)(input)
    {
        input = new_input;
        ctx = Ctx::after_token(&token, input.as_original_str());
        tokens.push(token);
        diagnostics.push(diagnostic);
    }

    while !input.is_empty() {
        match parse_command_token(input, ctx) {
            Ok((new_input, (token, diagnostic))) => {
                ctx = Ctx::after_token(&token, input.as_original_str());
                input = new_input;
                tokens.push(token);
                diagnostics.push(diagnostic);
            }
            Err(_) => {
                let next = input.chars().next().unwrap();
                let (new_input, range) = input.split_at(next.len_utf8());
                input = new_input;
                let token = Token::new(TokenKind::Symbol, range);
                tokens.push(token);
                diagnostics.push(Diagnostic::IllegalChar(range));
            }
        }
    }
    (tokens, diagnostics)
}

fn parse_command_token(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if input.is_empty() {
        return Err(NOT_FOUND);
    }

    macro_rules! try_parser {
        ($e:expr) => {
            if let Ok(r) = $e {
                return Ok(r);
            }
        };
    }

    try_parser!(map_valid_token(linebreak, TokenKind::LineBreak)(input));
    try_parser!(string_literal(input));

    if ctx != Ctx::Word {
        try_parser!(map_valid_token(argument_symbol, TokenKind::StringRaw)(
            input
        ));
        try_parser!(map_valid_token(
            cfm_prefix_operator,
            TokenKind::OperatorPrefix
        )(input));
    }

    try_parser!(map_valid_token(cfm_operator, TokenKind::Operator)(input));

    if ctx == Ctx::Word {
        try_parser!(map_valid_token(
            cfm_postfix_operator,
            TokenKind::OperatorPostfix
        )(input));
    }

    try_parser!(map_valid_token(any_punctuation, TokenKind::Punctuation)(
        input
    ));
    try_parser!(map_valid_token(any_keyword, TokenKind::Keyword)(input));
    try_parser!(map_valid_token(whitespace, TokenKind::Whitespace)(input));
    try_parser!(map_valid_token(value_symbol, TokenKind::ValueSymbol)(input));
    try_parser!(map_valid_token(comment, TokenKind::Comment)(input));
    try_parser!(map_valid_token(non_ascii, TokenKind::StringRaw)(input));

    cfm_parse_symbol(input)
}

fn cfm_parse_symbol(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // 读取直到遇到空格、括号或管道符号
    let mut chars = input.chars();
    let mut length = 0;
    // `=` is used for var asign: IFS='';xx
    while let Some(c) = chars.next() {
        if c.is_ascii_whitespace()
            || matches!(
                &c,
                '=' | '>'
                    | '('
                    | '['
                    | '{'
                    | '^'
                    | '$'
                    | '!'
                    | '|'
                    | ';'
                    | ')'
                    | ']'
                    | '}'
                    | '.'
                    | ','
            )
            || c.is_control()
        {
            break;
        }
        length += c.len_utf8();
    }

    if length == 0 {
        Err(NOT_FOUND)
    } else {
        let (rest, range) = input.split_at(length);
        let token = Token::new(TokenKind::Symbol, range);
        Ok((rest, (token, Diagnostic::Valid)))
    }
}

fn cfm_prefix_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        prefix_tag("."), //pipemethod
        prefix_tag("!"), //bool negtive
        prefix_tag("$"), //var
    ))(input)
}

fn cfm_postfix_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        punctuation_tag("."), //chaind call/property
        punctuation_tag("!"), //func call as flat as cmd
        punctuation_tag("^"), //make symbo as cmd
        punctuation_tag("("), //func call
    ))(input)
}

fn cfm_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((cfm_long_operator, cfm_short_operator))(input)
}
fn cfm_short_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("|"),
        punctuation_tag("="), // allow all.
        keyword_alone_tag("+"),
        keyword_alone_tag("?"),
        keyword_alone_tag(":"),
    ))(input)
}

fn cfm_long_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_tag("=>"),
        alt((
            keyword_tag("!~:"),
            keyword_tag("~:"),
            keyword_tag("!~="),
            keyword_tag("~="),
        )),
        keyword_tag("&&"),
        keyword_tag("||"),
        keyword_tag("|>"),
        keyword_tag("|^"),
        keyword_tag("<<"),
        keyword_tag(">!"),
        keyword_tag(">>"),
        keyword_tag(":="),
        punctuation_tag("->"),
        question_operator,
    ))(input)
}
