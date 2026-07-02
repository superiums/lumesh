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
    Number, // Digits sequence, used to detect postfix units like K,M,G,T
    Letter, // Alphabetic sequence
    Start,  // line start / initial state
    Space,  // prev token ended with whitespace (Whitespace/LineBreak/Comment)
    Word,   // prev token ended with  `_`, or closing bracket/quote
    Open,   // other (non-space, non-word symbol)
}

impl Ctx {
    /// Determine the next context based on the current token's ending character.
    /// Whitespace/LineBreak/Comment → Space
    /// Alphanumeric/`_`/closing bracket/quote → Word
    /// Other symbols → Open
    fn after_token(token: &Token, original: &str) -> Self {
        let last_char = token.range.to_str(original).chars().next_back();
        match token.kind {
            TokenKind::Whitespace | TokenKind::Comment => Ctx::Space,
            TokenKind::LineBreak => Ctx::Start,
            TokenKind::IntegerLiteral | TokenKind::FloatLiteral => Ctx::Number,
            _ => match last_char {
                // Some(c) if c.is_ascii_whitespace() => Ctx::Space,
                // Some(c) if c.is_ascii_digit() => Ctx::Number,
                Some(c) if c.is_ascii_alphabetic() => Ctx::Letter,
                Some(')' | ']' | '}' | '\'' | '"' | '`' | '_') => Ctx::Word,
                Some('(' | '[' | '{' | '|') => Ctx::Start,
                _ => Ctx::Open,
            },
        }
    }
}

/// Dispatch tokenization based on the first character and context.
/// Each character triggers a specific parsing path.
fn parse_token_dispatch(
    input: Input<'_>,
    ctx: Ctx,
    is_cfm: bool,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
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
        ' ' | '\t' | '\0' => m!(whitespace, TokenKind::Whitespace),

        ';' => m!(punctuation_tag(";"), TokenKind::LineBreak),
        // #[cfg(windows)]
        '\r' => alt((
            map_valid_token(punctuation_tag("\r\n"), TokenKind::LineBreak), // $var
            map_valid_token(punctuation_tag("\r"), TokenKind::Whitespace),  // $ as symbol
        ))(input),
        '\n' => m!(punctuation_tag("\n"), TokenKind::LineBreak),

        '\\' => {
            if let Ok(r) = m!(line_continuation, TokenKind::Whitespace) {
                return Ok(r);
            }
            m!(punctuation_tag("\\"), TokenKind::Symbol)
        }

        '#' => m!(comment, TokenKind::Comment),

        '"' | '\'' | '`' => string_literal(input),

        '.' => dot_dispatch(input, ctx), // context-aware: method call/range/path/customop

        '-' => minus_dispatch(input, ctx), // context-aware: negative vs flag vs operator

        '(' | ')' | '[' | ']' | '{' | '}' | ',' => paren_dispatch(input, ctx, first),

        '%' => percent_dispatch(input, ctx),

        '!' => bang_dispatch(input, ctx), // context-aware: prefix negate vs postfix call

        '?' => question_dispatch(input, ctx),

        '$' => alt((
            map_valid_token(prefix_tag("$"), TokenKind::OperatorPrefix), // $var
            map_valid_token(punctuation_tag("$"), TokenKind::Symbol),    // $ as symbol
        ))(input),

        '^' => circum_dispatch(input, ctx),
        '&' => and_dispatch(input, ctx),
        '|' => m!(pipe_parser, TokenKind::Operator),
        '=' => m!(equal_parser, TokenKind::Operator),
        '<' => m!(less_parser, TokenKind::Operator),
        '>' => m!(greater_parser, TokenKind::Operator),

        '+' => plus_dispatch(input, ctx, is_cfm),

        '*' => star_dispatch(input, ctx),

        '/' => slash_dispatch(input, ctx),

        '~' => tiled_dispatch(input, ctx),

        ':' => colon_dispatch(input, ctx),

        '@' => at_dispatch(input, ctx),

        '_' => underscore_dispatch(input, ctx), // standalone _ vs _ in symbol

        '0'..='9' => number_literal(input),

        'a'..='z' | 'A'..='Z' => alpha_dispatch(input, ctx, first, is_cfm), // keyword / value_symbol / string / symbol

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
fn paren_dispatch(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    if matches!(ctx, Ctx::Letter | Ctx::Word) && matches!(first, '(' | '[') {
        map_valid_token(
            punctuation_tag(&first.to_string()),
            TokenKind::OperatorPostfix,
        )(input)
    } else {
        map_valid_token(punctuation_tag(&first.to_string()), TokenKind::Punctuation)(input)
    }
}

fn equal_parser(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        space_brace_followed_tag("=>"),
        // punctuation_tag("!=="),
        punctuation_tag("==="),
        // punctuation_tag("!="),
        punctuation_tag("=="),
        punctuation_tag("="),
        // punctuation_tag(">="),
        // punctuation_tag("<="),\
    ))(input)
}
fn less_parser(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        punctuation_tag("<="),
        punctuation_tag("<<"),
        punctuation_tag("<"),
    ))(input)
}
fn greater_parser(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        punctuation_tag(">="),
        punctuation_tag(">>"),
        punctuation_tag(">!"),
        punctuation_tag(">"),
    ))(input)
}
fn plus_dispatch(
    input: Input<'_>,
    ctx: Ctx,
    is_cfm: bool,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // if ctx == Ctx::Space {
    //     map_valid_token(
    //         |input: Input<'_>| {
    //             input
    //                 .strip_prefix("+")
    //                 .filter(|(rest, _)| !rest.starts_with(is_symbol_char))
    //                 .ok_or(NOT_FOUND)
    //         },
    //         TokenKind::Operator,
    //     )(input)
    // } else {
    match ctx {
        Ctx::Space if is_cfm => alt((
            map_valid_token(punctuation_tag("+="), TokenKind::Operator),
            map_valid_token(whole_word("+"), TokenKind::Symbol), //important for `chmod +x`
        ))(input),
        _ => alt((
            map_valid_token(punctuation_tag("+="), TokenKind::Operator),
            map_valid_token(punctuation_tag("+"), TokenKind::Operator),
        ))(input),
    }
}
fn pipe_parser(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        operator_tag("||"),
        punctuation_tag("|>"),
        punctuation_tag("|^"),
        punctuation_tag("|"),
    ))(input)
}

fn and_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word | Ctx::Number | Ctx::Letter => {
            alt((map_valid_token(punctuation_tag("&"), TokenKind::Symbol),))(input)
        } //NEVER USE
        Ctx::Space | Ctx::Open => alt((
            map_valid_token(operator_tag("&&"), TokenKind::Operator),
            map_valid_token(postfix_break_tag("&+"), TokenKind::StringRaw),
            map_valid_token(postfix_break_tag("&-"), TokenKind::StringRaw),
            map_valid_token(postfix_break_tag("&?"), TokenKind::StringRaw),
            map_valid_token(postfix_break_tag("&."), TokenKind::StringRaw),
            map_valid_token(postfix_break_tag("&"), TokenKind::StringRaw),
        ))(input),
        Ctx::Start => alt((map_valid_token(punctuation_tag("&"), TokenKind::Symbol),))(input),
    }
}

fn star_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => {
            map_valid_token(operator_tag("*"), TokenKind::Operator)(input)
        }
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(operator_tag("*="), TokenKind::Operator),
            map_valid_token(path_tag("**/", true), TokenKind::StringRaw), // **/ path
            map_valid_token(path_tag("*/", true), TokenKind::StringRaw),  // */ path
            map_valid_token(path_tag("*.", true), TokenKind::StringRaw),  // *. path
            map_valid_token(operator_tag("*"), TokenKind::Operator),
            map_valid_token(last_path_tag("*"), TokenKind::StringRaw), // `ls *` path at end
        ))(input),
    }
}

fn tiled_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => {
            map_valid_token(punctuation_tag("~"), TokenKind::Symbol)(input) //NEVER
        } //NEVER USE
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(operator_tag("~:"), TokenKind::Operator),
            // map_valid_token(operator_tag("~="), TokenKind::Operator),
            map_valid_token(path_tag("~/", true), TokenKind::StringRaw), // ~/ path
            map_valid_token(last_path_tag("~"), TokenKind::StringRaw),   // `ls ~` path at end
                                                                         // map_valid_token(postfix_break_tag("~"), TokenKind::Operator), // reverse?
        ))(input),
    }
}

fn slash_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => {
            alt((map_valid_token(punctuation_tag("/"), TokenKind::Operator),))(input)
        } //divide
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punctuation_tag("/="), TokenKind::Operator),
            map_valid_token(path_tag("/", false), TokenKind::StringRaw), // /x path
            map_valid_token(last_path_tag("/"), TokenKind::StringRaw),   // `ls /` path at end
            map_valid_token(postfix_break_tag("/"), TokenKind::Operator), // divide
        ))(input),
    }
}

/// `Word` context → `...=`/`...`/`..=`/`..` infix range, or `.` postfix (method call)
/// `Start/Space/Open` context → `..` operator, `.` prefix (pipemethod), argument path, number literal, or standalone `.`/`..`
fn dot_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Number | Ctx::Word => alt((
            map_valid_token(prefix_range_tag("...="), TokenKind::OperatorInfix),
            map_valid_token(prefix_range_tag("..."), TokenKind::OperatorInfix),
            map_valid_token(prefix_range_tag("..="), TokenKind::OperatorInfix),
            map_valid_token(prefix_range_tag(".."), TokenKind::OperatorInfix), //a..b range
            map_valid_token(postfix_break_tag(".."), TokenKind::OperatorPostfix), //a.. range
            map_valid_token(punctuation_tag("."), TokenKind::OperatorPostfix), //call
        ))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(prefix_range_tag("..="), TokenKind::OperatorPrefix), // ..=b range
            map_valid_token(prefix_range_tag(".."), TokenKind::OperatorPrefix),  // ..b range
            number_literal,                                                      //.5
            map_valid_token(prefix_tag("."), TokenKind::OperatorPrefix),         //.pipemethod
            map_valid_token(path_tag("../", true), TokenKind::StringRaw),
            map_valid_token(path_tag("./", true), TokenKind::StringRaw),
            // eager eat, must bellow others
            map_valid_token(punct_seq_tag(".."), TokenKind::Operator), // ..+ customOp
            map_valid_token(postfix_break_tag(".."), TokenKind::StringRaw), // parent path
            map_valid_token(postfix_break_tag("."), TokenKind::StringRaw), // current path                                                                     // TODO ..3
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
        Ctx::Letter | Ctx::Word | Ctx::Number => alt((
            map_valid_token(punctuation_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            map_valid_token(punctuation_tag("-"), TokenKind::Operator), //a-b as operator
                                                                        // map_valid_token(symbol, TokenKind::Symbol),
        ))(input),
        Ctx::Start => alt((map_valid_token(prefix_minus_tag, TokenKind::OperatorPrefix),))(input),
        Ctx::Space => alt((
            map_valid_token(punctuation_tag("-="), TokenKind::Operator),
            map_valid_token(punctuation_tag("->"), TokenKind::Operator),
            // `--flag` style: two dashes → argument symbol
            map_valid_token(whole_word("--"), TokenKind::StringRaw),
            // single `-` followed by literal/number/paren/letter → OperatorPrefix
            map_valid_token(prefix_minus_tag, TokenKind::OperatorPrefix),
            // bare `-` as operator (e.g. `- ` followed by space)
            map_valid_token(space_followed_tag("-"), TokenKind::Operator),
            // must after '- ' to exclude it
            map_valid_token(postfix_break_tag("-"), TokenKind::StringRaw), //ls | cat -
        ))(input),
        Ctx::Open => alt((
            map_valid_token(prefix_minus_tag, TokenKind::OperatorPrefix),
            map_valid_token(punctuation_tag("-"), TokenKind::Symbol),
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

/// Maches range prefix, allow followed by literal/number/(_:]
/// a..-2  a.._  a..(a+b)  a..:2  [a..]
fn prefix_range_tag(prefix: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(prefix)
            .filter(|(rest, _)| {
                rest.starts_with(|c: char| {
                    c.is_ascii_alphanumeric() || matches!(c, '(' | '-' | '_' | ':' | ']')
                })
            })
            .ok_or(NOT_FOUND)
    }
}

fn circum_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => alt((map_valid_token(
            punctuation_tag("^"),
            TokenKind::OperatorPostfix,
        ),))(input), //5%
        Ctx::Start | Ctx::Space | Ctx::Open => {
            map_valid_token(punctuation_tag("^"), TokenKind::Operator)(input)
        }
    }
}

fn percent_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Number => alt((map_valid_token(
            punctuation_tag("%"),
            TokenKind::OperatorPostfix,
        ),))(input), //5%
        Ctx::Letter | Ctx::Word => {
            map_valid_token(punctuation_tag("%"), TokenKind::Operator)(input)
        } //a%b

        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punctuation_tag("%{"), TokenKind::Operator),
            map_valid_token(punctuation_tag("%"), TokenKind::Operator),
        ))(input),
    }
}

/// Context-aware `!` dispatch:
/// - Word context: `!=`/`!==` comparison, `!~:` pattern match, or `!` postfix (flat call)
/// - Start/Space/Open: same operators but `!` as prefix (negation) instead of postfix
fn bang_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => alt((
            map_valid_token(punctuation_tag("!=="), TokenKind::Operator),
            map_valid_token(punctuation_tag("!="), TokenKind::Operator),
            map_valid_token(prefix_tag("!~:"), TokenKind::Operator),
            map_valid_token(postfix_break_tag("!"), TokenKind::OperatorPostfix),
            map_valid_token(punctuation_tag("!"), TokenKind::Punctuation),
        ))(input),
        Ctx::Start | Ctx::Space | Ctx::Open => alt((
            map_valid_token(punctuation_tag("!=="), TokenKind::Operator),
            map_valid_token(punctuation_tag("!="), TokenKind::Operator),
            map_valid_token(prefix_tag("!~:"), TokenKind::Operator),
            map_valid_token(prefix_tag("!"), TokenKind::OperatorPrefix),
            map_valid_token(punctuation_tag("!"), TokenKind::Punctuation),
        ))(input),
    }
}

fn question_dispatch(input: Input<'_>, _ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    alt((
        map_valid_token(question_operator, TokenKind::Operator),
        map_valid_token(operator_tag("?"), TokenKind::Operator),
        map_valid_token(punctuation_tag("?"), TokenKind::Symbol),
    ))(input)
}

fn underscore_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Letter | Ctx::Word | Ctx::Number => alt((
            map_valid_token(punct_seq_tag("__"), TokenKind::OperatorPostfix),
            map_valid_token(punctuation_tag("_"), TokenKind::Symbol),
        ))(input),
        _ => map_valid_token(
            |input| {
                input
                    .strip_prefix("_")
                    .filter(|(rest, _)| {
                        rest.is_empty()
                            || rest.starts_with(&[' ', '\n', ')', ']', '}', ';'])
                            || rest.starts_with("..")
                    })
                    .ok_or(NOT_FOUND)
            },
            TokenKind::ValueSymbol,
        )(input), //`ls _` `[0.._]` `[_..9]`
    }
}

fn alpha_dispatch(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
    is_cfm: bool,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // filesize
    if ctx == Ctx::Number && matches!(&first, 'B' | 'K' | 'M' | 'G' | 'T' | 'P') {
        return map_valid_token(
            postfix_break_tag(&first.to_string()),
            TokenKind::OperatorPostfix,
        )(input);
    }

    // H{ M{ S{ map/set literals
    if matches!(ctx, Ctx::Space | Ctx::Start) && matches!(&first, 'H' | 'M' | 'S') {
        return try_map_or_symbol(input, ctx, first, is_cfm);
    }

    #[cfg(windows)]
    if let Ok(r) = map_valid_token(win_abpath_tag, TokenKind::StringRaw)(input) {
        return Ok(r);
    }

    // keyword should only in ctx::Start/Space, not in ctx::OPen, like `regex.match`
    if ctx == Ctx::Start || ctx == Ctx::Space {
        if let Ok(r) = map_valid_token(any_keyword, TokenKind::Keyword)(input) {
            return Ok(r);
        }
    }
    alt((
        map_valid_token(value_symbol, TokenKind::ValueSymbol),
        regex_literal,
        time_literal,
        map_valid_token(protocols, TokenKind::StringRaw),
        map_valid_token(
            |input| symbol(input, is_cfm, ctx == Ctx::Space),
            TokenKind::Symbol,
        ),
    ))(input)
}

fn try_map_or_symbol(
    input: Input<'_>,
    ctx: Ctx,
    first: char,
    is_cfm: bool,
) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // H{, M{, S{ — check if followed by {
    let bytes = input.as_ref().as_bytes();
    if bytes.len() > 1 && bytes[1] == b'{' {
        map_valid_token(
            punctuation_tag(&format!("{first}{{")),
            TokenKind::Punctuation,
        )(input)
    } else {
        map_valid_token(
            |input| symbol(input, is_cfm, ctx == Ctx::Space),
            TokenKind::Symbol,
        )(input)
    }
}

/// `::` module call infix operator (e.g. `mod::func`).
/// Requires Word context (preceded by identifier) and followed by identifier.
fn colon_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Word => alt((
            map_valid_token(prefix_tag("::"), TokenKind::OperatorInfix),
            map_valid_token(punctuation_tag(":="), TokenKind::Operator),
            map_valid_token(operator_tag(":"), TokenKind::Operator), //{k:v} a?b:c
        ))(input),
        _ => alt((
            map_valid_token(punctuation_tag(":="), TokenKind::Operator),
            map_valid_token(operator_tag(":"), TokenKind::Operator),
        ))(input),
    }
}

/// `@` as prefix operator for decorators (e.g. `@deco`).
/// Requires non-Word context (standalone `@` before identifier).
fn at_dispatch(input: Input<'_>, ctx: Ctx) -> TokenizationResult<'_, (Token, Diagnostic)> {
    match ctx {
        Ctx::Start => alt((
            map_valid_token(prefix_tag("@"), TokenKind::OperatorPrefix), // @deco, @(expr)
        ))(input),
        _ => {
            // Inside an identifier, `@` is not valid — fall through to symbol
            map_valid_token(punctuation_tag("@"), TokenKind::Symbol)(input)
        }
    }
}

/// Matches `?`-prefixed multi-char operators (`?+`, `?.`, `??`, `?>`, `?!`, `?:`, `?~`).
fn question_operator(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        postfix_break_tag("?+"),
        postfix_break_tag("?."),
        postfix_break_tag("??"),
        postfix_break_tag("?>"),
        postfix_break_tag("?!"),
        space_brace_followed_tag("?:"),
        postfix_break_tag("?~"),
    ))(input)
}

fn any_keyword(input: Input<'_>) -> TokenizationResult<'_> {
    alt((
        space_followed_tag("let"),
        space_followed_tag("set"),
        space_followed_tag("alias"),
        space_followed_tag("export"),
        space_brace_followed_tag("if"),
        space_brace_followed_tag("else"),
        space_followed_tag("fn"),
        space_brace_followed_tag("match"),
        space_followed_tag("for"),
        space_followed_tag("in"),
        space_brace_followed_tag("while"),
        space_brace_followed_tag("loop"),
        postfix_break_tag("break"),
        postfix_break_tag("return"),
        space_followed_tag("del"),
        space_followed_tag("use"),
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
            if !matches!(b, b' ' | b'\t') {
                if is_path_delimiter(b as char) {
                    return Ok(input.split_at(i));
                }
                return Err(NOT_FOUND);
            }
            i += (b as char).len_utf8();
        }
        return Ok(input.split_at(i));
    }
}

/// Returns true if byte `b` is a path-scanning delimiter.
#[inline]
fn is_path_delimiter(c: char) -> bool {
    c.is_ascii_whitespace() || matches!(c, ';' | '`' | ')' | ']' | '}' | '|' | '>')
}

#[cfg(windows)]
fn win_abpath_tag(input: Input<'_>) -> TokenizationResult<'_> {
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
//         postfix_break_tag("."),
//         postfix_break_tag(".."),
//         postfix_break_tag("&-"),
//         postfix_break_tag("&?"),
//         postfix_break_tag("&+"),
//         postfix_break_tag("&."),
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
//         postfix_break_tag("."),
//         postfix_break_tag(".."),
//         postfix_break_tag("&-"),
//         postfix_break_tag("&?"),
//         postfix_break_tag("&+"),
//         postfix_break_tag("&."),
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

fn ip_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // Simple IP detection: four groups of digits separated by dots
    let mut parts = 0;
    let mut i = 0;
    let bytes = input.as_ref().as_bytes();
    while i < bytes.len() {
        // consume digits
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            break;
        }
        parts += 1;
        if parts == 4 {
            break;
        }
        // expect a dot
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
        } else {
            break;
        }
    }
    if parts == 4 {
        let (rest, range) = input.split_at(i);
        Ok((
            rest,
            (Token::new(TokenKind::StringRaw, range), Diagnostic::Valid),
        ))
    } else {
        Err(NOT_FOUND)
    }
}

fn number_literal(input: Input<'_>) -> TokenizationResult<'_, (Token, Diagnostic)> {
    // First, try to parse as an IP address (e.g., 192.168.0.1)
    if let Ok(res) = ip_literal(input) {
        return Ok(res);
    }
    // Original numeric literal parsing
    let bytes = input.as_ref().as_bytes();
    let mut i = 0;
    if bytes.get(0) == Some(&b'.') {
        i += 1;
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
        postfix_break_tag("true"),
        postfix_break_tag("false"),
        postfix_break_tag("none"),
        space_punc_followed_tag("_"),
    ))(input)
}

fn symbol(input: Input<'_>, is_cfm: bool, is_space_ctx: bool) -> TokenizationResult<'_> {
    let len = input
        .chars()
        .take_while(|&c| is_symbol_char(c, is_cfm, is_space_ctx))
        .count();

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

// fn linebreak(input: Input<'_>) -> TokenizationResult<'_> {
//     let (input, _) = {
//         let ws_chars = input.chars().take_while(|c| *c == ' ').count();
//         if ws_chars > 0 {
//             input.split_at(ws_chars)
//         } else {
//             (input, input.as_str_slice())
//         }
//     };

//     #[cfg(windows)]
//     if let Some((rest, nl_slice)) = input.strip_prefix("\r\n") {
//         return Ok((rest, nl_slice));
//     }

//     if let Some((rest, nl_slice)) = input.strip_prefix("\n") {
//         Ok((rest, nl_slice))
//     } else if let Some((rest, matched)) = input.strip_prefix(";") {
//         Ok((rest, matched))
//     } else {
//         Err(NOT_FOUND)
//     }
// }
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
            if b == b'\\' && pos + 1 < bytes.len() {
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
// fn keyword_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
//     move |input: Input<'_>| {
//         input
//             .strip_prefix(keyword)
//             .filter(|(rest, _)| !rest.starts_with(is_symbol_char))
//             .ok_or(NOT_FOUND)
//     }
// }

/// Matches a keyword that must be followed by whitespace (not end-of-input).
/// Used for standalone keywords like `let`, `set`, `if`, `fn`, `match`.
fn space_followed_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.starts_with(char::is_whitespace))
            .ok_or(NOT_FOUND)
    }
}
fn space_brace_followed_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| {
                rest.starts_with(char::is_whitespace)
                    || rest.starts_with('{')
                    || rest.starts_with('(')
            })
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

/// Matches a token that must be followed by whitespace or punctuation (not letters).
/// Used for `_` to distinguish standalone `_` value from `_` within a symbol like `foo_bar`.
fn space_punc_followed_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
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

/// Matches a postfix operator that must be followed by whitespace or dilimeter or end-of-input.
/// Used for postfix `!` and `^` to prevent merging with following characters.
fn postfix_break_tag(keyword: &str) -> impl '_ + Fn(Input<'_>) -> TokenizationResult<'_> {
    move |input: Input<'_>| {
        input
            .strip_prefix(keyword)
            .filter(|(rest, _)| rest.is_empty() || rest.starts_with(|c: char| is_path_delimiter(c)))
            .ok_or(NOT_FOUND)
    }
}
/// Checks whether the character is allowed in a symbol.
/// Symbol chars: alphanumeric, `_`, `~`, `?`, `&`, `#`, `$`, `-`, `/`, `\`
/// Excluded (cause operator/punctuation parsing instead): `+`, `=`, `<`, `>`, `*`, `%`, `^`, `|`, `:`, `@`, `!`, `.`, `,`, `;`, `(`, `)`, `[`, `]`, `{`, `}`, `'`, `"`, backtick, whitespace
fn is_symbol_char(c: char, is_cfm: bool, is_space_ctx: bool) -> bool {
    if c.is_ascii_whitespace() {
        return false;
    }
    if is_cfm {
        // eat `.` only on space_ctx, for 'git tag v0.0.1'
        // eat `:` only on space_ctx, for 'cut -d:'
        if is_space_ctx {
            return matches!(
                c,
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '~' | '?' | '&' | '#' | '$' | '-' | '/' | '\\' | '=' | '+' | '.' | ':'
            );
        }
        // eat `=` for `dd if=/dev`
        // eat `+` for `cmd arg+`
        return matches!(
            c,
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '~' | '?' | '&' | '#' | '$' | '-' | '/' | '\\' | '=' | '+'
        );
    }
    // allow `a-b` as symbo
    matches!(
        c,
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '~' | '?' | '&' | '#' | '$' | '-' | '/' | '\\'
    )
}

/// Main tokenization entry point.
/// - Single-line input (no `:`) → CFM (Command First Mode) for shell-like parsing
/// - Multi-line or `:`-prefixed → Expression mode with context-aware dispatch
pub(crate) fn parse_tokens(input: Input<'_>) -> (Vec<Token>, Vec<Diagnostic>) {
    // if is_cfm_mode(input) {
    //     return parse_command_tokens(input);
    // }
    let is_cfm = is_cfm_mode(input);
    let leading_char = if is_cfm { ">" } else { ":" };

    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ctx = Ctx::Start;
    let mut input = input;

    // skip multiline mode prefix `:`
    if let Ok((new_input, (token, diagnostic))) =
        map_valid_token(punctuation_tag(leading_char), TokenKind::ModeTip)(input)
    {
        input = new_input;
        tokens.push(token);
        diagnostics.push(diagnostic);
    }

    // tokenize one by one with context tracking
    loop {
        match parse_token_dispatch(input, ctx, is_cfm) {
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
        input.starts_with(">") || (cfm_enabled && !input.starts_with(":") && !input.contains('\n'))
    })
}
