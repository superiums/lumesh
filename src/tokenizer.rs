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
/// Determines how the next token is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Start, // line start / initial state
    Space, // prev token ended with whitespace (Whitespace/LineBreak/Comment)
    Word,  // prev token ended with alphanumeric, `_`, or closing bracket/quote
    Open,  // other (non-space, non-word symbol)
}

impl Ctx {
    /// Determine the next context based on the current token's ending character.
    /// Whitespace/LineBreak/Comment → Space
    /// Alphanumeric/`_`/closing bracket/quote → Word
    /// Other symbols → Open
    fn after_token(token: &Token, original: &str) -> Self {
        let last_char = token.range.to_str(original).chars().next_back();
        match token.kind {
            TokenKind::Whitespace | TokenKind::LineBreak | TokenKind::Comment => Ctx::Space,
            _ => match last_char {
                Some(c) if c.is_ascii_whitespace() => Ctx::Space,
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => Ctx::Word,
                Some(')' | ']' | '}' | '\'' | '"' | '`') => Ctx::Word,
                _ => Ctx::Open,
            },
        }
    }
}

/// Dispatch tokenization based on the first character and context.
/// Each character triggers a specific parsing path.
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

        ';' => m!(punctuation_tag(";"), TokenKind::LineBreak),
        '\n' => m!(punctuation_tag("\n"), TokenKind::LineBreak),

        '\\' => {
            if let Ok(r) = m!(line_continuation, TokenKind::Whitespace) {
                return Ok(r);
            }
            m!(symbol, TokenKind::Symbol)
        }

        '#' => m!(comment, TokenKind::Comment),

        '"' | '\'' | '`' => string_literal(input),

        '0'..='9' => number_literal(input),

        '.' => dot_dispatch(input, ctx), // context-aware: method call vs path

        '-' => minus_dispatch(input, ctx), // context-aware: negative vs flag vs operator

        '(' | ')' | '[' | ']' | '{' | '}' | ',' => dispatch_paren(input, ctx, first),

        'H' | 'M' | 'S' => try_map_or_symbol(input, ctx, first), // H{ M{ S{ map/set literals

        '%' if input.len() > 1 => {
            let bytes = input.as_ref().as_bytes();
            if bytes.len() > 1 && bytes[1] == b'{' {
                m!(punctuation_tag("%{"), TokenKind::Punctuation) // %{ explicit block
            } else {
                operator_or_symbol(input, ctx, first)
            }
        }

        '+' | '=' => operator_or_symbol(input, ctx, first),

        '!' => bang_dispatch(input, ctx), // context-aware: prefix negate vs postfix call

        '?' => question_dispatch(input, ctx),

        '$' => alt((
            map_valid_token(prefix_tag("$"), TokenKind::OperatorPrefix), // $var
            map_valid_token(symbol, TokenKind::Symbol),                  // $ as symbol
        ))(input),

        '|' | '&' | '^' | '*' | '<' | '>' => operator_or_symbol(input, ctx, first),

        '/' => slash_dispatch(input, ctx),

        '~' => tiled_dispatch(input, ctx),

        ':' => colon_dispatch(input, ctx),

        '@' => at_dispatch(input, ctx),

        '_' => underscore_dispatch(input, ctx), // standalone _ vs _ in symbol

        'a'..='z' | 'A'..='Z' => alpha_dispatch(input, ctx), // keyword / value_symbol / string / symbol

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

/// `(` or `[` after Word context → OperatorPostfix (function call/index)
/// Otherwise → Punctuation (standalone bracket/comma)
fn dispatch_paren(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if ctx == Ctx::Word && matches!(first, '(' | '[') {
        map_valid_token(
            punctuation_tag(&first.to_string()),
            TokenKind::OperatorPostfix,
        )(input)
    } else {
        map_valid_token(any_punctuation, TokenKind::Punctuation)(input)
    }
}

fn tiled_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((map_valid_token(symbol, TokenKind::Symbol),))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(operator_tag("~:"), TokenKind::Operator),
            map_valid_token(operator_tag("~="), TokenKind::Operator),
            map_valid_token(path_tag("~/", true), TokenKind::StringRaw), // ~/ path
            map_valid_token(last_path_tag("~"), TokenKind::StringRaw),   // `ls ~` path at end
                                                                         // map_valid_token(keyword_alone_or_end("~"), TokenKind::Operator), // reverse?
        ))(input),
    }
}

fn slash_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((map_valid_token(symbol, TokenKind::Symbol),))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(path_tag("/", false), TokenKind::StringRaw), // /x path
            map_valid_token(last_path_tag("/"), TokenKind::StringRaw),   // `ls /` path at end
            map_valid_token(keyword_alone_or_end("/"), TokenKind::Operator), // divide
        ))(input),
    }
}

/// `Word` context → `...=`/`...`/`..=`/`..` infix range, or `.` postfix (method call)
/// `Start/Space/Open` context → `..` operator, `.` prefix (pipemethod), argument path, number literal, or standalone `.`/`..`
fn dot_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(infix_tag("...="), TokenKind::OperatorInfix),
            map_valid_token(infix_tag("..."), TokenKind::OperatorInfix),
            map_valid_token(infix_tag("..="), TokenKind::OperatorInfix),
            map_valid_token(infix_tag(".."), TokenKind::OperatorInfix), //range
            map_valid_token(punctuation_tag("."), TokenKind::OperatorPostfix), //call
        ))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punct_seq_tag(".."), TokenKind::Operator), // ..+ customOp
            number_literal,                                            //.5
            map_valid_token(prefix_tag("."), TokenKind::OperatorPrefix), //.method
            map_valid_token(path_tag("./", true), TokenKind::StringRaw),
            map_valid_token(path_tag("../", true), TokenKind::StringRaw),
            map_valid_token(keyword_alone_or_end(".."), TokenKind::StringRaw), // parent path
            map_valid_token(keyword_alone_or_end("."), TokenKind::StringRaw),  // current path
                                                                               // TODO ..3
        ))(input),
    }
}

/// Context-aware `-` dispatch:
/// - Word context: operator only (`-=`, `->`, `-` or symbol) — no prefix/argument
/// - Start/Space: prefix `-` when followed by literal/number/paren/letter (parser distinguishes negation vs flag);
///   `argument_symbol` only for `--` style flags; `number_literal` for bare digits
/// - Open: prefix `-` when followed by literal/number/paren/letter
fn minus_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(punctuation_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            map_valid_token(punctuation_tag("-"), TokenKind::Operator), //a-b as operator
                                                                        // map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        Ctx::Start | Ctx::Space => alt((
            map_valid_token(punctuation_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            // `--flag` style: two dashes → argument symbol
            map_valid_token(whole_word("--"), TokenKind::StringRaw),
            // single `-` followed by literal/number/paren/letter → OperatorPrefix
            map_valid_token(prefix_minus_tag, TokenKind::OperatorPrefix),
            // bare `-` as operator (e.g. `- ` followed by space)
            map_valid_token(punctuation_tag("-"), TokenKind::Operator),
            // map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        Ctx::Open => alt((
            // single `-` followed by literal/number/paren/letter → OperatorPrefix
            map_valid_token(prefix_minus_tag, TokenKind::OperatorPrefix),
            // number_literal,
            map_valid_token(punctuation_tag("-"), TokenKind::OperatorPrefix), // +(-5); a[-1]
                                                                              // map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
    }
}

/// Matches `-` as a prefix operator when followed by a literal/number/paren/identifier.
/// This lets the parser decide: `-42` → negation, `-arg` → flag, `-(expr)` → grouped negation.
fn prefix_minus_tag(input: Input<'_>) -> TokenizationResult<'_> {
    input
        .strip_prefix("-")
        .filter(|(rest, _)| {
            rest.starts_with(|c: char| {
                c.is_ascii_alphanumeric() || matches!(c, '(' | '[' | '{' | '.')
            })
        })
        .ok_or(NOT_FOUND)
}

/// Context-aware `!` dispatch:
/// - Word context: `!=`/`!==` comparison, `!~:` pattern match, or `!` postfix (flat call)
/// - Start/Space/Open: same operators but `!` as prefix (negation) instead of postfix
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
        regex_literal,
        time_literal,
        map_valid_token(protocols, TokenKind::StringRaw),
        map_valid_token(symbol, TokenKind::Symbol),
    ))(input)
}

/// Generic dispatch for operators (`+`, `=`, `|`, `&`, `*`, `/`, `<`, `>`, `:`, `~`, `@`, `^`):
/// - Word context: operator or symbol (no argument matching)
/// - Non-word context: also tries argument_symbol before short_operator
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
            // map_valid_token(argument_symbol, TokenKind::StringRaw),
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
    let bytes = input.as_ref().as_bytes();
    if bytes.len() > 1 && bytes[1] == b'{' {
        map_valid_token(
            punctuation_tag(&format!("{first}{{")),
            TokenKind::Punctuation,
        )(input)
    } else {
        alpha_dispatch(input, ctx)
    }
}

/// `::` module call infix operator (e.g. `mod::func`).
/// Requires Word context (preceded by identifier) and followed by identifier.
fn colon_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(keyword_tag("::"), TokenKind::OperatorInfix),
            map_valid_token(operator_tag(":"), TokenKind::Punctuation), //{k:v} a?b:c
        ))(input),
        _ => map_valid_token(operator_tag(":"), TokenKind::Operator)(input),
    }
}

/// `@` as prefix operator for decorators (e.g. `@deco`).
/// Requires non-Word context (standalone `@` before identifier).
fn at_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => {
            // Inside an identifier, `@` is not valid — fall through to symbol
            map_valid_token(symbol, TokenKind::Symbol)(input)
        }
        _ => alt((
            map_valid_token(prefix_tag("@"), TokenKind::OperatorPrefix), // @deco, @(expr)
                                                                         // map_valid_token(operator_tag("@"), TokenKind::Operator),
        ))(input),
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
        // comparison / equality
        keyword_tag("=>"),
        punctuation_tag("!=="),
        punctuation_tag("==="),
        punctuation_tag("!="),
        punctuation_tag("=="),
        punctuation_tag(">="),
        punctuation_tag("<="),
        keyword_tag("!~:"),
        keyword_tag("~:"),
    ))(input)
    .or_else(|_| {
        alt((
            // logical / pipe
            keyword_tag("&&"),
            keyword_tag("||"),
            keyword_tag("|>"),
            keyword_tag("|^"),
            keyword_tag("<<"),
            keyword_tag(">!"),
            keyword_tag(">>"),
            // assignment
            operator_tag("+="),
            operator_tag("-="),
            operator_tag("*="),
            operator_tag("/="),
            keyword_tag(":="),
            // arrow
            punctuation_tag("->"),
            // ?-operators
            question_operator,
        ))(input)
    })
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
        keyword_tag("-"),
        keyword_tag("/"),
        keyword_tag("|"),
        operator_tag("<"),
        operator_tag(">"),
        operator_tag("*"),
        operator_tag("%"),
        operator_tag("^"),
        punctuation_tag("+"),
        punctuation_tag("="),
        operator_tag("?"),
        punctuation_tag(":"),
    ))(input)
}

fn any_keyword(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        keyword_alone_tag("let"),
        keyword_alone_tag("set"),
        keyword_alone_tag("alias"),
        keyword_alone_tag("export"),
        keyword_alone_tag("if"),
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
/// Used for `..` custom operator at expression-start/non-word positions.
fn punct_seq_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if input.starts_with(punct) {
            let places = input.chars().take_while(char::is_ascii_punctuation).count();
            if places > 1 + punct.len() {
                return Ok(input.split_at(places));
            }
        }
        Err(NOT_FOUND)
    }
}

/// Path/argument prefix matcher: consumes a `punct` prefix then scans forward
/// until a delimiter or end of input.
///
/// Delimiters: whitespace, `;`, `` ` ``, `)`, `]`, `}`, `|`, `>`.
/// Escape: `\X` skips the next byte (on Unix, `\ ` / `\`"` / `\''` are escaped pairs).
fn path_tag(punct: &str, alone_ok: bool) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if !input.starts_with(punct) {
            return Err(NOT_FOUND);
        }

        let bytes = input.as_ref().as_bytes();
        let prefix_len = punct.len();
        let mut i = prefix_len;

        while i < bytes.len() {
            let b = bytes[i];

            // escape sequence: skip backslash + next byte
            if b == b'\\' {
                i += 1; // skip backslash
                if i < bytes.len() {
                    i += 1; // skip escaped byte
                    continue;
                }
                // backslash at end of input — treat as literal, stop
                break;
            }

            // delimiter — stop scanning
            if is_path_delimiter(b as char) {
                break;
            }

            // skip multi-byte UTF-8: advance by char length
            i += (b as char).len_utf8();
        }

        // need at least 1 byte of content beyond the prefix
        if alone_ok || i > prefix_len {
            Ok(input.split_at(i))
        } else {
            Err(NOT_FOUND)
        }
    }
}

fn last_path_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if !input.starts_with(punct) {
            return Err(NOT_FOUND);
        }
        let bytes = input.as_ref().as_bytes();
        let prefix_len = punct.len();
        let mut i = prefix_len;

        while i < bytes.len() {
            let b = bytes[i];
            if is_path_delimiter(b as char) {
                return Ok(input.split_at(i));
            }
            i += (b as char).len_utf8();
        }
        return Err(NOT_FOUND);
    }
}

/// Returns true if byte `b` is a path-scanning delimiter.
#[inline]
fn is_path_delimiter(c: char) -> bool {
    matches!(c, ';' | '`' | ')' | ']' | '}' | '|' | '>') || c.is_ascii_whitespace()
}

#[cfg(windows)]
fn win_abpath_tag(_: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        let mut it = input.chars();
        if input.len() > 1
            && it.next().map_or(false, |c| c.is_ascii_uppercase())
            && it.next().map_or(false, |c| c == ':')
        {
            let byte_len = input
                .chars()
                .take_while(|&c| !is_path_delimiter(c))
                .map(char::len_utf8)
                .sum::<usize>();
            Ok(input.split_at(byte_len))
        } else {
            Err(NOT_FOUND)
        }
    }
}

// parse argument such as ipconfig /all; C:\
// #[cfg(windows)]
// fn argument_symbol(input: Input<'_>) -> TokenizationResult<'_> {
//     alt((
//         // unix-style paths (also valid on Windows)
//         path_tag("../"),
//         path_tag("./"),
//         path_tag("*/"),
//         path_tag("**/"),
//         // windows drive paths
//         win_abpath_tag(":"),
//         // windows paths
//         path_tag("..\\"),
//         path_tag(".\\"),
//         path_tag("*\\"),
//         path_tag("**\\"),
//         // flags and special tokens
//         path_tag("--"),
//         path_tag("-"),
//         path_tag("~"),
//         path_tag("*."),
//         // url schemes
//         path_tag("http:"),
//         path_tag("https:"),
//         path_tag("ftp:"),
//         path_tag("ftps:"),
//         path_tag("file:"),
//         keyword_alone_or_end("."),
//         keyword_alone_or_end(".."),
//         keyword_alone_or_end("&-"),
//         keyword_alone_or_end("&?"),
//         keyword_alone_or_end("&+"),
//         keyword_alone_or_end("&."),
//     ))(input)
// }
// parse argument such as ls -l --color=auto ./
// #[cfg(unix)]
// fn argument_symbol(input: Input<'_>) -> TokenizationResult<'_> {
//     alt((
//         // flags
//         path_tag("--"),
//         path_tag("-"),
//         // paths
//         path_tag("/"),
//         path_tag("../"),
//         path_tag("./"),
//         path_tag("*/"),
//         path_tag("**/"),
//         path_tag("*."),
//         path_tag("~"),
//         // url schemes
//         path_tag("http:"),
//         path_tag("https:"),
//         path_tag("ftp:"),
//         path_tag("ftps:"),
//         path_tag("file:"),
//         keyword_alone_or_end("."),
//         keyword_alone_or_end(".."),
//         keyword_alone_or_end("&-"),
//         keyword_alone_or_end("&?"),
//         keyword_alone_or_end("&+"),
//         keyword_alone_or_end("&."),
//     ))(input)
// }

fn protocols(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        whole_word("https://"),
        whole_word("http://"),
        whole_word("ftps://"),
        whole_word("ftp://"),
        whole_word("file://"),
    ))(input)
}

fn string_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match input.chars().next() {
        Some('"') => parse_string(input, '"', TokenKind::StringLiteral),
        Some('\'') => parse_string(input, '\'', TokenKind::StringRaw),
        Some('`') => parse_string(input, '`', TokenKind::StringTemplate),
        _ => Err(NOT_FOUND),
    }
}

fn regex_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if input.as_ref().starts_with("r'") {
        parse_prefixed_string(input, "r'", TokenKind::Regex)
    } else {
        Err(NOT_FOUND)
    }
}

fn time_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if input.as_ref().starts_with("t'") {
        parse_prefixed_string(input, "t'", TokenKind::Time)
    } else {
        Err(NOT_FOUND)
    }
}

/// Core string parser: scan for matching close quote, return (rest, (token, diagnostic)).
fn parse_string(
    input: Input<'_>,
    quote: char,
    kind: TokenKind,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    let quote_str = quote.to_string();
    let (inner, _) = input.strip_prefix(&quote_str).ok_or(NOT_FOUND)?;

    let (rest_after_content, diagnostic) = parse_string_inner(inner, quote)?;

    let (rest, content) = finish_string(input, rest_after_content, quote);

    let token = Token::new(kind, content);
    Ok((rest, (token, diagnostic)))
}

fn parse_prefixed_string<'a>(
    input: Input<'a>,
    prefix: &str,
    kind: TokenKind,
) -> TokenizationResult<'a, (Token, Diagnostic)> {
    let (inner, _prefix) = input.strip_prefix(prefix).ok_or(NOT_FOUND)?;
    let quote = prefix.as_bytes().last().copied().unwrap_or(b'\'') as char;
    let (rest_after_content, diagnostic) = parse_string_inner(inner, quote)?;

    let (rest, content) = finish_string(input, rest_after_content, quote);

    let token = Token::new(kind, content);
    Ok((rest, (token, diagnostic)))
}

/// Given the full input and the position after the string content,
/// consume the closing quote (if present) and return (rest, content_range).
fn finish_string<'a>(
    input: Input<'a>,
    rest_after_content: Input<'a>,
    quote: char,
) -> (Input<'a>, StrSlice) {
    let quote_str = quote.to_string();
    match rest_after_content.strip_prefix(&quote_str) {
        Some((after_close, _)) => {
            let (_, content) = input.split_until(after_close);
            (after_close, content)
        }
        None => (rest_after_content, input.split_until(rest_after_content).1),
    }
}

fn number_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    let bytes = input.as_ref().as_bytes();
    let mut i = 0;
    // skip leading dot
    if bytes.get(0) == Some(&b'.') {
        i += 1
    }
    // skip leading digits
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return Err(NOT_FOUND);
    }

    let has_tailing_dot = bytes.get(i) == Some(&b'.');
    // `N..` → integer literal, leave `..` for range operator
    if has_tailing_dot && bytes.get(i + 1) == Some(&b'.') {
        let (rest, range) = input.split_at(i);
        return Ok((
            rest,
            (
                Token::new(TokenKind::IntegerLiteral, range),
                Diagnostic::Valid,
            ),
        ));
    }

    // optional fractional part
    if has_tailing_dot {
        i += 1;
        let frac_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == frac_start {
            // `3.` — invalid float (no fractional digits)
            let (rest, range) = input.split_at(i);
            return Ok((
                rest,
                (
                    Token::new(TokenKind::FloatLiteral, range),
                    Diagnostic::InvalidNumber(range),
                ),
            ));
        }
    }

    let (rest, range) = input.split_at(i);

    let kind = if digit_start > 0 || has_tailing_dot {
        TokenKind::FloatLiteral
    } else {
        TokenKind::IntegerLiteral
    };
    Ok((rest, (Token::new(kind, range), Diagnostic::Valid)))
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
    let len = input.chars().take_while(|&c| is_symbol_char(c)).count();

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

fn linebreak(input: Input<'_>) -> TokenizationResult<'_> {
    let (input, _) = {
        let ws_chars = input.chars().take_while(|c| *c == ' ').count();
        if ws_chars > 0 {
            input.split_at(ws_chars)
        } else {
            (input, input.as_str_slice())
        }
    };

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
    let len = input
        .chars()
        .take_while(|&c| !matches!(c, '\n' | '\r'))
        .map(char::len_utf8)
        .sum();
    Ok(input.split_at(len))
}

fn parse_string_inner(input: Input<'_>, quote_char: char) -> TokenizationResult<'_, Diagnostic> {
    let start_range = input.as_str_slice();
    let quote_byte = quote_char as u8;

    // 使用字节扫描，极速跳过非转义字符
    let bytes = input.as_ref().as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        let b = bytes[pos];
        if b == quote_byte {
            // 匹配到结束引号
            return Ok((input.split_at(pos).0, Diagnostic::Valid));
        } else {
            pos += 1;
            if pos + 1 < bytes.len() {
                // 处理转义：跳过反斜杠及其后一个字节
                pos += 1;
            }
        }
    }

    // 未能找到结束引号
    Ok((
        input.split_at(pos).0,
        Diagnostic::UnterminatedString(start_range),
    ))
}

/// Matches a literal string prefix without any continuation restrictions.
fn punctuation_tag(punct: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| input.strip_prefix(punct).ok_or(NOT_FOUND)
}

/// Matches a keyword/operator that must NOT be followed by symbol characters.
/// Prevents operators from merging into longer symbols (e.g. `&&` vs `&&&`).
fn keyword_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| !rest.starts_with(is_symbol_char))
            .ok_or(NOT_FOUND)
    }
}

/// Matches a keyword that must be followed by whitespace (not end-of-input).
/// Used for standalone keywords like `let`, `set`, `if`, `fn`, `match`.
fn keyword_alone_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}

/// Matches a keyword that must be followed by whitespace OR end-of-input.
/// Used for tokens like `.`, `..`, `&-` that can appear at end of input.
fn keyword_alone_or_end(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.is_empty() || rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}

/// Matches an operator that must NOT be followed by ASCII punctuation.
/// Prevents single-char operators from merging into longer sequences (e.g. `+` vs `+=`).
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

/// Mathes whole word with a prefix
/// similar with path_tag,but don't skip anything. eg `\ `
fn whole_word(prefix: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        if input.starts_with(prefix) {
            let len = input
                .chars()
                .take_while(|c| !matches!(c, ' ' | '\n' | '\t' | '\r' | ')' | ']' | '}'))
                .map(char::len_utf8)
                .sum();
            Ok(input.split_at(len))
        } else {
            Err(NOT_FOUND)
        }
    }
}

/// Matches a token that must be surrounded by whitespace or punctuation (not letters).
/// Used for `_` to distinguish standalone `_` value from `_` within a symbol like `foo_bar`.
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

/// Matches an infix operator (range `..`, `...` etc.) that must sit between operands.
/// Requires previous char to be alphanumeric/`)`/`]`/`_` and next char to be alphanumeric/`(`/`_`/`-`.
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
/// Used for postfix `!` and `^` to prevent merging with following characters.
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
/// Symbol chars: alphanumeric, `_`, `~`, `?`, `&`, `#`, `$`, `-`, `/`, `\`
/// Excluded (cause operator/punctuation parsing instead): `+`, `=`, `<`, `>`, `*`, `%`, `^`, `|`, `:`, `@`, `!`, `.`, `,`, `;`, `(`, `)`, `[`, `]`, `{`, `}`, `'`, `"`, backtick, whitespace
fn is_symbol_char(c: char) -> bool {
    matches!(
        c,
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '~' | '?' | '&' | '#' | '$' | '-' | '/' | '\\'
    )
}

/// Main tokenization entry point.
/// - Single-line input (no `:`) → CFM (Command First Mode) for shell-like parsing
/// - Multi-line or `:`-prefixed → Expression mode with context-aware dispatch
pub(crate) fn parse_tokens(input: Input<'_>) -> (Vec<Token>, Vec<Diagnostic>) {
    if is_cfm_mode(input) {
        return parse_command_tokens(input);
    }

    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ctx = Ctx::Start;
    let mut input = input;

    // skip multiline mode prefix `:`
    if let Ok((new_input, (token, diagnostic))) =
        map_valid_token(punctuation_tag(":"), TokenKind::Comment)(input)
    {
        input = new_input;
        ctx = Ctx::after_token(&token, input.as_original_str());
        tokens.push(token);
        diagnostics.push(diagnostic);
    }

    // tokenize one by one with context tracking
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
    // dbg!(&tokens);
    (tokens, diagnostics)
}

pub fn tokenize(input: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let str = input.into();
    let input = Input::new(&str);
    parse_tokens(input)
}

/// CFM: Command First Mode — shell-style parsing for single-line commands.
/// Active when: input starts with `>`, OR (not multiline, not `:`-prefixed, and CFM enabled).
fn is_cfm_mode(input: Input<'_>) -> bool {
    with_cfm_enabled(|cfm_enabled| {
        input.starts_with(">") || (!input.starts_with(":") && !input.contains('\n') && cfm_enabled)
    })
}

fn parse_command_tokens(input: Input<'_>) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ctx = Ctx::Start;
    let mut input = input;

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
    try_parser!(regex_literal(input));
    try_parser!(time_literal(input));

    if ctx != Ctx::Word {
        // try_parser!(map_valid_token(argument_symbol, TokenKind::StringRaw)(
        //     input
        // ));
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
    let chars = input.chars();
    let mut length = 0;
    // `=` is used for var asign: IFS='';xx
    for c in chars {
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
        keyword_tag("!~:"),
        keyword_tag("~:"),
        keyword_tag("!~="),
        keyword_tag("~="),
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
