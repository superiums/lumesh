use detached_str::StrSlice;
use nom::{
    IResult, Parser,
    branch::alt,
    combinator::{cut, eof, map, opt},
    error::ParseError,
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
};

use crate::{
    Diagnostic, Environment, Expression, Int, Pattern, Token, TokenKind,
    tokens::{Input, Tokens},
};
use std::collections::BTreeMap;

// -- 错误类型定义（与用户提供内容一致）--
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxError {/* 保持原有结构不变 */}

impl SyntaxError {
    /* 保持原有方法不变 */
}

// -- 辅助解析函数 --
#[inline]
fn kind(kind: TokenKind) -> impl Fn(Tokens<'_>) -> IResult<Tokens<'_>, StrSlice, SyntaxError> {
    /* 原有实现不变 */
}

#[inline]
fn text<'a>(text: &'a str) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxError> {
    /* 原有实现不变 */
}

// 表达式 → 赋值
// 赋值 → 条件表达式 ( ( '=' | ':=' ) 赋值 )?
// 条件表达式 → 逻辑或 ( '?' 表达式 ':' 条件表达式 )?
// 逻辑或 → 逻辑与 ( '||' 逻辑或 )?
// 逻辑与 → 比较 ( '&&' 逻辑与 )?
// 比较 → 加减 ( ('==' | '!=' | '>' | '<' | '>=' | '<=' ) 加减 )?
// 加减 → 乘除模 ( ('+' | '-') 乘除模 )*
// 乘除模 → 幂运算 ( ('*' | '/' | '%') 幂运算 )*
// 幂运算 → 单目运算符 ( '**' 幂运算 )?
// 单目运算符 → ('!' | '-') 单目运算符 | 基础表达式
// 基础表达式 → 符号 | 字面量 | 括号 | 列表 | 映射 | 函数调用
// -- 核心解析逻辑增强 --
fn parse_expression(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        parse_assignment,
        parse_conditional,
        parse_logical_or,
        parse_logical_and,
        parse_comparison,
        parse_add_sub,
        parse_mul_div,
        parse_power,
        parse_unary,
        parse_primary,
    ))(input)
}

// 运算符优先级处理
fn parse_primary(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        parse_group,
        parse_list,
        parse_map,
        parse_function_call,
        parse_symbol,
        parse_literal,
    ))(input)
}

// 完整列表解析
fn parse_list(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("[")(input)?;
    let (input, items) = cut(separated_list0(text(","), parse_expression))(input)?;
    let (input, _) = cut(text("]"))(input).map_err(|_| {
        SyntaxError::unrecoverable(input.get_str_slice(), "]", None, Some("列表需要闭合的 ]"))
    })?;

    Ok((input, Expression::List(items)))
}

// 映射解析
fn parse_map(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("{")(input)?;
    let (input, pairs) = cut(separated_list0(
        text(","),
        separated_pair(parse_expression, text(":"), parse_expression),
    ))(input)?;
    let (input, _) = cut(text("}"))(input).map_err(|_| {
        SyntaxError::unrecoverable(input.get_str_slice(), "}", None, Some("映射需要闭合的 }"))
    })?;

    Ok((input, Expression::Map(pairs.into_iter().collect())))
}

// 函数调用解析
fn parse_function_call(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, func) = parse_symbol(input)?;
    let (input, args) = delimited(
        text("("),
        cut(separated_list0(text(","), parse_expression)),
        cut(text(")")),
    )(input)?;

    Ok((
        input,
        Expression::Apply(Box::new(Expression::Symbol(func)), args),
    ))
}

// 增强的条件表达式解析
fn parse_conditional(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, cond) = parse_logical_or(input)?;
    let (input, branch) = opt(pair(
        text("?"),
        cut(pair(
            parse_expression,
            preceded(text(":"), parse_conditional),
        )),
    ))(input)?;

    Ok(match branch {
        Some((_, (then, else_))) => (
            input,
            Expression::Conditional(Box::new(cond), Box::new(then), Box::new(else_)),
        ),
        None => (input, cond),
    })
}

// 增强的赋值解析
fn parse_assignment(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        parse_lazy_assign,
        preceded(
            pair(parse_symbol, text("=")),
            parse_expression
                .map(|expr| Expression::Assign(Box::new(Expression::Symbol), Box::new(expr))),
        ),
    ))(input)
}

// -- 入口函数 --
pub fn parse_script(input: &str) -> Result<Expression, nom::Err<SyntaxError>> {
    // 词法分析阶段
    let str = input.into();
    let tokenization_input = Input::new(&str);
    let (mut token_vec, mut diagnostics) = super::parse_tokens(tokenization_input);

    // 错误处理
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    if !diagnostics.is_empty() {
        return Err(nom::Err::Failure(SyntaxError::TokenizationErrors(
            diagnostics.into_boxed_slice(),
        )));
    }

    // 构建Token流
    let tokens = Tokens {
        str: &str,
        slice: token_vec.as_slice(),
    };

    // 语法分析
    let (_, expr) = parse_script_tokens(tokens, true)?;
    Ok(expr)
}

// 脚本级解析
fn parse_script_tokens(
    input: Tokens<'_>,
    require_eof: bool,
) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, mut statements) = many0(parse_statement)(input)?;
    let (input, last) = opt(terminated(
        parse_expression,
        opt(kind(TokenKind::LineBreak)),
    ))(input)?;

    if let Some(expr) = last {
        statements.push(expr);
    }

    // EOF检查
    if require_eof {
        let _ = eof(input)?;
    }

    Ok((input, Expression::Do(statements)))
}

// -- 辅助类型和常量 --
const PREC_ASSIGNMENT: u8 = 1;
const PREC_CONDITIONAL: u8 = 2;
const PREC_LOGICAL_OR: u8 = 3;
const PREC_LOGICAL_AND: u8 = 4;
const PREC_COMPARISON: u8 = 5;
const PREC_ADD_SUB: u8 = 6;
const PREC_MUL_DIV: u8 = 7;
const PREC_POWER: u8 = 8;
const PREC_UNARY: u8 = 9;
