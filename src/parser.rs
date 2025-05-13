use core::option::Option::None;
use std::{collections::HashMap, rc::Rc};

use crate::{
    Diagnostic, Expression, Int, Pattern, SliceParams, SyntaxErrorKind, Token, TokenKind,
    expression::CatchType,
    tokens::{Input, Tokens},
};
use detached_str::StrSlice;
use nom::{IResult, branch::alt, combinator::*, multi::*, sequence::*};

// -- 辅助类型和常量 --

// 优先级常量
// 双目操作符
// const PREC_CONTROL: u8 = 0; // 控制结构（语句级）
const PREC_ASSIGN: u8 = 1; // 赋值 =
const PREC_REDIRECT: u8 = 2; // 重定向
const PREC_PIPE: u8 = 2; // 管道
const PREC_CATCH: u8 = 3;

const PREC_LAMBDA: u8 = 4; // lambda -> ~>
const PREC_CONDITIONAL: u8 = 5; // 条件运算符 ?:
const PREC_LOGICAL_OR: u8 = 6; // 逻辑或 ||
const PREC_LOGICAL_AND: u8 = 7; // 逻辑与 &&
const PREC_COMPARISON: u8 = 8; // 比较运算

const PREC_CMD_ARG: u8 = 9;
const PREC_FUNC_ARG: u8 = 10;

const PREC_ADD_SUB: u8 = 11; // 加减
const PREC_MUL_DIV: u8 = 12; // 乘除模 custom_op _*
const PREC_POWER: u8 = 13; // 幂运算 ^
const PREC_CUSTOM: u8 = 14; // 自定义
// 其他
// prefix
const PREC_UNARY: u8 = 20; // 单目运算符     ! -
const PREC_PRIFIX: u8 = 21; // 单目运算符     ++ --
// postfix
// const PREC_POSTFIX: u8 = 22; //             ++ --
// const PREC_CALL: u8 = 24; //                func()
// arry list
// const PREC_RANGE: u8 = 25; // range         ..
// const PREC_LIST: u8 = 25; // 数组         [1,2]
// const PREC_SLICE: u8 = 25; //               arry[]
const PREC_INDEX: u8 = 25; // 索引运算符      @ .
// group
const PREC_GROUP: u8 = 28; // 分组括号      ()

// Literal
const PREC_LITERAL: u8 = 29; //原始字面量     "x"
// cmd
// const PREC_CMD_NAME: u8 = 30;
// const PREC_FUNC_NAME: u8 = 31;

// var
// const PREC_SYMBOL: u8 = 32; //变量名         x

// -- 辅助结构 --
#[derive(Debug)]
struct OperatorInfo<'a> {
    symbol: &'a str,
    precedence: u8,
    right_associative: bool,
    // kind: OperatorKind,
}
// #[derive(Debug)]
// enum OperatorKind {
//     Prefix,
//     // Postfix,
//     Infix,
// }
impl<'a> OperatorInfo<'a> {
    fn new(symbol: &'a str, precedence: u8, right_associative: bool) -> Self {
        Self {
            symbol,
            precedence,
            right_associative,
        }
    }
}
// -- Pratt 解析器核心结构 --
const MAX_DEPTH: u8 = 100;
/// 基于优先级0
fn parse_expr(input: Tokens) -> IResult<Tokens, Expression, SyntaxErrorKind> {
    // dbg!("--parse--");
    let (input, got) = PrattParser::parse_expr_with_precedence(input, 0, 0)?;
    // dbg!(&input.slice, &got);
    Ok((input, got))
}

struct PrattParser;
/// Pratt解析器增强实现, 基于优先级的表达式解析
impl PrattParser {
    // 核心表达式解析
    fn parse_expr_with_precedence(
        mut input: Tokens<'_>,
        min_prec: u8,
        mut depth: u8,
    ) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
        // 1. 解析前缀表达式
        dbg!("===----prepare to prefix---===>", input, min_prec);
        let (new_input, mut lhs) = Self::parse_prefix(input, min_prec)?;
        input = new_input;
        dbg!("=======prefix=======>", input, &lhs, min_prec);
        // opt(alt((kind(TokenKind::LineBreak), eof_slice)));
        // 2. 循环处理中缀和后缀
        loop {
            depth += 1;
            if depth > MAX_DEPTH {
                // break;
                return Err(nom::Err::Error(SyntaxErrorKind::RecursionDepth {
                    input: input.get_str_slice(),
                    depth,
                }));
            }
            // 检查终止条件
            if input.is_empty()
            // || input
            //     .first()
            //     .map(|t| t.kind == TokenKind::LineBreak)
            //     .unwrap_or(false)
            {
                dbg!("---break1---");
                break;
            }

            // 获取运算符信息
            let operator_token = input.first().unwrap();
            let operator = operator_token.text(input);

            // let tk_kind = operator_token.kind.clone();
            // dbg!(&operator, operator_token.kind);

            // 处理不同类型的运算符
            match operator_token.kind {
                TokenKind::LineBreak | TokenKind::Punctuation | TokenKind::OperatorPrefix => {
                    dbg!("---break1.1---");
                    break;
                }
                TokenKind::OperatorInfix => {
                    // 中缀运算符 (. .. @)
                    input = input.skip_n(1);
                    let (new_input, rhs) = Self::parse_prefix(input, PREC_INDEX)?;
                    input = new_input;
                    match operator {
                        "." | "@" => lhs = Expression::Index(Rc::new(lhs), Rc::new(rhs)),
                        ".." => lhs = Expression::BinaryOp("..".into(), Rc::new(lhs), Rc::new(rhs)),
                        _ => unreachable!(),
                    }
                }
                TokenKind::Operator => {
                    // 双目运算符 (+ - * / 等)
                    // 获取当前运算符信息
                    let op_info = match Self::get_operator_info(operator) {
                        Some(opi) => opi,
                        None => break,
                    };
                    dbg!(&op_info);
                    if op_info.precedence < min_prec {
                        //dbg!("低于当前优先级则退出", op_info.precedence, min_prec);
                        break; // 低于当前优先级则退出
                    }

                    let next_min_prec = if op_info.right_associative {
                        op_info.precedence
                    } else {
                        op_info.precedence + 1
                    };

                    input = input.skip_n(1);

                    // dbg!("--> binOp: trying next loop", input, next_min_prec);
                    // dbg!("--> binOp: trying next loop", &lhs, &operator);
                    match operator {
                        "?." | "?+" | "??" | "?!" => {
                            // dbg!("--->try catch ast:");
                            lhs = Self::build_catch_ast(op_info, lhs)?
                        }
                        _ => {
                            if input.is_empty() {
                                //dbg!("---break2---");
                                break;
                            }
                            // dbg!("--->try bin:", &operator, operator == "?.");
                            // inclue | "?:"
                            let (new_input, rhs) =
                                Self::parse_expr_with_precedence(input, next_min_prec, depth)?;
                            // dbg!("--> binOp: after next loop", &rhs);
                            input = new_input;
                            lhs = Self::build_bin_ast(input, op_info, lhs, rhs)?;
                        }
                    }
                }
                TokenKind::OperatorPostfix => {
                    // dbg!("--->post fix");
                    // dbg!(&lhs, operator, input);
                    // 后缀运算符 (函数调用、数组索引等)
                    (input, lhs) = Self::build_postfix_ast(lhs, operator.to_string(), input)?;
                    // dbg!(&input, &lhs);
                }
                TokenKind::Symbol
                | TokenKind::StringLiteral
                | TokenKind::StringRaw
                | TokenKind::IntegerLiteral
                | TokenKind::FloatLiteral
                | TokenKind::ValueSymbol
                    if min_prec < PREC_CMD_ARG =>
                {
                    // 当operator不是符号时，表示这不是双目运算，而是类似cmd a 3 c+d e.f 之类的函数调用
                    //
                    // dbg!("--> Args: trying next loop", input.len(), PREC_CMD_ARG);
                    if input.len() == 1 {
                        // CMD arg1, 只有第一个参数
                        let (new_input, rhs) = alt((parse_symbol, parse_literal))(input)?;
                        // dbg!(&rhs);
                        input = new_input;
                        lhs = Expression::Command(Rc::new(lhs), Rc::new(vec![rhs]));
                    } else {
                        // CMD ... 所有参数
                        let (new_input, rhs) = many0(|input| {
                            Self::parse_expr_with_precedence(input, PREC_CMD_ARG, depth)
                        })(input)?;
                        //dbg!("--> Args: after next loop", &rhs);

                        // dbg!(&rhs);
                        input = new_input;
                        lhs = Expression::Command(Rc::new(lhs), Rc::new(rhs));
                    }
                    // dbg!("---break3---", input.len());

                    // break;
                }
                // TokenKind::Symbol
                // | TokenKind::StringLiteral
                // | TokenKind::StringRaw
                // | TokenKind::IntegerLiteral
                // | TokenKind::FloatLiteral
                // | TokenKind::ValueSymbol => {
                //     // CMD ... arg_last 多参数的最后一个
                //     dbg!("--> Args: last", input.len());
                //     if input.len() == 1 {
                //         return alt((parse_symbol, parse_literal))(input);
                //     }
                // }
                _ => {
                    dbg!("---break4---", input.len());
                    break;
                }
            }
            // if input.is_empty() {
            //     dbg!("---break5---");
            //     break;
            // }
        }

        // dbg!("---returning---", input);

        Ok((input, lhs))
    }

    // 前缀表达式解析
    fn parse_prefix(
        input: Tokens<'_>,
        min_prec: u8,
    ) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
        SyntaxErrorKind::empty_back(input)?;

        let first = input.first().unwrap();
        // dbg!(&first);
        match first.kind {
            TokenKind::OperatorPrefix => {
                let op = first.text(input);
                // vars
                if op == "$" {
                    return preceded(text("$"), map(parse_symbol_string, Expression::Variable))(
                        input,
                    );
                }
                // unary op
                let prec = match op {
                    "!" | "-" => PREC_UNARY,
                    "++" | "--" => PREC_PRIFIX,
                    _ => {
                        return Err(nom::Err::Error(SyntaxErrorKind::UnknownOperator(
                            op.to_string(),
                        )));
                    }
                };

                if prec < min_prec {
                    return Err(nom::Err::Error(SyntaxErrorKind::PrecedenceTooLow));
                }

                let input = input.skip_n(1);
                let (input, expr) = Self::parse_prefix(input, prec)?;
                Ok((input, Expression::UnaryOp(op.into(), Rc::new(expr), true)))
            }
            TokenKind::Symbol => parse_symbol(input),
            TokenKind::StringLiteral if PREC_LITERAL >= min_prec => parse_string(input),
            TokenKind::StringRaw if PREC_LITERAL >= min_prec => parse_string_raw(input),
            TokenKind::IntegerLiteral if PREC_LITERAL >= min_prec => parse_integer(input),
            TokenKind::FloatLiteral if PREC_LITERAL >= min_prec => parse_float(input),
            TokenKind::ValueSymbol if PREC_LITERAL >= min_prec => parse_value_symbol(input),
            TokenKind::Punctuation if PREC_GROUP >= min_prec => {
                let op = first.text(input);
                match op {
                    "(" => {
                        // 分组{表达式 (expr)
                        alt((parse_group, parse_lambda_param))(input)
                    }
                    "`" => {
                        // 数组字面量 [expr, ...]
                        parse_subcommand(input)
                    }
                    "[" => {
                        // 数组字面量 [expr, ...]
                        parse_list(input)
                    }
                    "{" => alt((parse_map, parse_block))(input),
                    // opx if opx.starts_with("__") => map(parse_operator(input),TokenKind::OperatorPrefix),
                    _ => Err(nom::Err::Error(SyntaxErrorKind::UnknownOperator(
                        op.to_string(),
                    ))), //其余的操作符，不在前缀中处理
                }
            }
            TokenKind::Keyword => parse_control_flow(input),
            _ => Err(nom::Err::Error(SyntaxErrorKind::UnknownOperator(
                first.text(input).to_string(),
            ))),
        }
    }
    // 后缀表达式构建
    fn build_postfix_ast(
        lhs: Expression,
        op: String,
        input: Tokens<'_>,
    ) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
        match op.as_str() {
            "(" => {
                // 函数调用
                // let (input, args) =
                //     terminated(separated_list0(text(","), parse_expr), text(")"))(input)?;
                let (input, args) = delimited(
                    text("("),
                    separated_list0(text(","), |inp| {
                        PrattParser::parse_expr_with_precedence(inp, PREC_FUNC_ARG, 0)
                        // PrattParser::parse_expr_with_precedence(inp, PREC_CMD_ARG, 0)
                    }),
                    text_close(")"),
                )(input)?;
                // dbg!(&lhs, &args);
                Ok((input, Expression::Apply(Rc::new(lhs), Rc::new(args))))
            }
            "!" => {
                // 函数调用
                // let (input, args) =
                //     terminated(separated_list0(text(","), parse_expr), text(")"))(input)?;

                let (input, args) =
                    many0(|inp| PrattParser::parse_expr_with_precedence(inp, PREC_FUNC_ARG, 0))(
                        input.skip_n(1),
                    )?;
                // dbg!(&lhs, &args);
                Ok((input, Expression::Apply(Rc::new(lhs), Rc::new(args))))
            }
            "[" => {
                // 数组索引或切片
                parse_index_or_slice(lhs, input)
            }
            "++" | "--" => {
                // 后置自增/自减
                Ok((
                    input.skip_n(1),
                    Expression::UnaryOp(op, Rc::new(lhs), false),
                ))
            }
            opx if opx.starts_with("__") => {
                // 后置自定义
                // dbg!(&opx, &lhs);
                Ok((
                    input.skip_n(1),
                    Expression::UnaryOp(opx.into(), Rc::new(lhs), false),
                ))
            }
            _ => Err(nom::Err::Error(SyntaxErrorKind::UnknownOperator(
                op.to_string(),
            ))),
        }
    }

    // 运算符元数据
    fn get_operator_info(op: &str) -> Option<OperatorInfo<'_>> {
        match op {
            // 赋值运算符（右结合）
            "=" | ":=" | "+=" | "-=" | "*=" | "/=" => {
                Some(OperatorInfo::new(op, PREC_ASSIGN, true))
            }
            // lambda
            "->" => Some(OperatorInfo::new(op, PREC_LAMBDA, true)),
            // 索引符
            // "@" | "." => Some(OperatorInfo::new(op, PREC_INDEX, false)),
            // range
            // ".." => Some(OperatorInfo::new("..", PREC_RANGE, false)),

            // 加减运算符
            "+" | "-" => Some(OperatorInfo::new(op, PREC_ADD_SUB, false)),
            // 乘除模运算符
            "*" | "/" | "%" => Some(OperatorInfo::new(op, PREC_MUL_DIV, false)),
            // 幂运算符
            "^" => Some(OperatorInfo::new("^", PREC_POWER, true)),
            // 单目前缀运算符
            // "!" | "++" | "--" => Some(OperatorInfo::new(
            //     op,
            //     PREC_UNARY,
            //     false,
            //     OperatorKind::Prefix,
            // )),
            // 逻辑运算符
            "&&" => Some(OperatorInfo::new("&&", PREC_LOGICAL_AND, false)),
            "||" => Some(OperatorInfo::new("||", PREC_LOGICAL_OR, false)),
            // 比较运算符
            "==" | "!=" | ">" | "<" | ">=" | "<=" => {
                Some(OperatorInfo::new(op, PREC_COMPARISON, false))
            }
            // 匹配
            "~~" | "~=" => Some(OperatorInfo::new(op, PREC_COMPARISON, false)),
            // 三目
            "?" => Some(OperatorInfo::new(
                "?",
                PREC_CONDITIONAL, // 优先级设为2
                true,             // 右结合
            )),
            ":" => Some(OperatorInfo::new(
                ":",
                PREC_CONDITIONAL,
                true, //important
            )),
            // ... 管道操作符 ...
            "|" | "|>" => Some(OperatorInfo::new(
                op, PREC_PIPE, // 例如设为 4（低于逻辑运算符）
                false,
            )),
            // ... 重定向操作符 ...
            "<<" | ">>" | ">>>" => Some(OperatorInfo::new(op, PREC_REDIRECT, false)),
            // opa if opa.starts_with("__") => Some(OperatorInfo::new(
            //     opa,
            //     PREC_UNARY,
            //     false,
            //     OperatorKind::Prefix,
            // )),
            "?." | "?+" | "??" | "?!" | "?:" => Some(OperatorInfo::new(op, PREC_CATCH, false)),

            opa if opa.starts_with("_+") => Some(OperatorInfo::new(opa, PREC_ADD_SUB, false)),
            ops if ops.starts_with("_*") => Some(OperatorInfo::new(ops, PREC_MUL_DIV, false)),
            opo if opo.starts_with("_") => Some(OperatorInfo::new(opo, PREC_CUSTOM, false)),
            _ => None,
        }
    }

    fn build_bin_ast(
        input: Tokens,
        op: OperatorInfo,
        lhs: Expression,
        rhs: Expression,
    ) -> Result<Expression, nom::Err<SyntaxErrorKind>> {
        match op.symbol {
            "+" | "-" | "*" | "/" | "%" | "^" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),
            // "." | "@" => Ok(Expression::Index(
            //     // op.symbol.into(),
            //     Box::new(lhs),
            //     Box::new(rhs),
            // )),
            "&&" | "||" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),
            "=" => {
                // 确保左侧是符号
                match lhs.to_symbol() {
                    Ok(name) => {
                        // 如果是命令, 则包装为字符串，命令应当明确用()包裹
                        let last = match rhs {
                            Expression::Command(s, v) => Expression::Symbol(
                                s.to_string()
                                    + " "
                                    + v.iter()
                                        .map(|e| e.to_string())
                                        .collect::<Vec<String>>()
                                        .join(" ")
                                        .as_str(),
                            ),
                            other => other,
                        };
                        Ok(Expression::Assign(name.to_string(), Rc::new(last)))
                    }
                    _ => {
                        // eprintln!("invalid left-hand-side: {:?}", lhs);
                        Err(SyntaxErrorKind::failure(
                            input.get_str_slice(),
                            "symbol",
                            Some(format!("{:?}", lhs)),
                            Some("only assign to symbol allowed"),
                        ))
                    }
                }
            }
            "==" | "!=" | ">" | "<" | ">=" | "<=" | "~~" | "~=" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),

            // "->" => {
            //     // 参数处理
            //     match lhs.to_symbol() {
            //         // .expect("Lambda参数必须是符号");
            //         Ok(name) => {
            //             // 解析体部分（强制解析为代码块）
            //             let body = match rhs {
            //                 Expression::Group(boxed_expr) => {
            //                     // 如果体是分组表达式，尝试解析为代码块
            //                     if let Expression::Do(statements) = *boxed_expr {
            //                         statements
            //                     } else {
            //                         vec![*boxed_expr]
            //                     }
            //                 }
            //                 _ => vec![rhs],
            //             };
            //             Ok(Expression::Lambda(
            //                 name.to_string(),
            //                 Box::new(Expression::Do(body)),
            //                 Environment::new(),
            //             ))
            //         }
            //         _ => {
            //             eprintln!("invalid lambda-param {:?}", lhs);

            //             Err(SyntaxErrorKind::expected(
            //                 input.get_str_slice(),
            //                 "symbol",
            //                 Some(lhs.to_string()),
            //                 "lambda params must be symbol".into(),
            //             ))
            //         }
            //     }

            //     // 解析体部分
            //     // Expression::Lambda(name.to_string(), Box::new(rhs), Environment::new())
            // }
            "->" => {
                // 解析参数列表
                let params = match lhs {
                    // 处理括号包裹的参数列表 (x,y,z)
                    Expression::Group(boxed_expr) => match boxed_expr.as_ref() {
                        Expression::List(elements) => elements
                            .as_ref()
                            .iter()
                            .map(|e| e.to_symbol().map(|s| s.to_string()))
                            .collect::<Result<Vec<_>, _>>(),
                        // 处理单个参数 (x)
                        // expr => match expr {
                        // Expression::List(s) => return Ok(Expression::List(s)),
                        Expression::Symbol(s) => Ok(vec![s.to_owned()]),
                        _ => {
                            return Err(SyntaxErrorKind::failure(
                                input.get_str_slice(),
                                "symbol in parameter list",
                                Some(boxed_expr.type_name()),
                                "put only valid symbols in lambda/macro param list".into(),
                            ));
                        } // },
                    },
                    // 处理无括号单参数
                    Expression::Symbol(name) => Ok(vec![name]),
                    _ => {
                        // eprintln!("invalid lambda/macro param {:?}", lhs);
                        return Err(SyntaxErrorKind::failure(
                            input.get_str_slice(),
                            "symbol or parameter list",
                            Some(lhs.to_string()),
                            "Lambda/Macro requires valid parameter list".into(),
                        ));
                    }
                };

                // 自动包装body为代码块
                let body = match rhs {
                    // 已有代码块保持原样
                    Expression::Do(_) => rhs,
                    // 分组表达式展开
                    Expression::Group(boxed_expr) => boxed_expr.as_ref().clone(),
                    // 其他表达式自动包装
                    _ => Expression::Do(Rc::new(vec![rhs])),
                };

                // 构建Lambda表达式
                // match op.symbol {
                // "->" =>
                Ok(Expression::Lambda(params.unwrap(), Rc::new(body)))
                //     "~>" => Ok(Expression::Macro(params.unwrap(), Box::new(body))),
                //     _ => unreachable!(),
                // }
            }
            // "~>" => {
            //     // 参数处理
            //     match lhs.to_symbol() {
            //         // 解析体部分
            //         Ok(name) => Ok(Expression::Macro(name.to_string(), Box::new(rhs))),
            //         _ => {
            //             eprintln!("invalid macro-param {:?}", lhs);

            //             Err(SyntaxErrorKind::expected(
            //                 input.get_str_slice(),
            //                 "symbol",
            //                 Some(lhs.to_string()),
            //                 "macro params must be symbol".into(),
            //             ))
            //         }
            //     }
            // }
            "?" => {
                // dbg!("?+--->", &lhs, &rhs);
                let (true_expr, false_expr) = match rhs {
                    Expression::BinaryOp(op, t, f) if op == ":" => (t, f),
                    _ => {
                        // eprintln!("invalid conditional ?: {:?}", rhs);
                        return Err(SyntaxErrorKind::failure(
                            input.get_str_slice(),
                            "a:b",
                            Some(rhs.to_string()),
                            "add true_value:false_value after '?'".into(),
                        ));
                    }
                };
                Ok(Expression::If(Rc::new(lhs), true_expr, false_expr))
            }
            ":" => {
                // dbg!(":---->", &lhs, &rhs);
                Ok(Expression::BinaryOp(":".into(), Rc::new(lhs), Rc::new(rhs)))
            }

            ":=" => {
                // 确保左侧是符号
                match lhs.to_symbol() {
                    Ok(name) => {
                        // 如果是单独的symbol，则包装为命令
                        let last = match rhs {
                            Expression::Symbol(s) => {
                                Expression::Command(Rc::new(Expression::Symbol(s)), Rc::new(vec![]))
                            }
                            other => other,
                        };
                        Ok(Expression::Assign(
                            name.to_string(),
                            Rc::new(Expression::Quote(Rc::new(last))),
                        ))
                    }
                    _ => {
                        // eprintln!("invalid left-hide-side {:?}", lhs);
                        Err(SyntaxErrorKind::failure(
                            input.get_str_slice(),
                            "symbol",
                            Some(lhs.to_string()),
                            "only assign to symbol allowed".into(),
                        ))
                    }
                }
            }
            "+=" | "-=" | "*=" | "/=" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),
            // {

            //         let base_op = op.symbol.trim_end_matches('=');
            //         let new_rhs =
            //             Expression::BinaryOp(base_op.into(), Box::new(lhs.clone()), Box::new(rhs));
            //         Ok(Expression::Assign(lhs.to_string(), Box::new(new_rhs)))
            //     }
            "|" | "|>" => Ok(Expression::Pipe(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),

            "<<" | ">>" | ">>>" => Ok(Expression::Pipe(
                op.symbol.into(),
                Rc::new(lhs),
                Rc::new(rhs),
            )),
            opx if opx.starts_with("_") => {
                Ok(Expression::BinaryOp(opx.into(), Rc::new(lhs), Rc::new(rhs)))
            }
            "?:" => Ok(Expression::Catch(
                Rc::new(lhs),
                CatchType::Deel,
                Some(Rc::new(rhs)),
            )),

            _ => {
                unreachable!()
            }
        }
    }

    fn build_catch_ast(
        op: OperatorInfo,
        lhs: Expression,
    ) -> Result<Expression, nom::Err<SyntaxErrorKind>> {
        // dbg!("--->catch ast:", &op);
        Ok(match op.symbol {
            "?." => Expression::Catch(Rc::new(lhs), CatchType::Ignore, None),
            "?+" => Expression::Catch(Rc::new(lhs), CatchType::PrintStd, None),
            "??" => Expression::Catch(Rc::new(lhs), CatchType::PrintErr, None),
            "?!" => Expression::Catch(Rc::new(lhs), CatchType::PrintOver, None),
            _ => unreachable!(),
        })
    }
}
// 统一控制流解析（适用于语句和表达式）
fn parse_control_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    alt((
        parse_if_flow,    // 同时处理if语句和if表达式
        parse_match_flow, // 同时处理match语句和match表达式
        parse_while_flow, // 同时处理while/for循环
        parse_loop_flow,
        parse_for_flow,
        parse_break,
        parse_return,
    ))(input)
}
/// -- expr with cmd
/// a b c as expr
/// a as cmd
/// 如果是单独的symbol，则包装为命令
fn parse_expr_with_single_cmd(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    // dbg!("---parse_single_expr");
    let (input, expr) = parse_expr(input)?;
    if let Expression::Symbol(s) = expr {
        Ok((
            input,
            Expression::Command(Rc::new(Expression::Symbol(s)), Rc::new(vec![])),
        ))
    } else {
        Ok((input, expr))
    }
}
// -- 子命令 --
fn parse_subcommand(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    // dbg!(input);
    // let (input, sub) = delimited(text("`"), parse_command_call, text_close("`"))(input)?;
    // 需要允许 && | 等操作，不能只是单独的命令。
    let (input, sub) = delimited(text("`"), parse_expr_with_single_cmd, text_close("`"))(input)?;
    // dbg!(input, &sub);
    Ok((input, sub))
}
// -- 分组 --
fn parse_group(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    delimited(
        text("("),
        map(parse_expr_with_single_cmd, |e| {
            Expression::Group(Rc::new(e))
        }),
        // map(alt((parse_math, parse_command_call)), |e| {
        //     Expression::Group(Box::new(e))
        // }),
        text_close(")"),
    )(input)
}

fn parse_list(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    delimited(
        text("["),
        map(separated_list0(text(","), parse_expr), |s| {
            Expression::from(s)
        }),
        // opt(text(",")), //TODO 允许末尾，
        text_close("]"),
    )(input)
}

// 参数解析函数
fn parse_param(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, (String, Option<Expression>), SyntaxErrorKind> {
    alt((
        // 带默认值的参数解析分支
        map(
            separated_pair(
                parse_symbol_string,
                text("="),
                // 限制只能解析基本类型表达式
                parse_literal,
            ),
            |(name, expr)| (name, Some(expr)), // 将结果包装为Some
        ),
        // map(preceded(text("*"), parse_symbol_string), |s| {
        //     (s, None, true)
        // }),
        // 普通参数解析分支
        map(parse_symbol_string, |s| (s, None)), // , 1+2 also match first symbol, so failed in ) parser.
    ))(input)
}

// 函数参数列表解析
fn parse_param_list(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, (Vec<(String, Option<Expression>)>, Option<String>), SyntaxErrorKind> {
    let (input, _) = cut(text("("))(input).map_err(|_| {
        SyntaxErrorKind::failure(
            input.get_str_slice(),
            "function params declare",
            None,
            Some("add something like (x,y)"),
        )
    })?;
    let (input, params) = separated_list0(text(","), parse_param)(input)?;
    // let mut params = vec![];
    // let mut param_collector: Option<String> = None;
    // for (p, dvalue, is_colllector) in x {
    //     if is_colllector {
    //         param_collector = Some(p);
    //     } else {
    //         params.push((p, dvalue));
    //     }
    // }
    let (input, param_collector) = opt(preceded(
        opt(text(",")),
        preceded(text("*"), parse_symbol_string),
    ))(input)?;
    // let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; //允许可选回车
    // 如果还有其他字符，应报错
    // dbg!(&input, &params);
    if !input.is_empty() {
        match input.first() {
            Some(&token) if token.text(input) != ")" => {
                // dbg!(token.text(input));
                return Err(SyntaxErrorKind::failure(
                    input.get_str_slice(),
                    "valid function params declare",
                    None,
                    Some("params should like (x,y=0)"),
                ));
            }
            _ => {}
        }
    }
    let (input, _) = cut(text_close(")"))(input)?;
    Ok((input, (params, param_collector)))
}
// lambda参数
fn parse_lambda_param(input: Tokens) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, expr) = delimited(
        text("("),
        // alt((
        // 参数列表特殊处理
        map(separated_list0(text(","), parse_symbol), |symbols| {
            Expression::from(symbols)
        }),
        //     parse_expr, // 常规表达式
        // )),
        text_close(")"),
    )(input)?;
    Ok((input, Expression::Group(Rc::new(expr))))
}
// 函数定义解析
fn parse_fn_declare(mut input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    (input, _) = opt(many0(kind(TokenKind::LineBreak)))(input)?;

    SyntaxErrorKind::empty_back(input)?;

    let (input, _) = text("fn")(input)?;
    // dbg!("---parse_fn_declare");

    let (input, name) = parse_symbol_string(input).map_err(|_| {
        // eprintln!("mising fn name?");
        // dbg!(input, input.get_str_slice());
        SyntaxErrorKind::failure(
            input.get_str_slice(),
            "function name",
            None,
            Some("add a name for your function"),
        )
    })?;
    let (input, (params, param_collector)) = parse_param_list(input)?; // 使用新参数列表
    let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; //允许可选回车

    // 无函数体应报错
    // dbg!(&input, &params);
    if match input.first() {
        Some(&token) if token.text(input).ne("{") => true,
        None => true,
        _ => false,
    } {
        return Err(SyntaxErrorKind::failure(
            input.get_str_slice(),
            "function body",
            None,
            Some("add a function body like {...}"),
        ));
    }
    let (input, body) = parse_block(input)?;
    // catch
    let (input, handler_options) = opt(alt((
        map(text("?."), |_| (CatchType::Ignore, None)),
        map(text("?+"), |_| (CatchType::PrintStd, None)),
        map(text("??"), |_| (CatchType::PrintErr, None)),
        map(text("?!"), |_| (CatchType::PrintOver, None)),
        map(preceded(text("?:"), parse_expr), |e| {
            (CatchType::Deel, Some(Rc::new(e)))
        }),
    )))(input)?;

    let last_body = match handler_options {
        Some((ctyp, handler)) => Rc::new(Expression::Catch(Rc::new(body), ctyp, handler)),
        _ => Rc::new(body),
    };
    Ok((
        input,
        Expression::Function(name, params, param_collector, last_body),
    ))
}
// return statement
fn parse_return(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("return")(input)?;
    let (input, expr) = opt(parse_expr)(input)?;
    Ok((
        input,
        Expression::Return(Rc::new(expr.unwrap_or(Expression::None))),
    ))
}
fn parse_break(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("break")(input)?;
    let (input, expr) = opt(parse_expr)(input)?;
    Ok((
        input,
        Expression::Break(Rc::new(expr.unwrap_or(Expression::None))),
    ))
}

// fn parse_command_call(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
//     // 如果第二个token为operator,则不是命令。如 a = 3,
//     // dbg!(input.len());
//     // if input.len() > 1
//     //     && input
//     //         .skip_n(1)
//     //         .first()
//     //         .is_some_and(|x| x.kind == TokenKind::Operator)
//     // // && !["`", "("].contains(&x.text(input)))
//     // {
//     //     // dbg!("--->cmd escape---");

//     //     return Err(nom::Err::Error(SyntaxErrorKind::NoExpression));
//     // }
//     // dbg!("--->cmd call---");
//     let (input, ident) = PrattParser::parse_expr_with_precedence(input, PREC_CMD_NAME, 0)?;
//     let (input, args) =
//         many0(|inp| PrattParser::parse_expr_with_precedence(inp, PREC_CMD_ARG, 0))(input)?;
//     Ok((input, Expression::Command(Box::new(ident), args)))
// }

// -- 其他辅助函数保持与用户提供代码一致 --
#[inline]
fn kind(kind: TokenKind) -> impl Fn(Tokens<'_>) -> IResult<Tokens<'_>, StrSlice, SyntaxErrorKind> {
    move |input: Tokens<'_>| match input.first() {
        Some(&token) if token.kind == kind => Ok((input.skip_n(1), token.range)),
        _ => Err(nom::Err::Error(SyntaxErrorKind::InternalError)),
    }
}

#[inline]
fn text<'a>(text: &'a str) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxErrorKind> {
    move |input: Tokens<'a>| match input.first() {
        Some(&token) if token.text(input) == text => Ok((input.skip_n(1), token)),
        _ => Err(nom::Err::Error(SyntaxErrorKind::InternalError)),
    }
}
// fn text_starts_with<'a>(
//     text: &'a str,
// ) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxErrorKind> {
//     move |input: Tokens<'a>| match input.first() {
//         Some(&token) if token.text(input).starts_with(text) => Ok((input.skip_n(1), token)),
//         _ => Err(nom::Err::Error(SyntaxErrorKind::InternalError)),
//     }
// }
#[inline]
fn text_close<'a>(
    text: &'static str,
) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxErrorKind> {
    move |input: Tokens<'a>| match input.first() {
        Some(&token) if token.text(input) == text => Ok((input.skip_n(1), token)),
        _ => Err(SyntaxErrorKind::unclosed_delimiter(
            input.get_str_slice(),
            text,
        )),
    }
}

#[inline]
fn parse_symbol(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    map(kind(TokenKind::Symbol), |t| {
        Expression::Symbol(t.to_str(input.str).to_string())
    })(input)
}
fn parse_symbol_string(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxErrorKind> {
    map(kind(TokenKind::Symbol), |t| t.to_str(input.str).to_string())(input)
}

#[inline]
fn parse_string(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, string) = kind(TokenKind::StringLiteral)(input)?;
    Ok((
        input,
        Expression::String(snailquote::unescape(string.to_str(input.str)).unwrap()),
    ))
}
#[inline]
fn parse_string_raw(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, expr) = kind(TokenKind::StringRaw)(input)?;
    let raw_str = expr.to_str(input.str);

    // 检查首尾单引号
    if raw_str.len() >= 2 {
        // 通过StrSlice直接计算子范围
        let start = expr.start() + 1;

        // 如果有右侧引号，则调整结束位置
        let end = match raw_str.chars().last().unwrap() {
            '\'' => expr.end() - 1,
            _ => expr.end(),
        };

        let content = input.str.get(start..end); // 截取中间部分
        Ok((
            input,
            Expression::String(content.to_str(input.str).to_string()),
        ))
    } else {
        Err(SyntaxErrorKind::failure(
            expr,
            "raw string enclosed in single quotes",
            Some(raw_str.to_string()),
            Some("raw strings must surround with '"),
        ))
    }
}

// -- 字面量解析 --
fn parse_literal(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    alt((
        parse_integer,
        parse_float,
        parse_string,
        parse_string_raw,
        parse_value_symbol,
    ))(input)
}

// 映射解析
fn parse_map(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    // 不能用cut，防止map识别失败时，影响后面的block解析。
    let (input, _) = terminated(text("{"), opt(kind(TokenKind::LineBreak)))(input)?;
    let (input, pairs) = separated_list0(
        terminated(text(","), opt(kind(TokenKind::LineBreak))),
        separated_pair(
            parse_symbol_string,
            text(":"),
            alt((parse_literal, parse_symbol)),
        ),
    )(input)?;
    // dbg!(&input, &pairs);
    let (input, _) = opt(terminated(text(","), opt(kind(TokenKind::LineBreak))))(input)?;
    let (input, _) = terminated(text_close("}"), opt(kind(TokenKind::LineBreak)))(input)?;
    // dbg!(&input);
    let hashmap: HashMap<String, Expression> = pairs.into_iter().collect();
    // Ok((input, Expression::Map(pairs)))
    Ok((input, Expression::from(hashmap)))
}

fn parse_integer(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, num) = kind(TokenKind::IntegerLiteral)(input)?;
    let num = num.to_str(input.str).parse::<Int>().map_err(|e| {
        SyntaxErrorKind::failure(num, "integer", Some(format!("error: {}", e)), None)
    })?;
    Ok((input, Expression::Integer(num)))
}

fn parse_float(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, num) = kind(TokenKind::FloatLiteral)(input)?;
    let num = num.to_str(input.str).parse::<f64>().map_err(|e| {
        SyntaxErrorKind::failure(
            num,
            "float",
            Some(format!("error: {}", e)),
            Some("valid floats can be written like 1.0 or 5.23"),
        )
    })?;
    Ok((input, Expression::Float(num)))
}

#[inline]
fn parse_value_symbol(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    map(kind(TokenKind::ValueSymbol), |s| {
        match s.to_str(input.str) {
            "None" => Expression::None,
            "True" => Expression::Boolean(true),
            "False" => Expression::Boolean(false),
            _ => Expression::None,
        }
    })(input)
}

// -- 入口函数与脚本解析 --

// -- 入口函数 --
pub fn parse_script(input: &str) -> Result<Expression, nom::Err<SyntaxErrorKind>> {
    // 词法分析阶段
    let str = input.into();
    let tokenization_input = Input::new(&str);
    let (mut token_vec, mut diagnostics) = super::parse_tokens(tokenization_input);

    // dbg!(&token_vec);
    // 错误处理
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    if !diagnostics.is_empty() {
        return Err(nom::Err::Failure(SyntaxErrorKind::TokenizationErrors(
            diagnostics.into_boxed_slice(),
        )));
    }

    // remove whitespace
    token_vec.retain(|t| !matches!(t.kind, TokenKind::Whitespace | TokenKind::Comment));

    // 语法分析
    match parse_script_tokens(
        Tokens {
            str: &str,
            slice: token_vec.as_slice(),
        },
        // true,
    ) {
        Ok((_, expr)) => Ok(expr),
        Err(e) => {
            // eprintln!("parse error");
            Err(e)
        }
    }
    // dbg!(&expr);

    // Ok(expr)
}
// ================== 控制结构解析 ==================
// 核心解析流程架构
pub fn parse_script_tokens(
    input: Tokens<'_>,
    // require_eof: bool,
) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    if input.is_empty() {
        return Ok((input, Expression::None));
    }
    // dbg!("---------parse_script_tokens");

    // 阶段1：解析语句序列（控制结构在此处理）
    // dbg!("------>1", input);
    // let (input, mut functions) = many0(parse_statement)(input)?;
    let (input, functions) = terminated(
        parse_functions,
        opt(alt((kind(TokenKind::LineBreak), eof_slice))), // 允许换行符作为语句分隔
    )(input)?;

    if !input.is_empty() {
        // dbg!("-----==>Remaining:", &input.slice, &functions);
        eprintln!(
            "unrecognized satement. \nremaining:{:?}\nrecognized:{:?}",
            &input.slice, &functions
        );
        return Err(SyntaxErrorKind::expected(
            input.get_str_slice(),
            "valid Expression",
            Some("unrecognized expression".into()),
            Some("check your syntax"),
        ));
    }

    // dbg!("==========", &functions);
    match functions.len() {
        0 => Err(nom::Err::Error(SyntaxErrorKind::NoExpression)),
        1 => {
            let s = functions.first().unwrap();
            Ok((input, s.clone()))
        }
        _ => Ok((input, Expression::Do(Rc::new(functions)))),
    }
}
/// 函数解析（顶层结构）
fn parse_functions(input: Tokens<'_>) -> IResult<Tokens<'_>, Vec<Expression>, SyntaxErrorKind> {
    // dbg!("---parse_functions");

    let (input, statement) = many0(alt((
        // parse_import,        // 模块导入（仅语句级）
        terminated(
            parse_fn_declare,
            opt(kind(TokenKind::LineBreak)), // 允许换行符作为语句分隔
        ), // 函数声明（顶级）
        terminated(
            parse_statement,
            opt(kind(TokenKind::LineBreak)), // 允许换行符作为语句分隔
        ), // 函数声明（顶级）
    )))(input)?;
    // let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; // 消费换行符

    // dbg!(&input, &statement);
    Ok((input, statement))
}
// 语句块解析器（顶层结构）
fn parse_statement(mut input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    // dbg!("---parse_statement");
    // dbg!(input);
    (input, _) = opt(many0(kind(TokenKind::LineBreak)))(input)?;
    // if input.is_empty() {
    //     // dbg!("---break prefix---");
    //     return Err(nom::Err::Error(SyntaxErrorKind::NoExpression));
    // }
    SyntaxErrorKind::empty_back(input)?;
    let (input, statement) = alt((
        parse_fn_declare, // 函数声明（仅语句级）这里的作用是允许函数嵌套
        // 1.声明语句
        parse_lazy_assign,
        parse_declare,
        parse_alias,
        parse_del,
        // 2.控制流语句
        // parse_control_flow,
        // 3.运算语句: !3, 1+2, must before flat_call,
        // or discard this, only allow `let a=3+2` => parse_declare
        // or discard this, only allow `a=3+2` => parse_expr
        // parse_direct_print,
        // parse_math,
        // 4.执行语句: ls -l, add(x)
        // parse_func_call,
        // parse_direct_fn_call,
        // parse_cmd_or_math,
        // parse_command_call,
        // 便捷控制台打印
        // 5.单语句： 字面量和单独的symbol：[2,3] 4 "5" ls x
        parse_single_expr,
        // 块语句 {}
        // parse_block,
    ))(input)?;
    // let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; // 消费换行符
    // dbg!(&input, &statement, &statement.type_name());
    // dbg!(&statement, &statement.type_name());
    Ok((input, statement))
}
///命令或数学运算。
///语句开始，等号后，括号中：应匹配 cmd call，match compute.

/// 便捷控制台打印
// fn parse_direct_print(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
//     // dbg!("--direct print--");
//     let (input, _) = text(":")(input)?;
//     parse_func_call(input)
// }

/// 运算语句
// fn parse_math(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
//     if input.is_empty() {
//         return Err(nom::Err::Error(SyntaxErrorKind::NoExpression));
//     }
//     // dbg!("--->parse_math---");
//     match input.first().unwrap().kind {
//         TokenKind::IntegerLiteral
//         | TokenKind::FloatLiteral
//         | TokenKind::Operator
//         | TokenKind::StringLiteral
//         | TokenKind::StringRaw
//         | TokenKind::OperatorPrefix => {
//             parse_expr(input) // 完整表达式
//             // terminated(
//             //     opt(alt((
//             //         // 必须包含语句终止符
//             //         kind(TokenKind::LineBreak),
//             //         eof_slice, // 允许文件末尾无终止符
//             //     ))),
//             // )(input)
//         }
//         TokenKind::Symbol
//             if input.len() > 1
//                 && input
//                     .skip_n(1)
//                     .first()
//                     .is_some_and(|x| x.kind == TokenKind::Operator) =>
//         {
//             parse_expr(input)
//         }
//         _ => Err(nom::Err::Error(SyntaxErrorKind::NoExpression)),
//     }
// }
/// 单独语句
fn parse_single_expr(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    // dbg!("---parse_single_expr");
    // let is_single_cmd = input.len() == 1;
    let (input, expr) = terminated(
        parse_expr, // 完整表达式
        opt(alt((
            // 必须包含语句终止符
            kind(TokenKind::LineBreak),
            eof_slice, // 允许文件末尾无终止符
        ))),
    )(input)?;
    // if is_single_cmd && input.is_empty() {
    if let Expression::Symbol(s) = expr {
        Ok((
            input,
            Expression::Command(Rc::new(Expression::Symbol(s)), Rc::new(vec![])),
        ))
    } else {
        Ok((input, expr))
    }
    // }
    // Ok((input, expr))
}

// IF语句解析（支持else if链）
// TOTO 允许无{} ?
fn parse_if_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("if")(input)?;
    let (input, cond) = parse_expr(input)?;
    let (input, then_block) = opt(parse_block)(input)?; //must have block to differ with condition
    // 解析else分支
    let (input, else_branch) = opt(preceded(
        text("else"),
        alt((
            parse_if_flow,       // else if
            parse_block_or_expr, // else
        )),
    ))(input)?;

    let els = else_branch.unwrap_or(Expression::None);
    Ok((
        input,
        Expression::If(
            Rc::new(cond),
            Rc::new(then_block.unwrap_or(Expression::None)),
            Rc::new(els),
        ),
    ))
}

// WHILE循环解析
fn parse_while_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("while")(input)?;
    let (input, cond) = parse_expr(input)?;
    let (input, body) = parse_block(input)?;

    Ok((input, Expression::While(Rc::new(cond), Rc::new(body))))
}
// LOOP循环解析
fn parse_loop_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("loop")(input)?;
    let (input, body) = parse_block(input)?;

    Ok((input, Expression::Loop(Rc::new(body))))
}

// FOR循环解析
fn parse_for_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("for")(input)?;
    let (input, pattern) = parse_symbol_string(input)?; // 或更复杂的模式匹配
    let (input, _) = text("in")(input)?;
    let (input, iterable) = parse_expr(input)?;
    let (input, body) = parse_block(input)?;

    Ok((
        input,
        Expression::For(pattern, Rc::new(iterable), Rc::new(body)),
    ))
}

// MATCH表达式解析
fn parse_match_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("match")(input)?;
    let (input, matched) = parse_expr(input)?;
    let (input, _) = terminated(text("{"), opt(kind(TokenKind::LineBreak)))(input)?;

    // 解析多个匹配分支
    let (input, expr_map) = separated_list1(
        terminated(
            map(opt(text(",")), |_| {}),
            map(kind(TokenKind::LineBreak), |_| {}),
        ),
        separated_pair(parse_pattern, text("=>"), parse_expr),
    )(input)?;
    let (input, _) = opt(text(","))(input)?;
    let (input, _) = opt(kind(TokenKind::LineBreak))(input)?;
    let (input, _) = terminated(text_close("}"), opt(kind(TokenKind::LineBreak)))(input)?;
    let branches = expr_map
        .into_iter()
        .map(|(pattern, expr)| (pattern, Rc::new(expr)))
        .collect::<Vec<_>>();
    Ok((input, Expression::Match(Rc::new(matched), branches)))
}

// ================== 条件运算符?: ==================

// 条件运算符处理

// 一元运算符具体实现

// ================== 辅助函数 ==================
// 动态识别块或表达式
fn parse_block_or_expr(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    alt((
        parse_block, // 优先识别 {...} 块
        parse_expr,  // 单行表达式（如 x > y ? a : b）
    ))(input)
}
// 解析代码块（带花括号）
// TODO with return?
fn parse_block(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, block) = delimited(
        terminated(text("{"), opt(many0(kind(TokenKind::LineBreak)))),
        cut(map(
            many0(terminated(
                parse_statement,
                opt(many0(kind(TokenKind::LineBreak))),
            )),
            |stmts| Expression::Do(Rc::new(stmts)),
        )),
        cut(text_close("}")),
    )(input)?;
    // dbg!(&block);
    Ok((input, block))
}

// 别名解析逻辑
fn parse_alias(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("alias")(input)?;
    let (input, symbol) = parse_symbol_string(input)?;
    let (input, _) = text("=")(input)?;
    let (input, expr) = parse_expr_with_single_cmd(input)?;
    // dbg!(&expr);
    Ok((input, Expression::Alias(symbol, Rc::new(expr))))
}
// 延迟赋值解析逻辑
fn parse_lazy_assign(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("let")(input)?;
    let (input, symbol) = parse_symbol_string(input)?;
    let (input, _) = text(":=")(input)?; // 使用:=作为延迟赋值符号
    let (input, expr) = parse_expr_with_single_cmd(input)?;
    // dbg!(&expr);
    Ok((
        input,
        Expression::Assign(symbol, Rc::new(Expression::Quote(Rc::new(expr)))),
    ))
}

fn parse_declare(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("let")(input)?;
    // dbg!("parse_declare");
    // 解析逗号分隔的多个符号, 允许重载操作符,自定义操作符
    let (input, symbols) = separated_list0(
        text(","),
        alt((
            parse_symbol_string,
            parse_operator,
            parse_custom_postfix_operator,
        )),
    )(input)
    .map_err(|_| {
        SyntaxErrorKind::failure(
            input.get_str_slice(),
            "symbol list",
            None,
            Some("try: `let x, y = 1, 2`"),
        )
    })?;

    // 解析等号和多表达式
    let (input, exprs) = opt(preceded(text("="), separated_list0(text(","), parse_expr)))(input)?;

    // 构建右侧表达式
    let assignments = match exprs {
        Some(e) if e.len() == 1 => {
            if symbols.len() == 1 {
                // 如果是单独的symbol，则包装为命令
                // let last = match &e[0] {
                //     Expression::Symbol(s) => {
                //         Expression::Command(Rc::new(Expression::Symbol(s.to_owned())), vec![])
                //     }
                //     other => other.clone(),
                // };
                // 如果是命令, 则包装为字符串，命令应当明确用()包裹
                let last = match e[0].clone() {
                    Expression::Command(s, v) => Expression::Symbol(
                        s.to_string()
                            + " "
                            + v.iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<String>>()
                                .join(" ")
                                .as_str(),
                    ),
                    other => other,
                };
                return Ok((
                    input,
                    Expression::Declare(symbols[0].clone(), Rc::new(last)),
                ));
            }
            (0..symbols.len())
                .map(|i| Expression::Declare(symbols[i].clone(), Rc::new(e[0].clone())))
                .collect()
        }
        Some(e) if e.len() == symbols.len() => (0..symbols.len())
            .map(|i| Expression::Declare(symbols[i].clone(), Rc::new(e[i].clone())))
            .collect(),
        Some(e) => {
            return Err(SyntaxErrorKind::failure(
                input.get_str_slice(),
                "matching values count",
                Some(format!(
                    "got {} variables but {} values",
                    symbols.len(),
                    e.len()
                )),
                Some("ensure each variable has a corresponding value"),
            ));
        }
        None => vec![],
    };
    Ok((input, Expression::Do(Rc::new(assignments))))
}

fn parse_del(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, _) = text("del")(input)?;
    let (input, symbol) = parse_symbol_string(input).map_err(|_| {
        SyntaxErrorKind::failure(
            input.get_str_slice(),
            "symbol",
            Some("no symbol".into()),
            Some("you can only del symbol"),
        )
    })?;
    Ok((input, Expression::Del(symbol)))
}

fn parse_operator(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxErrorKind> {
    map(kind(TokenKind::Operator), |t| {
        t.to_str(input.str).to_string()
    })(input)
}
fn parse_custom_postfix_operator(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, String, SyntaxErrorKind> {
    map(kind(TokenKind::OperatorPostfix), |t| {
        t.to_str(input.str).to_string()
    })(input)
}
// 模式匹配解析（简化示例）
fn parse_pattern(input: Tokens<'_>) -> IResult<Tokens<'_>, Pattern, SyntaxErrorKind> {
    alt((
        map(text("_"), |_| Pattern::Bind("_".to_string())), // 将_视为特殊绑定
        map(parse_symbol_string, Pattern::Bind),
        map(parse_literal, |lit| Pattern::Literal(Rc::new(lit))),
    ))(input)
}
// 自定义EOF解析器，返回StrSlice类型
fn eof_slice(input: Tokens<'_>) -> IResult<Tokens<'_>, StrSlice, SyntaxErrorKind> {
    if input.is_empty() {
        Ok((input, StrSlice::default()))
    } else {
        Err(nom::Err::Error(SyntaxErrorKind::Expected {
            input: input.get_str_slice(),
            expected: "end of input",
            found: None,
            hint: Some("Check your input"),
        }))
    }
}

// 索引/切片解析
fn parse_index_or_slice(
    target: Expression,
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, Expression, SyntaxErrorKind> {
    let (input, (params, is_slice)) =
        delimited(text("["), parse_slice_params, text_close("]"))(input)?;

    Ok((
        input,
        match is_slice {
            true => Expression::Slice(Rc::new(target), params),
            false => Expression::Index(Rc::new(target), params.start.unwrap()),
        },
    ))
}

fn parse_slice_params(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, (SliceParams, bool), SyntaxErrorKind> {
    // 解析 start 部分
    let (input, start) = opt(alt((parse_integer, parse_symbol)))(input)?;

    // 检查第一个冒号
    let (input, has_first_colon) = opt(text(":"))(input)?;

    // 解析 end 部分
    let (input, end) = if has_first_colon.is_some() {
        opt(alt((parse_integer, parse_symbol)))(input)?
    } else {
        (input, None) // 如果没有第一个冒号，就没有 end
    };

    // 检查第二个冒号
    let (input, has_second_colon) = opt(text(":"))(input)?;

    // 解析 step 部分
    let (input, step) = if has_second_colon.is_some() {
        opt(alt((parse_integer, parse_symbol)))(input)?
    } else {
        (input, None) // 如果没有第二个冒号，就没有 step
    };

    Ok((
        input,
        (
            SliceParams {
                start: start.map(Rc::new),
                end: end.map(Rc::new),
                step: step.map(Rc::new),
            },
            has_first_colon.is_some(),
        ),
    ))
}
