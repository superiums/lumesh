use crate::{
    Diagnostic, Environment, Expression, Int, Pattern, SliceParams, SyntaxError, Token, TokenKind,
    tokens::{Input, Tokens},
};
use detached_str::StrSlice;
use nom::{IResult, branch::alt, combinator::*, multi::*, sequence::*};

// 输入：if x > 5 { y = 1 } else { y = 0 }; a + b * c

// 解析流程：
// 1. parse_script_tokens 进入语句解析循环
// 2. parse_statement 识别if关键字，进入parse_if_flow
//    a. 解析条件表达式 x > 5（调用Pratt解析器）
//    b. 解析then块 { y = 1 }
//    c. 解析else块 { y = 0 }
// 3. 消费分号终止符
// 4. 解析表达式语句 a + b * c
//    a. Pratt解析器处理运算符优先级（*优先于+）
// 5. 生成最终的Do表达式包含两个子节点

// -- 辅助类型和常量 --

// 优先级常量（从代码中提取）
// const PREC_CONTROL: u8 = 0; // 控制结构（语句级）
const PREC_ASSIGN: u8 = 3; // 赋值 =
const PREC_PIPE: u8 = 4; // 管道
const PREC_REDIRECT: u8 = 5; // 重定向
const PREC_LAMBDA: u8 = 6; // lambda -> ~>
const PREC_CONDITIONAL: u8 = 7; // 条件运算符 ?:
const PREC_LOGICAL_OR: u8 = 8; // 逻辑或 ||
const PREC_LOGICAL_AND: u8 = 9; // 逻辑与 &&
const PREC_COMPARISON: u8 = 10; // 比较运算
const PREC_ADD_SUB: u8 = 11; // 加减
const PREC_MUL_DIV: u8 = 12; // 乘除模
const PREC_POWER: u8 = 13; // 幂运算 **
const PREC_UNARY: u8 = 14; // 单目运算符 ! - ++ --
const PREC_FUNC_ARGS: u8 = 15;
const PREC_FUNC_NAME: u8 = 16;
const PREC_INDEX: u8 = 17; // 索引运算符 @
// -- 辅助结构 --
#[derive(Debug)]
struct OperatorInfo {
    symbol: &'static str,
    precedence: u8,
    right_associative: bool,
    kind: OperatorKind,
}
#[derive(Debug)]
enum OperatorKind {
    Prefix,
    // Postfix,
    Infix,
}
impl OperatorInfo {
    fn new(
        symbol: &'static str,
        precedence: u8,
        right_associative: bool,
        kind: OperatorKind,
    ) -> Self {
        Self {
            symbol,
            precedence,
            right_associative,
            kind,
        }
    }
}
// -- Pratt 解析器核心结构 --

/// 基于优先级0
fn parse_expr(input: Tokens) -> IResult<Tokens, Expression, SyntaxError> {
    // dbg!("--parse--");
    let (input, got) = PrattParser::parse_expr_with_precedence(input, 0)?;
    dbg!(&input.slice, &got);
    Ok((input, got))
}

struct PrattParser;
/// Pratt解析器增强实现, 基于优先级的表达式解析
impl PrattParser {
    fn parse_expr_with_precedence(
        input: Tokens<'_>,
        min_prec: u8,
    ) -> IResult<Tokens<'_>, Expression, SyntaxError> {
        // if input.is_empty(){
        //     return SyntaxError();
        // }
        // let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; // 消费换行符
        if input.is_empty() {
            // dbg!("---break0---");
            return Err(nom::Err::Error(SyntaxError::Expected {
                input: input.get_str_slice(),
                expected: "expression prefix",
                found: None,
                hint: Some("EOF while parsing expression"),
            }));
        }
        // 阶段1：解析前缀元素（基础值/一元运算符）
        let (mut input, mut lhs) = if min_prec == PREC_FUNC_ARGS {
            parse_prefix_argument(input)?
        } else if min_prec >= PREC_FUNC_NAME {
            parse_prefix_atomic(input)?
        } else {
            parse_prefix(input)?
        };
        // dbg!("=======prefix=======>", input, &lhs);
        // 阶段2：循环处理中缀运算符
        loop {
            // 获取当前运算符信息
            let op_info = match input.first() {
                Some(t) => {
                    if t.kind == TokenKind::LineBreak {
                        break;
                    }
                    Self::get_operator_info(t.text(input))
                }
                None => break, //未找到退出
            };
            // dbg!(&op_info);

            match op_info {
                Some(op) => {
                    if op.precedence < min_prec {
                        // dbg!("低于当前优先级则退出", op.precedence, min_prec);
                        break; // 低于当前优先级则退出
                    }

                    // 处理右结合运算符
                    let next_min_prec = if op.right_associative {
                        op.precedence
                    } else {
                        op.precedence + 1
                    };

                    // 阶段3：递归解析右侧表达式
                    input = input.skip_n(1);
                    // 🔴 递归前检查输入是否为空
                    // if input.is_empty() {
                    //     return Err(nom::Err::Failure(SyntaxError::Expected {
                    //         input: input.get_str_slice(),
                    //         expected: "expression after operator",
                    //         found: None,
                    //         hint: None,
                    //     }));
                    // }
                    if input.is_empty() {
                        // dbg!("---break1---");
                        break;
                    }
                    // dbg!(&input);
                    let (new_input, rhs) = Self::parse_expr_with_precedence(input, next_min_prec)?;
                    // dbg!(&new_input, &rhs);
                    // 阶段4：构建AST节点
                    input = new_input;
                    lhs = Self::build_ast(input, op, lhs, rhs)?;
                    // dbg!(&lhs, &input);
                    if input.is_empty() {
                        // dbg!("---break2---");
                        break;
                    }
                }
                None => break,
            }
        }

        Ok((input, lhs))
    }
    // 运算符元数据
    fn get_operator_info(t: &str) -> Option<OperatorInfo> {
        match t {
            // 赋值运算符（右结合）
            "=" => Some(OperatorInfo::new(
                "=",
                PREC_ASSIGN,
                true,
                OperatorKind::Infix,
            )),
            ":=" => Some(OperatorInfo::new(
                ":=",
                PREC_ASSIGN,
                true,
                OperatorKind::Infix,
            )),
            // lambda
            "->" => Some(OperatorInfo::new(
                "->",
                PREC_LAMBDA,
                true,
                OperatorKind::Infix,
            )),
            "~>" => Some(OperatorInfo::new(
                "~>",
                PREC_LAMBDA,
                true,
                OperatorKind::Infix,
            )),
            // 索引符
            "@" => Some(OperatorInfo::new(
                "@",
                PREC_INDEX,
                false,
                OperatorKind::Infix,
            )),
            "." => Some(OperatorInfo::new(
                ".",
                PREC_INDEX,
                false,
                OperatorKind::Infix,
            )),
            // 加减运算符
            "+" => Some(OperatorInfo::new(
                "+",
                PREC_ADD_SUB,
                false,
                OperatorKind::Infix,
            )),
            "-" => Some(OperatorInfo::new(
                "-",
                PREC_ADD_SUB,
                false,
                OperatorKind::Infix,
            )),
            // 乘除模运算符
            "*" => Some(OperatorInfo::new(
                "*",
                PREC_MUL_DIV,
                false,
                OperatorKind::Infix,
            )),
            "/" => Some(OperatorInfo::new(
                "/",
                PREC_MUL_DIV,
                false,
                OperatorKind::Infix,
            )),
            "%" => Some(OperatorInfo::new(
                "%",
                PREC_MUL_DIV,
                false,
                OperatorKind::Infix,
            )),
            // 幂运算符
            "**" => Some(OperatorInfo::new(
                "**",
                PREC_POWER,
                true,
                OperatorKind::Infix,
            )),
            // 单目前缀运算符
            "!" => Some(OperatorInfo::new(
                "!",
                PREC_POWER,
                true,
                OperatorKind::Prefix,
            )),
            "++" => Some(OperatorInfo::new(
                "++",
                PREC_POWER,
                false,
                OperatorKind::Prefix,
            )),
            "--" => Some(OperatorInfo::new(
                "--",
                PREC_POWER,
                false,
                OperatorKind::Prefix,
            )),
            // 逻辑运算符
            "&&" => Some(OperatorInfo::new(
                "&&",
                PREC_LOGICAL_AND,
                false,
                OperatorKind::Infix,
            )),
            "||" => Some(OperatorInfo::new(
                "||",
                PREC_LOGICAL_OR,
                false,
                OperatorKind::Infix,
            )),
            // 比较运算符
            "==" => Some(OperatorInfo::new(
                "==",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            "!=" => Some(OperatorInfo::new(
                "!=",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            ">" => Some(OperatorInfo::new(
                ">",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            "<" => Some(OperatorInfo::new(
                "<",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            ">=" => Some(OperatorInfo::new(
                ">=",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            "<=" => Some(OperatorInfo::new(
                "<=",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            // 匹配
            "~~" => Some(OperatorInfo::new(
                "~~",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            "~=" => Some(OperatorInfo::new(
                "~=",
                PREC_COMPARISON,
                false,
                OperatorKind::Infix,
            )),
            // 三目
            "?" => Some(OperatorInfo::new(
                "?",
                PREC_CONDITIONAL, // 优先级设为2
                true,             // 右结合
                OperatorKind::Infix,
            )),
            ":" => Some(OperatorInfo::new(
                ":",
                PREC_CONDITIONAL,
                false, // 非结合（仅作为分隔符）
                OperatorKind::Infix,
            )),
            // ... 管道操作符 ...
            "|" => Some(OperatorInfo::new(
                "|",
                PREC_PIPE, // 例如设为 4（低于逻辑运算符）
                false,
                OperatorKind::Infix,
            )),
            // ... 重定向操作符 ...
            "<<" => Some(OperatorInfo::new(
                "<<",
                PREC_REDIRECT, // 与赋值同级
                false,
                OperatorKind::Infix,
            )),
            ">>" => Some(OperatorInfo::new(
                ">>",
                PREC_REDIRECT,
                false,
                OperatorKind::Infix,
            )),
            ">>>" => Some(OperatorInfo::new(
                ">>>",
                PREC_ASSIGN,
                false,
                OperatorKind::Infix,
            )),
            _ => None,
        }
    }

    fn build_bin_ast(
        input: Tokens,
        op: OperatorInfo,
        lhs: Expression,
        rhs: Expression,
    ) -> Result<Expression, nom::Err<SyntaxError>> {
        match op.symbol {
            "." | "@" | "+" | "-" | "*" | "/" | "%" | "**" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Box::new(lhs),
                Box::new(rhs),
            )),
            "&&" | "||" => Ok(Expression::BinaryOp(
                op.symbol.into(),
                Box::new(lhs),
                Box::new(rhs),
            )),
            // "|>" => Expression::Pipe(Box::new(lhs), Box::new(rhs)),
            "=" => {
                // 确保左侧是符号
                match lhs.to_symbol() {
                    // .unwrap_or_else(|_| panic!("Invalid assignment target: {:?}", lhs));
                    Ok(name) => Ok(Expression::Assign(name.to_string(), Box::new(rhs))),
                    _ => {
                        eprintln!("invalid left-hand-side: {:?}", lhs);
                        Err(SyntaxError::expected(
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
                Box::new(lhs),
                Box::new(rhs),
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

            //             Err(SyntaxError::expected(
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
                    Expression::Group(boxed_expr) => match *boxed_expr {
                        Expression::List(elements) => elements
                            .into_iter()
                            .map(|e| e.to_symbol().map(|s| s.to_string()))
                            .collect::<Result<Vec<_>, _>>(),
                        // 处理单个参数 (x)
                        // expr => match expr {
                        // Expression::List(s) => return Ok(Expression::List(s)),
                        Expression::Symbol(s) => Ok(vec![s]),
                        _ => {
                            return Err(SyntaxError::expected(
                                input.get_str_slice(),
                                "symbol in parameter list",
                                Some(boxed_expr.type_name()),
                                "put only valid symbols in lambda param list".into(),
                            ));
                        } // },
                    },
                    // 处理无括号单参数
                    Expression::Symbol(name) => Ok(vec![name]),
                    _ => {
                        eprintln!("invalid lambda-param {:?}", lhs);
                        return Err(SyntaxError::expected(
                            input.get_str_slice(),
                            "symbol or parameter list",
                            Some(lhs.to_string()),
                            "Lambda requires valid parameter list".into(),
                        ));
                    }
                };

                // 自动包装body为代码块
                let body = match rhs {
                    // 已有代码块保持原样
                    Expression::Do(_) => rhs,
                    // 分组表达式展开
                    Expression::Group(boxed_expr) => *boxed_expr,
                    // 其他表达式自动包装
                    _ => Expression::Do(vec![rhs]),
                };

                // 构建Lambda表达式
                Ok(Expression::Lambda(
                    params.unwrap(),
                    Box::new(body),
                    Environment::new(),
                ))
            }
            "~>" => {
                // 参数处理
                match lhs.to_symbol() {
                    // 解析体部分
                    Ok(name) => Ok(Expression::Macro(name.to_string(), Box::new(rhs))),
                    _ => {
                        eprintln!("invalid macro-param {:?}", lhs);

                        Err(SyntaxError::expected(
                            input.get_str_slice(),
                            "symbol",
                            Some(lhs.to_string()),
                            "macro params must be symbol".into(),
                        ))
                    }
                }
            }
            "?" => {
                let (true_expr, false_expr) = match rhs {
                    Expression::BinaryOp(op, t, f) if op == ":" => (t, f),
                    _ => {
                        eprintln!("invalid conditional ?: {:?}", rhs);

                        return Err(SyntaxError::expected(
                            input.get_str_slice(),
                            "symbol",
                            Some(lhs.to_string()),
                            "Invalid conditional expression".into(),
                        ));
                    }
                };
                Ok(Expression::If(Box::new(lhs), true_expr, false_expr))
            }
            ":" => Ok(Expression::BinaryOp(
                ":".into(),
                Box::new(lhs),
                Box::new(rhs),
            )),
            ":=" => {
                // 确保左侧是符号
                match lhs.to_symbol() {
                    Ok(name) => Ok(Expression::Assign(
                        name.to_string(),
                        Box::new(Expression::Quote(Box::new(rhs))),
                    )),
                    _ => {
                        eprintln!("invalid left-hide-side {:?}", lhs);

                        Err(SyntaxError::expected(
                            input.get_str_slice(),
                            "symbol",
                            Some(lhs.to_string()),
                            "only assign to symbol allowed".into(),
                        ))
                    }
                }
            }
            "+=" | "-=" | "*=" | "/=" => {
                let base_op = op.symbol.trim_end_matches('=');
                let new_rhs =
                    Expression::BinaryOp(base_op.into(), Box::new(lhs.clone()), Box::new(rhs));
                Ok(Expression::Assign(lhs.to_string(), Box::new(new_rhs)))
            }
            // "|" => Expression::Pipe(Box::new(lhs), Box::new(rhs)),
            // "|" => Expression::Apply(Box::new("fs@pipe"), Box::new(rhs)),
            // "|" => Expression::Apply(
            //     Box::new(Expression::Symbol("|".to_string())),
            //     vec![lhs, rhs],
            // ),
            // "<<" => {
            //     let content = Expression::Apply(
            //         Expression::BinaryOp(
            //             "@".to_string(),
            //             Box::new(Expression::Symbol("fs".to_string())),
            //             Box::new(Expression::Symbol("read".to_string())),
            //         ),
            //         vec![rhs],
            //     );
            //     Expression::Apply(
            //         Box::new(Expression::Symbol("|".to_string())),
            //         vec![lhs, rhs],
            //     )
            // }
            "|" => Ok(Expression::BinaryOp(
                "|".into(),
                Box::new(lhs),
                Box::new(rhs),
            )),

            "<<" => Ok(Expression::BinaryOp(
                "<<".into(),
                Box::new(lhs),
                Box::new(rhs),
            )),

            ">>" => Ok(Expression::BinaryOp(
                ">>".into(),
                Box::new(lhs),
                Box::new(rhs),
            )),

            ">>>" => Ok(Expression::BinaryOp(
                ">>>".into(),
                Box::new(lhs),
                Box::new(rhs),
            )),

            // "<<" | ">>" | ">>>" => {
            //     let redirect_type = match op.symbol {
            //         "<<" => RedirectType::Input,
            //         ">>" => RedirectType::Overwrite,
            //         ">>>" => RedirectType::Append,
            //         _ => unreachable!(),
            //     };
            //     Expression::Redirect(
            //         redirect_type,
            //         Box::new(lhs),
            //         Box::new(rhs), // rhs 应为文件名表达式
            //     )
            // }
            _ => {
                unreachable!()
            }
        }
    }

    fn build_ast(
        input: Tokens,
        op: OperatorInfo,
        lhs: Expression,
        rhs: Expression,
    ) -> Result<Expression, nom::Err<SyntaxError>> {
        match op.kind {
            // 需要扩展OperatorInfo包含kind字段
            OperatorKind::Prefix => match op.symbol {
                "++" | "--" | "!" => Ok(Expression::UnaryOp(op.symbol.into(), Box::new(rhs), true)),
                _ => unreachable!(),
            },
            // OperatorKind::Postfix => Expression::UnaryOp(op.symbol.into(), Box::new(rhs), false), //differ
            OperatorKind::Infix => match op.symbol {
                // 原有双目运算符处理...
                _ => Self::build_bin_ast(input, op, lhs, rhs),
            },
        }
    }
}

/// -- 函数名的基础解析 --
fn parse_prefix_atomic(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!("--parse_prefix_atomic--", input.slice);

    alt((
        map(parse_symbol, Expression::Symbol),
        //虽然函数名不需要整数，但所有@索引由于优先级最高，都会来到这里
        map(parse_integer, Expression::Integer),
        // map(parse_float, Expression::Float),
        // map(parse_string, Expression::String),
        // map(parse_boolean, Expression::Boolean),
    ))(input)
}
/// -- 函数参数解析 --
fn parse_prefix_argument(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!("--parse_prefix--", input.slice);
    let (input, prefix) = alt((
        parse_group,
        parse_index_or_slice, //索引或切片 避免被当作函数调用，应先于函数调用。其中不能包含{}[],否则会影响map,list。
        // parse_func_call,  // func(a,b)
        // parse_block,          // 优先解析block，从而让lambda 可以识别为do块，而不是字典解析。
        parse_list,
        parse_map, // 应后于所有block,func block调用。
        map(parse_symbol, Expression::Symbol),
        parse_literal,
        parse_unary,
        parse_none, // parse_conditional,
                    // |inp| Ok((inp, Expression::None)), //for anary operators.
    ))(input)?;
    // .expect("NO ANY PREFIX");
    Ok((input, prefix))
}
/// -- 左侧基础表达式解析 --
fn parse_prefix(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!("--parse_prefix--", input.slice);
    let (input, prefix) = alt((
        parse_param_group, // new, lambda params
        parse_group,
        parse_control_flow,   // ✅ 新增：允许if作为表达式
        parse_index_or_slice, //索引或切片 避免被当作函数调用，应先于函数调用。其中不能包含{}[],否则会影响map,list。
        parse_func_call,      // func(a,b)
        parse_func_flat_call, //函数调用 func a b
        parse_map,            // 应先于block调用。
        parse_block,          // 优先解析block，从而让lambda 可以识别为do块。
        parse_list,
        map(parse_symbol, Expression::Symbol),
        parse_literal,
        parse_unary,
        parse_none, // parse_conditional,
                    // |inp| Ok((inp, Expression::None)), //for anary operators.
    ))(input)?;
    // .expect("NO ANY PREFIX");
    Ok((input, prefix))
}
// 统一控制流解析（适用于语句和表达式）
fn parse_control_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        parse_if_flow,    // 同时处理if语句和if表达式
        parse_match_flow, // 同时处理match语句和match表达式
        parse_while_flow, // 同时处理while/for循环
        parse_for_flow,
    ))(input)
}
// -- 完整运算符支持 --
fn parse_group(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    delimited(
        text("("),
        cut(map(parse_expr, |e| Expression::Group(Box::new(e)))),
        cut(text_close(")")),
    )(input)
}

fn parse_list(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    delimited(
        text("["),
        cut(map(
            separated_list0(text(","), parse_expr),
            Expression::List,
        )),
        // opt(text(",")), //TODO 允许末尾，
        cut(text_close("]")),
    )(input)
}

// 增参数解析函数
fn parse_param(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, (String, Option<Expression>), SyntaxError> {
    alt((
        // 带默认值的参数解析分支
        map(
            separated_pair(
                parse_symbol,
                text("="),
                // 限制只能解析基本类型表达式
                parse_literal,
            ),
            |(name, expr)| (name, Some(expr)), // 将结果包装为Some
        ),
        // 普通参数解析分支
        map(parse_symbol, |s| (s, None)), // , 1+2 also match first symbol, so failed in ) parser.
    ))(input)
}
// 函数参数列表解析
fn parse_param_list(
    input: Tokens<'_>,
) -> IResult<Tokens<'_>, Vec<(String, Option<Expression>)>, SyntaxError> {
    let (input, _) = cut(text("("))(input).map_err(|_| {
        SyntaxError::expected(
            input.get_str_slice(),
            "function params declare",
            None,
            Some("add something like (x,y)"),
        )
    })?;
    let (input, params) = separated_list0(text(","), parse_param)(input)?;
    let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; //允许可选回车
    // 如果还有其他字符，应报错
    // dbg!(&input, &params);
    if !input.is_empty() {
        match input.first() {
            Some(&token) if token.text(input) != ")" => {
                dbg!(token.text(input));
                return Err(SyntaxError::expected(
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
    Ok((input, params))
}
// lambda参数
fn parse_param_group(input: Tokens) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, expr) = delimited(
        text("("),
        alt((
            // 参数列表特殊处理
            map(separated_list1(text(","), parse_symbol), |symbols| {
                Expression::List(symbols.into_iter().map(|s| Expression::Symbol(s)).collect())
            }),
            parse_expr, // 常规表达式
        )),
        text_close(")"),
    )(input)?;
    Ok((input, Expression::Group(Box::new(expr))))
}
// 函数定义解析
fn parse_fn_declare(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("fn")(input)?;
    let (input, name) = cut(parse_symbol)(input).map_err(|_| {
        eprintln!("mising fn name?");
        // why not raise?
        SyntaxError::expected(
            input.get_str_slice(),
            "function name",
            None,
            Some("add a name for your function"),
        )
    })?;
    let (input, params) = cut(parse_param_list)(input)?; // 使用新参数列表
    let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; //允许可选回车

    // 无函数体应报错
    // dbg!(&input, &params);
    if match input.first() {
        Some(&token) if token.text(input).ne("{") => true,
        None => true,
        _ => false,
    } {
        eprintln!("mising fn body?");
        // why not raise?
        return Err(SyntaxError::expected(
            input.get_str_slice(),
            "valid function body declare",
            None,
            Some("add a function body like {...}"),
        ));
    }
    let (input, body) = cut(parse_block)(input)?;

    Ok((
        input,
        Expression::Function(
            name,
            params,
            Box::new(body),
            Environment::new(), // 捕获当前环境（需在调用时处理）
        ),
    ))
}
// return statement
fn parse_return(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("return")(input)?;
    let (input, expr) = opt(parse_expr)(input)?;
    Ok((
        input,
        Expression::Return(Box::new(expr.unwrap_or(Expression::None))),
    ))
}
// -- 函数调用解析增强 --
fn parse_func_call(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!("---func call---", input);
    let (input, ident) = PrattParser::parse_expr_with_precedence(input, PREC_FUNC_NAME)?;
    // let (input, ident) = alt((
    //     parse_index_expr,
    //     parse_symbol.map(|s| Expression::Symbol(s)),
    // ))(input)?;
    let (input, args) = delimited(
        text("("),
        cut(separated_list0(text(","), parse_expr)),
        cut(text_close(")")),
    )(input)?;
    // dbg!(&ident, &args);

    Ok((input, Expression::Apply(Box::new(ident), args)))
    // Ok((
    //     input,
    //     Expression::Apply(Box::new(Expression::Symbol(ident)), args),
    // ))
}

fn parse_func_flat_call(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, ident) = PrattParser::parse_expr_with_precedence(input, PREC_FUNC_NAME)?;
    // let (input, ident) = alt((
    //     parse_index_expr,
    //     parse_symbol.map(|s| Expression::Symbol(s)),
    // ))(input)?;
    // let (input, args) = many1(parse_expr)(input)?;
    let (input, args) =
        many1(|inp| PrattParser::parse_expr_with_precedence(inp, PREC_FUNC_ARGS))(input)?;
    Ok((input, Expression::Apply(Box::new(ident), args)))
}
// -- 入口函数与脚本解析 --

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
    // let tokens = Tokens {
    //     str: &str,
    //     slice: token_vec.as_slice(),
    // };

    // for window in tokens.slice.windows(2) {
    //     let (a, b) = (window[0], window[1]);
    //     if is_symbol_like(a.kind)
    //         && is_symbol_like(b.kind)
    //         && a.text(tokens) != "@"
    //         && b.text(tokens) != "@"
    //     {
    //         return Err(nom::Err::Failure(SyntaxError::Expected {
    //             input: a.range.join(b.range),
    //             expected: "whitespace",
    //             found: Some(b.text(tokens).to_string()),
    //             hint: None,
    //         }));
    //     }
    // }

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
            eprintln!("parse error");
            Err(e)
        }
    }
    // dbg!(&expr);

    // Ok(expr)
}

// -- 其他辅助函数保持与用户提供代码一致 --
#[inline]
fn kind(kind: TokenKind) -> impl Fn(Tokens<'_>) -> IResult<Tokens<'_>, StrSlice, SyntaxError> {
    move |input: Tokens<'_>| match input.first() {
        Some(&token) if token.kind == kind => Ok((input.skip_n(1), token.range)),
        _ => Err(nom::Err::Error(SyntaxError::InternalError)),
    }
}

#[inline]
fn text<'a>(text: &'a str) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxError> {
    move |input: Tokens<'a>| match input.first() {
        Some(&token) if token.text(input) == text => Ok((input.skip_n(1), token)),
        _ => Err(nom::Err::Error(SyntaxError::InternalError)),
    }
}
#[inline]
fn text_close<'a>(
    text: &'static str,
) -> impl Fn(Tokens<'a>) -> IResult<Tokens<'a>, Token, SyntaxError> {
    move |input: Tokens<'a>| match input.first() {
        Some(&token) if token.text(input) == text => Ok((input.skip_n(1), token)),
        _ => Err(nom::Err::Error(SyntaxError::unclosed_delimiter(
            input.get_str_slice(),
            text,
        ))),
    }
}

// -- 字面量解析 --
fn parse_literal(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        map(parse_integer, Expression::Integer),
        map(parse_float, Expression::Float),
        map(parse_string, Expression::String),
        map(parse_string_raw, Expression::String),
        map(parse_boolean, Expression::Boolean),
    ))(input)
}
fn parse_none(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!("---parsing none---", &input);
    match text("None")(input) {
        Ok(_) => Ok((input, Expression::None)),
        _ => Err(SyntaxError::expected(
            input.get_str_slice(),
            "None",
            None,
            None,
        )),
    }
    // if let Ok((input, _)) = text("None")(input) {
    //     Ok((input, Expression::None))
    // }
    // SyntaxError::expected(input.get_str_slice(), "None or ()", None, None)
}

// 映射解析
fn parse_map(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // 不能用cut，防止map识别失败时，影响后面的block解析。
    let (input, _) = text("{")(input)?;
    let (input, pairs) = separated_list0(
        text(","),
        separated_pair(parse_symbol, text(":"), parse_literal),
    )(input)?;
    let (input, _) = text_close("}")(input)?;

    // Ok((input, Expression::Map(pairs)))
    Ok((input, Expression::Map(pairs.into_iter().collect())))
}

#[inline]
fn parse_symbol(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxError> {
    map(kind(TokenKind::Symbol), |t| t.to_str(input.str).to_string())(input)
}

#[inline]
fn parse_string(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxError> {
    let (input, string) = kind(TokenKind::StringLiteral)(input)?;
    Ok((
        input,
        snailquote::unescape(string.to_str(input.str)).unwrap(),
    ))
}
#[inline]
fn parse_string_raw(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxError> {
    let (input, expr) = kind(TokenKind::StringRaw)(input)?;
    let raw_str = expr.to_str(input.str);

    // 检查首尾单引号
    if raw_str.len() >= 2 {
        // 通过StrSlice直接计算子范围
        let start = expr.start() + 1;
        let end = expr.end() - 1;
        let content = input.str.get(start..end); // 截取中间部分
        Ok((input, content.to_str(input.str).to_string()))
    } else {
        Err(SyntaxError::unrecoverable(
            expr,
            "raw string enclosed in single quotes",
            Some(raw_str.to_string()),
            Some("raw strings must surround with '"),
        ))
    }
}

fn parse_integer(input: Tokens<'_>) -> IResult<Tokens<'_>, Int, SyntaxError> {
    let (input, num) = kind(TokenKind::IntegerLiteral)(input)?;
    let num = num.to_str(input.str).parse::<Int>().map_err(|e| {
        SyntaxError::unrecoverable(num, "integer", Some(format!("error: {}", e)), None)
    })?;
    Ok((input, num))
}

fn parse_float(input: Tokens<'_>) -> IResult<Tokens<'_>, f64, SyntaxError> {
    let (input, num) = kind(TokenKind::FloatLiteral)(input)?;
    let num = num.to_str(input.str).parse::<f64>().map_err(|e| {
        SyntaxError::unrecoverable(
            num,
            "float",
            Some(format!("error: {}", e)),
            Some("valid floats can be written like 1.0 or 5.23"),
        )
    })?;
    Ok((input, num))
}

#[inline]
fn parse_boolean(input: Tokens<'_>) -> IResult<Tokens<'_>, bool, SyntaxError> {
    map(kind(TokenKind::BooleanLiteral), |s| {
        s.to_str(input.str) == "True"
    })(input)
}

// ================== 控制结构解析 ==================
// 核心解析流程架构
pub fn parse_script_tokens(
    input: Tokens<'_>,
    // require_eof: bool,
) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    if input.is_empty() {
        return Ok((input, Expression::None));
    }
    // 阶段1：解析语句序列（控制结构在此处理）
    // dbg!("------>1", input);
    // let (input, mut statements) = many0(parse_statement)(input)?;
    let (input, statements) = many0(terminated(
        parse_statement,
        alt((kind(TokenKind::LineBreak), eof_slice)), // 允许换行符作为语句分隔
    ))(input)?;

    if !input.is_empty() {
        dbg!("-----==>2", &input.slice, &statements);
        eprintln!("unrecognized satement");
    }
    // if !input.is_empty() {
    //     // 阶段2：解析最后可能的表达式（无显式分号的情况）
    //     let (input, last) = opt(terminated(
    //         parse_expr, // 完整表达式解析
    //         // PrattParser::parse_expr, // 完整表达式解析
    //         opt(kind(TokenKind::LineBreak)),
    //     ))(input)?;
    //     dbg!("-----==>3", &input.slice, &last);
    //     dbg!(input.slice, &last);

    //     // 阶段3：合并结果
    //     if let Some(expr) = last {
    //         statements.push(expr);
    //     }
    //     // dbg!("-----==>4", &statements);

    //     // 新增：清理所有末尾换行符
    //     // let (input, _) = many0(kind(TokenKind::LineBreak))(input)?;
    //     // // 阶段4：严格模式下的EOF验证
    //     // if require_eof {
    //     //     // input.is_empty()
    //     //     eof(input)
    //     //         .map_err(|_: nom::Err<SyntaxError>| {
    //     //             SyntaxError::expected(input.get_str_slice(), "end of input", None, None)
    //     //         })?
    //     //         .0;
    //     // }
    // }

    Ok((input, Expression::Do(statements)))
}
// 语句解析器（顶层结构）
fn parse_statement(mut input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // dbg!(input);
    loop {
        let (input_pure, lbk) = opt(kind(TokenKind::LineBreak))(input)?; // 消费换行符
        if lbk.is_none() {
            break;
        }
        input = input_pure
    }
    let (input, statement) = alt((
        parse_fn_declare, // 函数声明（仅语句级）
        // parse_import,        // 模块导入（仅语句级）
        parse_control_flow, // 控制流（可嵌套在表达式中）
        // parse_assign,        // 赋值语句

        // func
        // parse_lambda,
        // 声明和赋值
        parse_lazy_assign,
        parse_declare,
        parse_assign,
        parse_del,
        // call
        // parse_apply,
        // 兜底：表达式语句
        parse_return, //return in func
        // parse_expr,   // 完整表达式
        terminated(
            parse_expr, // 完整表达式
            opt(alt((
                // 必须包含语句终止符
                kind(TokenKind::LineBreak),
                eof_slice, // 允许文件末尾无终止符
            ))),
        ),
    ))(input)?;
    // let (input, _) = opt(kind(TokenKind::LineBreak))(input)?; // 消费换行符

    // dbg!(&input, &statement);
    Ok((input, statement))
}

// IF语句解析（支持else if链）
fn parse_if_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("if")(input)?;
    let (input, cond) = parse_expr(input)?;
    let (input, then_block) = parse_block_or_expr(input)?;

    // 解析else分支
    let (input, else_branch) = opt(preceded(
        text("else"),
        alt((
            parse_if_flow, // else if
            parse_block,   // else
        )),
    ))(input)?;

    let els = else_branch.unwrap_or(Expression::None);
    Ok((
        input,
        Expression::If(Box::new(cond), Box::new(then_block), Box::new(els)),
    ))
}

// WHILE循环解析
fn parse_while_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("while")(input)?;
    let (input, cond) = parse_expr(input)?;
    let (input, body) = parse_block(input)?;

    Ok((input, Expression::While(Box::new(cond), Box::new(body))))
}

// FOR循环解析
fn parse_for_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("for")(input)?;
    let (input, pattern) = parse_symbol(input)?; // 或更复杂的模式匹配
    let (input, _) = text("in")(input)?;
    let (input, iterable) = parse_expr(input)?;
    let (input, body) = parse_block(input)?;

    Ok((
        input,
        Expression::For(pattern, Box::new(iterable), Box::new(body)),
    ))
}

// MATCH表达式解析
fn parse_match_flow(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("match")(input)?;
    let (input, matched) = parse_expr(input)?;
    let (input, _) = text("{")(input)?;

    // 解析多个匹配分支
    let (input, expr_map) = separated_list1(
        text(","),
        separated_pair(parse_pattern, text("=>"), parse_expr),
    )(input)?;

    let (input, _) = text_close("}")(input)?;
    let branches = expr_map
        .into_iter()
        .map(|(pattern, expr)| (pattern, Box::new(expr)))
        .collect::<Vec<_>>();
    Ok((input, Expression::Match(Box::new(matched), branches)))
}

// ================== 条件运算符?: ==================

// 条件运算符处理

// 一元运算符具体实现
fn parse_unary(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    // 匹配前缀运算符 !、++、-- 等
    let (input, op) = alt((text("!"), text("++"), text("--")))(input)?;
    let (input, expr) = PrattParser::parse_expr_with_precedence(input, PREC_UNARY)?; // 递归解析后续表达式
    Ok((
        input,
        Expression::UnaryOp(op.text(input).to_string(), Box::new(expr), true), // true 表示前缀
    ))
}

// ================== 辅助函数 ==================
// 动态识别块或表达式
fn parse_block_or_expr(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    alt((
        parse_block, // 优先识别 {...} 块
        parse_expr,  // 单行表达式（如 x > y ? a : b）
    ))(input)
}
// 解析代码块（带花括号）
fn parse_block(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, block) = delimited(
        text("{"),
        cut(map(
            many0(terminated(parse_statement, opt(kind(TokenKind::LineBreak)))),
            |stmts| Expression::Do(stmts),
        )),
        cut(text_close("}")),
    )(input)?;
    // dbg!(&block);
    Ok((input, block))
}

// 赋值解析
// 新增 parse_assign 函数
fn parse_assign(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, symbol) = parse_symbol(input)?;
    let (input, _) = text("=")(input)?;
    // let (input, expr) = alt((
    //     parse_conditional, // 支持条件表达式作为右值 //TODO del
    //     parse_expr,
    // ))(input)?;
    let (input, expr) = PrattParser::parse_expr_with_precedence(input, PREC_ASSIGN + 1)?;
    // 验证语句终止符
    // let (input, _) = cut(alt((kind(TokenKind::LineBreak), eof_slice)))(input)?;
    Ok((input, Expression::Assign(symbol, Box::new(expr))))
}
// 延迟赋值解析逻辑
fn parse_lazy_assign(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("let")(input)?;
    let (input, symbol) = parse_symbol(input)?;
    let (input, _) = text(":=")(input)?; // 使用:=作为延迟赋值符号
    let (input, expr) = parse_expr(input)?;
    // dbg!(&expr);
    Ok((
        input,
        Expression::Assign(symbol, Box::new(Expression::Quote(Box::new(expr)))),
    ))
}

fn parse_declare(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("let")(input)?;

    // 解析逗号分隔的多个符号, 允许重载操作符
    let (input, symbols) = separated_list0(text(","), alt((parse_symbol, parse_operator)))(input)
        .map_err(|_| {
        SyntaxError::unrecoverable(
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
                return Ok((
                    input,
                    Expression::Declare(symbols[0].clone(), Box::new(e[0].clone())),
                ));
            }
            (0..symbols.len())
                .map(|i| Expression::Declare(symbols[i].clone(), Box::new(e[0].clone())))
                .collect()
        }
        Some(e) if e.len() == symbols.len() => (0..symbols.len())
            .map(|i| Expression::Declare(symbols[i].clone(), Box::new(e[i].clone())))
            .collect(),
        Some(e) => {
            return Err(SyntaxError::unrecoverable(
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
    Ok((input, Expression::Do(assignments)))
}

fn parse_del(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, _) = text("del")(input)?;
    let (input, symbol) = parse_symbol(input).map_err(|_| {
        SyntaxError::unrecoverable(
            input.get_str_slice(),
            "symbol",
            Some("no symbol".into()),
            Some("you can only del symbol"),
        )
    })?;
    Ok((input, Expression::Del(symbol)))
}

fn parse_operator(input: Tokens<'_>) -> IResult<Tokens<'_>, String, SyntaxError> {
    map(kind(TokenKind::Operator), |t| {
        t.to_str(input.str).to_string()
    })(input)
}
// 模式匹配解析（简化示例）
fn parse_pattern(input: Tokens<'_>) -> IResult<Tokens<'_>, Pattern, SyntaxError> {
    alt((
        map(text("_"), |_| Pattern::Bind("_".to_string())), // 将_视为特殊绑定
        map(parse_symbol, Pattern::Bind),
        map(parse_literal, |lit| Pattern::Literal(Box::new(lit))),
    ))(input)
}
// 自定义EOF解析器，返回StrSlice类型
fn eof_slice(input: Tokens<'_>) -> IResult<Tokens<'_>, StrSlice, SyntaxError> {
    if input.is_empty() {
        Ok((input, StrSlice::default()))
    } else {
        Err(nom::Err::Error(SyntaxError::Expected {
            input: input.get_str_slice(),
            expected: "end of input",
            found: None,
            hint: Some("Check your input"),
        }))
    }
}

// 新增切片参数解析
fn parse_slice_params(input: Tokens<'_>) -> IResult<Tokens<'_>, SliceParams, SyntaxError> {
    let (input, parts) = separated_list0(
        text(":"),
        opt(alt((
            map(parse_integer, Expression::Integer),
            map(parse_symbol, Expression::Symbol),
        ))),
        // opt(alt((
        // parse_expr,
        // map(kind(TokenKind::LineContinuation), |_| Expression::None),
        // ))),
    )(input)?;

    Ok((
        input,
        SliceParams {
            start: parts.get(0).and_then(|v| v.clone().map(|e| Box::new(e))),
            end: parts.get(1).and_then(|v| v.clone().map(|e| Box::new(e))),
            step: parts.get(2).and_then(|v| v.clone().map(|e| Box::new(e))),
        },
    ))
}

// 新增索引/切片解析
fn parse_index_or_slice(input: Tokens<'_>) -> IResult<Tokens<'_>, Expression, SyntaxError> {
    let (input, target) = alt((
        map(parse_symbol, Expression::Symbol),
        map(parse_string, Expression::String),
        // parse_list,
        // parse_map,
    ))(input)?;
    let (input, (params, is_slice)) = delimited(
        text("["),
        alt((
            // TODO a[2] was a[2:], changeorder not resolve.
            map(parse_slice_params, |p| (p, true)), // 切片
            map(parse_integer, |e| {
                (
                    SliceParams {
                        // 索引转切片
                        start: Some(Box::new(Expression::Integer(e))),
                        end: None,
                        step: None,
                    },
                    false,
                )
            }),
            map(parse_symbol, |e| {
                (
                    SliceParams {
                        // 索引转切片
                        start: Some(Box::new(Expression::Symbol(e))),
                        end: None,
                        step: None,
                    },
                    false,
                )
            }),
        )),
        cut(text_close("]")),
    )(input)?;

    Ok((
        input,
        match is_slice {
            true => Expression::Slice(Box::new(target), params),
            false => Expression::Index(Box::new(target), params.start.unwrap()),
        },
    ))
}
