// parser.rs
// 采用Pratt解析器处理运算符优先级，结构更清晰
use crate::{Expression, SyntaxError, Token, TokenKind, Tokens};
use detached_str::StrSlice;
use nom::{branch::alt, combinator::cut, sequence::delimited};

struct PrattParser;

impl PrattParser {
    // 基于优先级的表达式解析
    fn parse_expression(input: Tokens, min_prec: u8) -> IResult<Tokens, Expression, SyntaxError> {
        let (mut input, mut lhs) = self.parse_prefix(input)?;

        loop {
            let op = match input.first() {
                Some(t) => Self::get_operator_info(t),
                None => break,
            };

            if op.precedence < min_prec {
                break;
            }

            // 处理右结合运算符
            let next_min_prec = if op.right_associative {
                op.precedence
            } else {
                op.precedence + 1
            };

            input = input.skip(1);
            let (new_input, rhs) = self.parse_expression(input, next_min_prec)?;
            lhs = self.build_expression(op, lhs, rhs);
            input = new_input;
        }

        Ok((input, lhs))
    }

    // 运算符元数据
    fn get_operator_info(t: &Token) -> OperatorInfo {
        match t.text() {
            "**" => OperatorInfo::new(4, true),
            "*" | "/" | "%" => OperatorInfo::new(5, false),
            "+" | "-" => OperatorInfo::new(6, false),
            "==" | "!=" | ">" | "<" | ">=" | "<=" => OperatorInfo::new(7, false),
            "&&" => OperatorInfo::new(8, false),
            "||" => OperatorInfo::new(9, false),
            "|>" => OperatorInfo::new(10, false),
            "=" | ":=" => OperatorInfo::new(11, true), // 右结合
            _ => OperatorInfo::none(),
        }
    }

    // 前缀表达式（基础元素）
    fn parse_prefix(&self, input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
        alt((
            parse_group,
            parse_list,
            parse_map,
            parse_function_call,
            parse_symbol,
            parse_literal,
            parse_unary_op,
        ))(input)
    }

    // 中缀表达式构建
    fn build_expression(&self, op: OperatorInfo, lhs: Expression, rhs: Expression) -> Expression {
        match op.symbol {
            "&&" | "||" => Expression::LogicalOp(op.symbol, Box::new(lhs), Box::new(rhs)),
            "**" | "+" | "-" | "*" | "/" | "%" => {
                Expression::BinaryOp(op.symbol, Box::new(lhs), Box::new(rhs))
            }
            "|>" => Expression::Pipe(Box::new(lhs), Box::new(rhs)),
            "=" => Expression::Assign(Box::new(lhs), Box::new(rhs)),
            _ => unreachable!(),
        }
    }
}
pub fn parse_script(input: &str) -> Result<Expression, nom::Err<SyntaxError>> {
    // 执行词法分析
    let str = input.into();
    let tokenization_input = Input::new(&str);
    let (mut token_vec, mut diagnostics) = super::parse_tokens(tokenization_input);

    // 处理词法错误
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    if !diagnostics.is_empty() {
        return Err(nom::Err::Failure(SyntaxError::TokenizationErrors(
            diagnostics.into_boxed_slice(),
        )));
    }

    // 解析Token流
    let (_, expr) = parse_script_tokens(
        Tokens {
            str: &str,
            slice: token_vec.as_slice(),
        },
        true,
    )?;
    Ok(expr)
}
// 管道运算符解析（示例）
fn parse_pipe_expression(input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
    let (input, lhs) = parse_primary(input)?;
    let (input, rhs) = delimited(
        parse_operator("|>"),
        PrattParser::parse_expression(input, PREC_PIPE),
    )(input)?;
    Ok((input, Expression::Pipe(Box::new(lhs), Box::new(rhs))))
}

// 错误处理增强
fn parse_group(input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
    let start = input.first().unwrap().range;
    let (input, _) = text("(")(input)
        .map_err(|_| SyntaxError::expected(start, "(", input.first().map(|t| t.text())))?;

    let (input, expr) = cut(parse_expression)(input)?;

    let (input, _) =
        cut(text(")"))(input).map_err(|_| SyntaxError::unclosed_delimiter(start, ")"))?;

    Ok((input, Expression::Group(Box::new(expr))))
}

// ----
fn parse_control_flow(input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
    alt((
        parse_if_chain,   // 处理if-else if-else链
        parse_nested_for, // 支持嵌套循环
        parse_while_with_condition,
    ))(input)
}

// 示例：链式if解析
fn parse_if_chain(input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
    let (input, _) = text("if")(input)?;
    let (input, cond) = parse_expression(input)?;
    let (input, then_block) = parse_block(input)?;

    // 处理else if链
    let (input, else_block) = opt(preceded(
        text("else"),
        alt((
            parse_if_chain.map(|e| Expression::ElseIf(Box::new(e))),
            parse_block.map(|b| Expression::Else(Box::new(b))),
        )),
    ))(input)?;

    Ok((
        input,
        Expression::If(Box::new(cond), Box::new(then_block), else_block),
    ))
}

// ----
// 统一列表/映射终止符检查
macro_rules! delimited_container {
    ($start:literal, $end:literal, $parser:ident) => {{
        move |input| {
            let start = input.first().unwrap().range;
            let (input, _) = text($start)(input)?;
            let (input, items) = cut(separated_list0(text(","), $parser))(input)?;
            let (input, _) =
                cut(text($end))(input).map_err(|_| SyntaxError::unclosed_delimiter(start, $end))?;
            Ok((input, items))
        }
    }};
}

// let parse_list = delimited_container!("[", "]", parse_expression);
// let parse_map = delimited_container!("{", "}", parse_key_value);

// 优先级常量
const PREC_PIPE: u8 = 10;
const PREC_ASSIGNMENT: u8 = 11;

struct OperatorInfo {
    precedence: u8,
    right_associative: bool,
    symbol: &'static str,
}
