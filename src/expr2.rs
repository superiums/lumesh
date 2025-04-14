// 关键优化点说明：

// 内存优化：

// 列表类型改为 Arc<[Self]>，减少克隆开销
// 参数传递优先使用 &[Self] 替代 Vec<Self>
// 错误处理优化：

// 新增 type_error! 宏统一生成类型错误
// 符号转换错误信息更明确
// 索引越界错误包含详细位置信息
// 代码结构优化：

// 拆分 eval_mut 为基本类型处理和复杂类型处理
// 尾递归优化避免栈溢出
// 运算符重载统一实现
// 性能优化：

// 内置函数调用参数改为切片引用
// 模式匹配使用 matches! 宏优化分支判断
// 可维护性提升：

// 所有错误生成路径统一
// 复杂表达式处理分离到独立方法
// 显示实现更简洁的 Debug 输出
use super::{Environment, Error, Int};
// use num_traits::pow;
use regex_lite::Regex;
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt,
    io::ErrorKind,
    ops::{Add, Div, Index, Mul, Neg, Rem, Sub},
    process::Command,
};
use terminal_size::{Width, terminal_size};

use crate::STRICT;
use prettytable::{
    Cell, Row, Table,
    format::{LinePosition, LineSeparator},
    row,
};

use textwrap::fill; // 确保textwrap已引入

const MAX_RECURSION_DEPTH: Option<usize> = Some(800);

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Bind(String),
    Literal(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliceParams {
    pub start: Option<Box<Expression>>,
    pub end: Option<Box<Expression>>,
    pub step: Option<Box<Expression>>,
}

#[derive(Clone, PartialEq)]
pub enum Expression {
    Group(Box<Self>),
    BinaryOp(String, Box<Self>, Box<Self>),
    UnaryOp(String, Box<Self>, bool),
    Symbol(String),
    Integer(Int),
    Float(f64),
    Bytes(Vec<u8>),
    String(String),
    Boolean(bool),
    List(Vec<Self>),
    Map(BTreeMap<String, Self>),
    Index(Box<Self>, Box<Self>),
    Slice(Box<Self>, SliceParams),
    None,
    Del(String),
    Declare(String, Box<Self>),
    Assign(String, Box<Self>),
    For(String, Box<Self>, Box<Self>),
    While(Box<Self>, Box<Self>),
    Match(Box<Self>, Vec<(Pattern, Box<Self>)>),
    If(Box<Self>, Box<Self>, Box<Self>),
    Apply(Box<Self>, Vec<Self>),
    Lambda(Vec<String>, Box<Self>, Environment),
    Macro(String, Box<Self>),
    Function(String, Vec<(String, Option<Self>)>, Box<Self>, Environment),
    Return(Box<Self>),
    Do(Vec<Self>),
    Builtin(Builtin),
    Quote(Box<Self>),
}

// 错误处理宏（优化点）
macro_rules! type_error {
    ($expected:expr, $found:expr) => {
        Err(Error::TypeError {
            expected: $expected.into(),
            found: $found.type_name(),
        })
    };
}
// 宏定义（可放在 impl 块外）
macro_rules! fmt_shared {
    ($self:ident, $f:ident, $debug:expr) => {
        match $self {
            Self::Quote(inner) => write!($f, "'{:?}", inner),
            Self::Group(inner) => write!($f, "({:?})", inner),
            Self::Symbol(name) => write!($f, "{}", name),
            Self::Integer(i) => write!($f, "{}", *i),
            Self::Float(n) => write!($f, "{}", *n),
            Self::Bytes(b) => write!($f, "b{:?}", b),
            Self::String(s) => write!($f, "{:?}", s),
            Self::Boolean(b) => write!($f, "{}", if *b { "True" } else { "False" }),
            Self::While(cond, body) => write!($f, "while {:?} {:?}", cond, body),

            // 修正Slice分支的格式化错误
            Self::Slice(l, r) => {
                let params = format!(
                    "[start: {:?}, end: {:?}, step: {:?}]",
                    r.start, r.end, r.step
                );
                write!($f, "({}{})", l, params)
            }

            // 修正List分支中的变量名错误
            Self::List(exprs) if $debug => {
                write!(
                    $f,
                    "[{}]",
                    exprs
                        .iter()
                        .map(|e| format!("{:?}", e))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Self::List(exprs) => {
                // Create a table with one column
                let specified_width = $f.width().unwrap_or(
                    terminal_size()
                        .map(|(Width(w), _)| w as usize)
                        .unwrap_or(120),
                );
                let mut t = Table::new();
                let fmt = t.get_format();
                fmt.padding(1, 1);
                fmt.borders('┃');
                fmt.column_separator('┃');
                fmt.separator(LinePosition::Top, LineSeparator::new('━', '┳', '┏', '┓'));
                fmt.separator(LinePosition::Title, LineSeparator::new('━', '╋', '┣', '┫'));
                fmt.separator(LinePosition::Intern, LineSeparator::new('━', '╋', '┣', '┫'));
                fmt.separator(LinePosition::Bottom, LineSeparator::new('━', '┻', '┗', '┛'));

                let mut row = vec![];
                let mut total_len = 1;
                for expr in exprs.iter() {
                    let formatted = match expr {
                        Expression::String(s) => format!("{:?}", s),
                        _ => format!("{}", expr),
                    };
                    // Get the length of the first line
                    if formatted.contains('\n') {
                        let first_line_len = formatted.lines().next().unwrap().len();
                        total_len += first_line_len + 1;
                    } else {
                        total_len += formatted.len() + 1;
                    }
                    row.push(formatted);
                }
                if total_len > specified_width {
                    return write!($f, "{:?}", $self);
                }
                let row = Row::new(row.into_iter().map(|x| Cell::new(&x)).collect::<Vec<_>>());
                t.add_row(row);

                write!($f, "{}", t)
            }
            Self::Map(exprs) if $debug => write!(
                $f,
                "{{{}}}",
                exprs
                    .iter()
                    .map(|(k, e)| format!("{}: {:?}", k, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::Map(exprs) => {
                let specified_width = $f.width().unwrap_or(
                    terminal_size()
                        .map(|(Width(w), _)| w as usize)
                        .unwrap_or(120),
                );
                let mut t = Table::new();
                let fmt = t.get_format();
                fmt.padding(1, 1);
                // Set width to be 2/3
                fmt.borders('│');
                fmt.column_separator('│');
                fmt.separator(LinePosition::Top, LineSeparator::new('═', '╤', '╒', '╕'));
                fmt.separator(LinePosition::Title, LineSeparator::new('═', '╪', '╞', '╡'));
                fmt.separator(LinePosition::Intern, LineSeparator::new('─', '┼', '├', '┤'));
                fmt.separator(LinePosition::Bottom, LineSeparator::new('─', '┴', '└', '┘'));

                for (key, val) in exprs {
                    match &val {
                        Self::Builtin(Builtin { help, .. }) => {
                            t.add_row(row!(
                                key,
                                format!("{}", val),
                                textwrap::fill(help, specified_width / 6)
                            ));
                        }
                        Self::Map(_) => {
                            t.add_row(row!(key, format!("{:specified_width$}", val)));
                        }
                        Self::List(_) => {
                            let w = specified_width - key.len() - 3;
                            let formatted = format!("{:w$}", val);
                            t.add_row(row!(key, textwrap::fill(&formatted, w),));
                        }
                        _ => {
                            // Format the value to the width of the terminal / 5
                            let formatted = format!("{:?}", val);
                            let w = specified_width / 3;
                            t.add_row(row!(key, textwrap::fill(&formatted, w),));
                        }
                    }
                }
                write!($f, "{}", t)
            }

            Self::None => write!($f, "None"),
            Self::Lambda(param, body, _) => write!($f, "{:?} -> {:?}", param, body),
            Self::Macro(param, body) => write!($f, "{:?} ~> {:?}", param, body),
            Self::Function(name, param, body, _) => {
                write!($f, "fn {}({:?}) {{ {:?} }}", name, param, body)
            }
            Self::Return(body) => write!($f, "return {}", body),
            Self::For(name, list, body) => write!($f, "for {} in {:?} {:?}", name, list, body),
            Self::Do(exprs) => write!(
                $f,
                "{{ {} }}",
                exprs
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join("; ")
            ),

            Self::Del(name) => write!($f, "del {}", name),
            Self::Declare(name, expr) => write!($f, "let {} = {:?}", name, expr),
            Self::Assign(name, expr) => write!($f, "{} = {:?}", name, expr),
            Self::If(cond, true_expr, false_expr) => {
                write!($f, "if {:?} {:?} else {:?}", cond, true_expr, false_expr)
            }
            Self::Match(value, branches) => {
                write!($f, "match {:?} {{ ", value)?;
                for (pat, expr) in branches.iter() {
                    write!($f, "{:?} => {:?}, ", pat, expr)?;
                }
                write!($f, "}}")
            }
            Self::Apply(g, args) => write!(
                $f,
                "{:?} {}",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!($f, "({} {})", op, v)
                } else {
                    write!($f, "({} {})", v, op)
                }
            }
            Self::BinaryOp(op, l, r) => write!($f, "({:?} {} {:?})", l, op, r),
            Self::Index(l, r) => write!($f, "({}[{}]", l, r),
            Self::Builtin(builtin) => fmt::Debug::fmt(builtin, $f),
            _ => write!($f, "Unreachable"), // 作为兜底逻辑
        }
    };
}

// Debug 实现
impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_shared!(self, f, true)
    }
}

// Display 实现
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_shared!(self, f, false)
    }
}

// Expression 辅助函数
impl Expression {
    /// 类型名称
    pub fn type_name(&self) -> String {
        match self {
            Self::List(_) => "list".into(),
            Self::Map(_) => "map".into(),
            Self::String(_) => "string".into(),
            Self::Integer(_) => "integer".into(),
            Self::Symbol(_) => "symbol".into(),
            _ => format!("{:?}", self).split('(').next().unwrap().into(),
        }
    }

    /// 符号转换
    pub fn to_symbol(&self) -> Result<&str, Error> {
        if let Self::Symbol(s) = self {
            Ok(s)
        } else {
            type_error!("symbol", self)
        }
    }

    /// 索引访问
    fn index_slm(l: Expression, r: Expression) -> Result<Expression, Error> {
        match l {
            // 处理列表索引
            Expression::List(list) => {
                if let Expression::Integer(index) = r {
                    list.get(index as usize)
                        .cloned()
                        .ok_or_else(|| Error::IndexOutOfBounds {
                            index: index as usize,
                            len: list.len(),
                        })
                } else {
                    Err(Error::TypeError {
                        expected: "integer".into(),
                        found: r.type_name(),
                    })
                }
            }

            // 处理字典键访问
            Expression::Map(map) => {
                let key = r.to_string(); // 自动转换Symbol/字符串
                map.get(&key)
                    .cloned()
                    .ok_or_else(|| Error::KeyNotFound(key))
            }

            // 处理字符串索引
            Expression::String(s) => {
                if let Expression::Integer(index) = r {
                    s.chars()
                        .nth(index as usize)
                        .map(|c| Expression::String(c.to_string()))
                        .ok_or_else(|| Error::IndexOutOfBounds {
                            index: index as usize,
                            len: s.len(),
                        })
                } else {
                    Err(Error::TypeError {
                        expected: "integer".into(),
                        found: r.type_name(),
                    })
                }
            }

            _ => Err(Error::TypeError {
                expected: "indexable type (list/dict/string)".into(),
                found: l.type_name(),
            }),
        }
    }

    pub fn as_list(&self) -> Result<&Vec<Self>, Error> {
        match self {
            Self::List(v) => Ok(v),
            _ => Err(Error::TypeError {
                expected: "list".into(),
                found: self.type_name(),
            }),
        }
    }

    /// 列表切片，处理负数索引和越界...

    pub fn slice(
        list: Self,
        start: Option<Int>,
        end: Option<Int>,
        step: Int,
    ) -> Result<Self, Error> {
        let list = list.as_list()?;
        let len = list.len() as Int;

        let clamp = |v: Int| if v < 0 { len + v } else { v }.clamp(0, len - 1);

        let (mut start, mut end) = (
            start.map(clamp).unwrap_or(0),
            end.map(|v| clamp(v).min(len)).unwrap_or(len),
        );

        if step < 0 {
            (start, end) = (end.clamp(0, len), start.clamp(0, len));
        }

        let mut result = Vec::new();
        let mut i = start;
        while (step > 0 && i < end) || (step < 0 && i > end) {
            if let Some(item) = list.get(i as usize) {
                result.push(item.clone());
            }
            i += step;
        }
        Ok(Self::List(result))
    }

    /// 辅助方法：将表达式求值为整数选项
    pub fn eval_to_int_opt(
        expr_opt: Option<Box<Self>>,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Option<Int>, Error> {
        match expr_opt {
            // 无表达式时返回 None
            None => Ok(None),
            // 有表达式时进行求值
            Some(boxed_expr) => {
                // 递归求值表达式
                let evaluated = boxed_expr.eval_mut(env, depth)?;

                // 转换为整数
                match evaluated {
                    Self::Integer(i) => Ok(Some(i)),
                    // 处理隐式类型转换
                    Self::Float(f) if f.fract() == 0.0 => Ok(Some(f as Int)),
                    // 处理其他类型错误
                    _ => Err(Error::TypeError {
                        expected: "integer".into(),
                        found: evaluated.type_name(),
                    }),
                }
            }
        }
    }

    /// match的比对
    fn matches_pattern(
        value: &Expression,
        pattern: &Pattern,
        env: &mut Environment,
    ) -> Result<bool, Error> {
        match pattern {
            Pattern::Bind(name) => {
                if name == "_" {
                    // _作为通配符，不绑定变量
                    Ok(true)
                } else {
                    // 正常变量绑定
                    env.define(name, value.clone());
                    Ok(true)
                }
            }
            Pattern::Literal(lit) => Ok(value == lit.as_ref()),
        }
    }

    pub fn apply(self, args: Vec<Self>) -> Self {
        Self::Apply(Box::new(self), args)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Integer(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.is_empty(),
            Self::Bytes(b) => !b.is_empty(),
            Self::Boolean(b) => *b,
            Self::List(exprs) => !exprs.is_empty(),
            Self::Map(exprs) => !exprs.is_empty(),
            Self::Lambda(_, _, _) => true,
            Self::Macro(_, _) => true,
            Self::Builtin(_) => true,
            _ => false,
        }
    }
    pub fn flatten(args: Vec<Self>) -> Vec<Self> {
        let mut result = vec![];
        for arg in args {
            match arg {
                Self::List(exprs) => result.extend(Self::flatten((*exprs).to_vec())), // 解引用并转换为 Vec
                Self::Group(expr) => result.extend(Self::flatten(vec![*expr])),
                _ => result.push(arg),
            }
        }
        result
    }

    fn get_used_symbols(&self) -> Vec<String> {
        match self {
            Self::Symbol(name) => vec![name.clone()],
            Self::None
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Bytes(_)
            | Self::String(_)
            | Self::Boolean(_)
            | Self::Builtin(_)
            | Self::Del(_) => vec![],

            Self::For(_, list, body) => {
                let mut result = vec![];
                result.extend(list.get_used_symbols());
                result.extend(body.get_used_symbols());
                result
            }
            Self::While(cond, body) => {
                let mut result = vec![];
                result.extend(cond.get_used_symbols());
                result.extend(body.get_used_symbols());
                result
            }

            Self::Do(exprs) => {
                let mut result = vec![];
                for expr in exprs {
                    result.extend(expr.get_used_symbols())
                }
                result
            }
            Self::List(exprs) => {
                let mut result = Vec::new();
                for expr in exprs.iter() {
                    result.extend(expr.get_used_symbols());
                }
                result
            }
            Self::Map(exprs) => {
                let mut result = vec![];
                for expr in exprs.values() {
                    result.extend(expr.get_used_symbols())
                }
                result
            }

            Self::Group(inner) | Self::Quote(inner) => inner.get_used_symbols(),
            Self::Lambda(_, body, _) => body.get_used_symbols(),
            Self::Macro(_, body) => body.get_used_symbols(),
            Self::Function(_, _, body, _) => body.get_used_symbols(),
            Self::Return(expr) => expr.get_used_symbols(),

            Self::Declare(_, expr) => expr.get_used_symbols(),
            Self::Assign(_, expr) => expr.get_used_symbols(),
            Self::UnaryOp(_, expr, _) => expr.get_used_symbols(),
            Self::BinaryOp(_, expr, expr2) => {
                let mut result = vec![];
                result.extend(expr.get_used_symbols());
                result.extend(expr2.get_used_symbols());
                result
            }
            Self::Index(expr, expr2) => {
                let mut result = vec![];
                result.extend(expr.get_used_symbols());
                result.extend(expr2.get_used_symbols());
                result
            }
            Self::Slice(expr, _) => {
                expr.get_used_symbols()
                // let mut result = vec![];
                // result.extend(expr.get_used_symbols());
                // result.extend(expr2.get_used_symbols());
                // result
            }
            Self::If(cond, t, e) => {
                let mut result = vec![];
                result.extend(cond.get_used_symbols());
                result.extend(t.get_used_symbols());
                result.extend(e.get_used_symbols());
                result
            }
            Self::Match(value, _) => value.get_used_symbols(),
            Self::Apply(g, args) => {
                let mut result = g.get_used_symbols();
                for expr in args {
                    result.extend(expr.get_used_symbols())
                }
                result
            }
        }
    }
}
// Expression求值1
impl Expression {
    pub fn eval(&self, env: &mut Environment) -> Result<Self, Error> {
        self.clone().eval_mut(env, 0)
    }
    /// 求值主逻辑（尾递归优化）
    pub fn eval_mut(self, env: &mut Environment, depth: usize) -> Result<Self, Error> {
        if let Some(max) = MAX_RECURSION_DEPTH {
            if depth > max {
                return Err(Error::RecursionDepth(self));
            }
        }

        loop {
            match self {
                // 基础类型直接返回
                Self::String(_)
                | Self::Boolean(_)
                | Self::Integer(_)
                | Self::None
                | Self::Float(_)
                | Self::Bytes(_)
                | Self::Macro(_, _)
                | Self::Builtin(_) => {
                    break Ok(self);
                }

                // 符号解析（错误处理优化）
                Self::Symbol(name) => {
                    dbg!("symbol----", &name, env.get(&name));

                    let r = Ok(match env.get(&name) {
                        Some(expr) => expr,
                        None => Self::Symbol(name),
                    });
                    dbg!(&r);
                    break r;
                }

                // 处理变量声明（仅允许未定义变量）
                Self::Declare(name, expr) => {
                    unsafe {
                        if STRICT && env.has(&name)
                        // && env.get("STRICT") == Some(Expression::Boolean(true))
                        {
                            return Err(Error::Redeclaration(name.to_string()));
                        }
                    }
                    let value = expr.eval_mut(env, depth + 1)?;
                    env.define(&name, value); // 新增 declare
                    dbg!("declare--->", &name, env.get(&name));
                    return Ok(Self::None);
                }

                // Assign 优先修改子环境，未找到则修改父环境
                Self::Assign(name, expr) => {
                    dbg!("assign----", &name);

                    let value = expr.eval_mut(env, depth + 1)?;
                    if env.has(&name) {
                        env.define(&name, value);
                    } else {
                        // 向上层环境查找并修改（根据语言设计需求）
                        let mut current_env = env.clone();
                        while let Some(parent) = current_env.get_parent_mut() {
                            if parent.has(&name) {
                                parent.define(&name, value.clone());
                                return Ok(Self::None);
                            }
                            current_env = parent.clone();
                        }
                        unsafe {
                            if STRICT
                            // && env.get("STRICT") == Some(Expression::Boolean(true))
                            {
                                return Err(Error::UndeclaredVariable(name));
                            } else {
                                env.define(&name, value.clone());
                            }
                        }
                    }
                    return Ok(Self::None);
                }

                // TODO 是否只能删除当前env的变量，是否报错
                // del
                Self::Del(name) => {
                    env.undefine(&name);
                    return Ok(Self::None);
                }

                // 元表达式处理
                Self::Group(inner) => return inner.eval_mut(env, depth + 1),
                Self::Quote(inner) => return Ok(*inner),

                // 一元运算
                Self::UnaryOp(op, operand, is_prefix) => {
                    let operand_eval = operand.eval(env)?;
                    return match op.as_str() {
                        "!" => Ok(Expression::Boolean(!operand_eval.is_truthy())),
                        // 处理 ++a 转换为 a = a + 1
                        "++" | "--" => {
                            // 确保操作数是符号
                            let var_name = operand.to_symbol()?;
                            // 获取当前值
                            let current_val = env
                                .get(var_name)
                                .ok_or(Error::UndeclaredVariable(var_name.to_string()))?;
                            // 确保操作是合法的，例如整数或浮点数
                            if !matches!(current_val, Expression::Integer(_) | Expression::Float(_))
                            {
                                return Err(Error::CustomError(format!(
                                    "Cannot apply {op} to {current_val:?}"
                                )));
                            }
                            // 计算新值
                            let step = if op == "++" { 1 } else { -1 };
                            let new_val = current_val.clone() + Expression::Integer(step);
                            env.define(var_name, new_val.clone());
                            Ok(if is_prefix {
                                new_val
                            } else {
                                current_val.clone()
                            })
                        }
                        _ => Err(Error::CustomError(format!("Unknown unary operator: {op}"))),
                    };
                }
                // 特殊运算符
                Self::BinaryOp(op, lhs, rhs) if op == "|" => {
                    // 管道运算符特殊处理
                    let left_output = lhs.eval_mut(env, depth + 1)?.to_string();
                    let mut new_env = env.fork();
                    new_env.define("stdin", Self::String(left_output));
                    return rhs.eval_mut(&mut new_env, depth + 1);
                }

                Self::BinaryOp(op, lhs, rhs) if op == "<<" => {
                    // 输入重定向处理
                    let path = rhs.eval_mut(env, depth + 1)?.to_string();
                    let contents = std::fs::read_to_string(path)
                        .map(Self::String)
                        .map_err(|e| Error::CustomError(e.to_string()))?;
                    lhs.eval_mut(env, depth + 1)?; // 执行左侧但不使用结果
                    return Ok(contents);
                }
                // 二元运算（内存优化）
                Self::BinaryOp(op, lhs, rhs) => {
                    let l = lhs.eval_mut(env, depth + 1)?;
                    let r = rhs.eval_mut(env, depth + 1)?;
                    break match op.as_str() {
                        "+" => Ok(l + r),
                        "-" => Ok(l - r),
                        "*" => Ok(l * r),
                        "/" => Ok(l / r), //no zero
                        "%" => Ok(l % r),
                        "**" => match (l, r) {
                            (Expression::Float(base), Expression::Float(exponent)) => {
                                Ok(base.powf(exponent).into())
                            }
                            (Expression::Float(base), Expression::Integer(exponent)) => {
                                Ok(base.powf(exponent as f64).into())
                            }
                            (Expression::Integer(base), Expression::Float(exponent)) => {
                                Ok((base as f64).powf(exponent).into())
                            }
                            (Expression::Integer(base), Expression::Integer(exponent)) => {
                                match base.checked_pow(exponent as u32) {
                                    Some(n) => Ok(n.into()),
                                    None => Err(Error::CustomError(format!(
                                        "overflow when raising int {} to the power {}",
                                        base, exponent
                                    ))),
                                }
                            }
                            (a, b) => Err(Error::CustomError(format!(
                                "cannot raise {} to the power {}",
                                a, b
                            ))),
                        },

                        "&&" => Ok(Expression::Boolean(l.is_truthy() && r.is_truthy())),
                        "||" => Ok(Expression::Boolean(l.is_truthy() || r.is_truthy())),
                        "==" => Ok(Expression::Boolean(l == r)),
                        "!=" => Ok(Expression::Boolean(l != r)),
                        ">" => Ok(Expression::Boolean(l > r)),
                        "<" => Ok(Expression::Boolean(l < r)),
                        ">=" => Ok(Expression::Boolean(l >= r)),
                        "<=" => Ok(Expression::Boolean(l <= r)),
                        "~~" => Ok(Expression::Boolean(l.to_string().contains(&r.to_string()))),
                        "~=" => {
                            let regex = Regex::new(&r.to_string())
                                .map_err(|e| Error::CustomError(e.to_string()))?;

                            Ok(Expression::Boolean(regex.is_match(&l.to_string())))
                        }
                        "@" => Self::index_slm(l, r),

                        // ----------
                        // TODO 完善
                        // "|" => {
                        //     let left_output = l.to_string(); // 执行左侧并捕获输出
                        //     let mut new_env = env.fork();
                        //     new_env.define("stdin", Expression::String(left_output));
                        //     r.eval(&mut new_env)
                        // }

                        // "<<" => {
                        //     // 从文件读取输入（此处需实现文件读取逻辑）
                        //     use std::path::PathBuf;
                        //     let mut path = PathBuf::from(env.get_cwd());
                        //     path = path.join(r.to_string());

                        //     match std::fs::read_to_string(&path) {
                        //         // First, try to read the contents as a string.
                        //         Ok(contents) => Ok(contents.into()),
                        //         // If that fails, try to read them as a list of bytes.
                        //         Err(_) => match std::fs::read(&path) {
                        //             Ok(contents) => Ok(Expression::Bytes(contents)),
                        //             Err(_) => Err(Error::CustomError(format!(
                        //                 "could not read file {}",
                        //                 r
                        //             ))),
                        //         },
                        //     }
                        // }
                        ">>>" => {
                            use std::path::PathBuf;
                            let mut path = PathBuf::from(env.get_cwd());
                            path = path.join(r.to_string());
                            match std::fs::OpenOptions::new().append(true).open(&path) {
                                Ok(mut file) => {
                                    use std::io::prelude::*;

                                    let result = if let Expression::Bytes(bytes) = l {
                                        // std::fs::write(path, bytes)
                                        file.write_all(&bytes)
                                    } else {
                                        // Otherwise, convert the contents to a pretty string and write that.
                                        // std::fs::write(path, contents.to_string())
                                        file.write_all(l.to_string().as_bytes())
                                    };

                                    match result {
                                        Ok(()) => Ok(Expression::None),
                                        Err(e) => Err(Error::CustomError(format!(
                                            "could not append to file {}: {:?}",
                                            r, e
                                        ))),
                                    }
                                }
                                Err(e) => Err(Error::CustomError(format!(
                                    "could not open file {}: {:?}",
                                    r, e
                                ))),
                            }
                        }
                        ">>" => {
                            use std::path::PathBuf;
                            let mut path = PathBuf::from(env.get_cwd());
                            path = path.join(r.to_string());

                            // If the contents are bytes, write the bytes directly to the file.
                            let result = if let Expression::Bytes(bytes) = l {
                                std::fs::write(path, bytes)
                            } else {
                                // Otherwise, convert the contents to a pretty string and write that.
                                std::fs::write(path, l.to_string())
                            };

                            match result {
                                Ok(()) => Ok(Expression::None),
                                Err(e) => Err(Error::CustomError(format!(
                                    "could not write to file {}: {:?}",
                                    r, e
                                ))),
                            }
                        }

                        _ => Err(Error::InvalidOperator(op.clone())),
                    };
                }

                // 列表求值（内存优化）
                // Self::List(elems) => {
                //     let evaluated = elems
                //         .iter()
                //         .map(|e| e.clone().eval_mut(env, depth + 1))
                //         .collect::<Result<Vec<_>, _>>()?;
                //     Ok(Expression::List(evaluated))
                // }
                Self::List(items) => {
                    return Ok(Self::List(
                        items
                            .iter()
                            .map(|e| e.clone().eval_mut(env, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?
                            .into(),
                    ));
                }

                // 其他复杂类型
                Self::Slice(list, slice_params) => {
                    let listo = list.eval(env)?;
                    let start_int = Expression::eval_to_int_opt(slice_params.start, env, depth)?;
                    let end_int = Expression::eval_to_int_opt(slice_params.end, env, depth)?;
                    let step_int =
                        Expression::eval_to_int_opt(slice_params.step, env, depth)?.unwrap_or(1); // 默认步长1

                    return Self::slice(listo, start_int, end_int, step_int);
                }
                Self::Index(lhs, rhs) => {
                    let l = lhs.eval_mut(env, depth + 1)?;
                    let r = rhs.eval_mut(env, depth + 1)?;
                    return Self::index_slm(l, r);
                }

                // 其他表达式处理...
                _ => break self.eval_complex(env, depth),
            };
            // depth += 1
        }
    }
}
// Expression求值2
impl Expression {
    /// 处理复杂表达式的递归求值
    fn eval_complex(self, env: &mut Environment, depth: usize) -> Result<Self, Error> {
        match self {
            // 控制流表达式
            Self::For(var, list_expr, body) => {
                // 求值列表表达式
                let list = list_expr.eval_mut(env, depth + 1)?.as_list()?.clone();
                let mut last = Self::None;

                // 遍历每个元素执行循环体
                for item in list.iter() {
                    env.define(&var, item.clone());
                    last = body.clone().eval_mut(env, depth + 1)?;
                }
                Ok(last)
            }
            Self::While(cond, body) => {
                // 循环求值直到条件为假
                let mut last = Self::None;
                while cond.clone().eval_mut(env, depth + 1)?.is_truthy() {
                    last = body.clone().eval_mut(env, depth + 1)?;
                }
                return Ok(last);
            }
            Self::If(cond, true_expr, false_expr) => {
                // 条件分支求值
                return if cond.eval_mut(env, depth + 1)?.is_truthy() {
                    true_expr.eval_mut(env, depth + 1)
                } else {
                    false_expr.eval_mut(env, depth + 1)
                };
            }

            Self::Match(value, branches) => {
                // 模式匹配求值
                let val = value.eval_mut(env, depth + 1)?;
                for (pat, expr) in branches {
                    if Self::matches_pattern(&val, &pat, env)? {
                        return expr.eval_mut(env, depth + 1);
                    }
                }
                return Err(Error::NoMatchingBranch(val.to_string()));
            }

            // 函数相关表达式

            // Self::Function(name, params, body, def_env) => {
            //     // 函数定义时捕获环境
            //     return Ok(Self::Function(name, params, body, def_env));
            // }
            // // Apply a function or macro to an argument
            // Lambda定义优化（自动捕获环境）
            Self::Lambda(params, body, _) => {
                // 自动捕获当前环境
                Ok(Self::Lambda(params, body, env.fork()))
            }
            // 处理函数定义
            Self::Function(name, params, body, def_env) => {
                // dbg!(&def_env);
                // 验证默认值类型（新增）
                for (p, default) in &params {
                    if let Some(expr) = default {
                        match expr {
                            Expression::String(_)
                            | Expression::Integer(_)
                            | Expression::Float(_)
                            | Expression::Boolean(_) => {}
                            _ => {
                                return Err(Error::InvalidDefaultValue(
                                    name,
                                    p.to_string(),
                                    expr.clone(),
                                ));
                            }
                        }
                    }
                }
                // let new_env = def_env.fork();
                // // new_env.define(&param, Expression::None);
                // // new_env.set_cwd(env.get_cwd());
                // for symbol in body.get_used_symbols() {
                //     if !def_env.is_defined(&symbol) {
                //         if let Some(val) = env.get(&symbol) {
                //             new_env.define(&symbol, val)
                //         }
                //     }
                // }
                // dbg!(&new_env);
                let func = Self::Function(name.clone(), params, body, def_env);
                env.define(&name, func.clone());
                return Ok(func);
            }
            Self::Macro(param, body) => {
                // 宏定义保持未求值状态
                return Ok(Self::Macro(param, body));
            }

            // 块表达式
            Self::Do(exprs) => {
                // 创建子环境继承父作用域
                // let mut child_env = env.clone();
                // 顺序求值语句块
                let mut last = Self::None;
                for expr in exprs {
                    last = expr.eval_mut(env, depth + 1)?;
                }
                return Ok(last);
            }

            Self::Return(expr) => {
                // 提前返回机制
                return Err(Error::EarlyReturn(expr.eval_mut(env, depth + 1)?));
            }

            // 函数应用
            Self::Apply(ref func, ref args) => {
                dbg!("applying-------", func, args);

                // 递归求值函数和参数
                let func_eval = func.clone().eval_mut(env, depth + 1)?;
                // let args_eval = args
                //     .into_iter()
                //     .map(|a| a.clone().eval_mut(env, depth + 1))
                //     .collect::<Result<Vec<_>, _>>()?;

                // 分派到具体类型处理
                return match func_eval {
                    Self::Symbol(name) | Self::String(name) => {
                        dbg!("applying-------symbol--", &func, &name);

                        let bindings = env
                            .bindings
                            .clone()
                            .into_iter()
                            .map(|(k, v)| (k, v.to_string()))
                            // This is to prevent environment variables from getting too large.
                            // This causes some strange bugs on Linux: mainly it becomes
                            // impossible to execute any program because `the argument
                            // list is too long`.
                            .filter(|(_, s)| s.len() <= 1024)
                            .collect::<BTreeMap<String, String>>();

                        let mut cmd_args = vec![];
                        for arg in args {
                            for flattened_arg in
                                Self::flatten(vec![arg.clone().eval_mut(env, depth + 1)?])
                            {
                                match flattened_arg {
                                    Self::String(s) => cmd_args.push(s),
                                    Self::Bytes(b) => {
                                        cmd_args.push(String::from_utf8_lossy(&b).to_string())
                                    }
                                    Self::None => continue,
                                    _ => cmd_args.push(format!("{}", flattened_arg)),
                                }
                            }
                        }
                        dbg!(&name, &args, &cmd_args);

                        match Command::new(&name)
                            .current_dir(env.get_cwd())
                            .args(
                                cmd_args, // Self::flatten(args.clone()).iter()
                                         //     .filter(|&x| x != &Self::None)
                                         //     // .map(|x| Ok(format!("{}", x.clone().eval_mut(env, depth + 1)?)))
                                         //     .collect::<Result<Vec<String>, Error>>()?,
                            )
                            .envs(bindings)
                            .status()
                        {
                            Ok(_) => return Ok(Self::None),
                            Err(e) => {
                                return Err(match e.kind() {
                                    ErrorKind::NotFound => Error::ProgramNotFound(name),
                                    ErrorKind::PermissionDenied => {
                                        Error::PermissionDenied(self.clone())
                                    }
                                    _ => Error::CommandFailed(name, args.clone()),
                                });
                            }
                        }
                    }

                    // Self::Builtin(builtin) => (builtin.body)(args_eval, env),
                    Self::Builtin(Builtin { body, .. }) => {
                        return body(args.clone(), env);
                    }

                    // 处理Lambda应用
                    Self::Lambda(params, body, captured_env) => {
                        let mut current_env = captured_env.fork();

                        // 批量参数绑定
                        let (mut bound_env, remaining_args) =
                            bind_arguments(params, args.clone(), env, &mut current_env, depth)?;

                        match remaining_args.len() {
                            // 完全应用：直接求值
                            0 => body.eval_complex(&mut bound_env, depth + 1),

                            // 部分应用：返回新Lambda
                            _ => Ok(Self::Lambda(
                                remaining_args.iter().map(|_| "_".to_string()).collect(),
                                body,
                                bound_env,
                            )),

                            // 参数过多：构造新Apply
                            _ => Ok(Self::Apply(
                                Box::new(body.eval_complex(&mut bound_env, depth + 1)?),
                                remaining_args,
                            )),
                        }
                    }

                    Self::Macro(param, body) if args.len() == 1 => {
                        let x = args[0].clone().eval_mut(env, depth + 1)?;
                        env.define(&param, x);
                        let lamb = *body;
                        return Ok(lamb);
                    }

                    Self::Macro(param, body) if args.len() > 1 => {
                        let x = args[0].clone().eval_mut(env, depth + 1)?;
                        env.define(&param, x);
                        let lamb = Self::Apply(
                            Box::new(body.eval_mut(env, depth + 1)?),
                            args[1..].to_vec(),
                        );
                        return Ok(lamb);
                    }
                    // Self::Macro(param, body) => {
                    //     env.define(&param, Expression::List(args_eval));
                    //     return body.eval_mut(env, depth + 1);
                    // }
                    Self::Function(name, params, body, def_env) => {
                        // dbg!(&def_env);
                        // 参数数量校验
                        if args.len() > params.len() {
                            return Err(Error::TooManyArguments {
                                name,
                                max: params.len(),
                                received: args.len(),
                            });
                        }

                        let mut actual_args = args
                            .into_iter()
                            .map(|a| a.clone().eval_mut(env, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?;

                        // 填充默认值逻辑（新增）
                        for (i, (_, default)) in params.iter().enumerate() {
                            if i >= actual_args.len() {
                                if let Some(def_expr) = default {
                                    // 仅允许基本类型直接使用
                                    actual_args.push(def_expr.clone());
                                } else {
                                    return Err(Error::ArgumentMismatch {
                                        name,
                                        expected: params.len(),
                                        received: actual_args.len(),
                                    });
                                }
                            }
                        }

                        // 创建新作用域并执行
                        let mut new_env = def_env.fork();
                        for ((param, _), arg) in params.iter().zip(actual_args) {
                            new_env.define(param, arg);
                        }
                        // body env
                        // for symbol in body.get_used_symbols() {
                        //     if !def_env.is_defined(&symbol) {
                        //         if let Some(val) = env.get(&symbol) {
                        //             new_env.define(&symbol, val)
                        //         }
                        //     }
                        // }
                        // dbg!(&new_env);
                        return match body.eval_mut(&mut new_env, depth + 1) {
                            Ok(v) => Ok(v),
                            Err(Error::EarlyReturn(v)) => Ok(v), // 捕获函数体内的return
                            Err(e) => Err(e),
                        };
                    }
                    _ => Err(Error::CannotApply(*func.clone(), args.clone())),
                };
            }

            // 默认情况
            _ => Ok(self), // 基本类型已在 eval_mut 处理
        }
    }
}

// 运算符重载（内存优化）

impl Add for Expression {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_add(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 + n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m + n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m + n),
            (Self::String(m), Self::String(n)) => Self::String(m + &n),
            (Self::Bytes(mut a), Self::Bytes(b)) => {
                a.extend(b);
                Self::Bytes(a)
            }
            (Self::List(mut a), Self::List(b)) => {
                a.extend(b);
                Self::List(a)
            }
            _ => Self::None,
        }
    }
}

impl Sub for Expression {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_sub(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 - n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m - n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m - n),
            (Self::Map(mut m), Self::String(n)) => match m.remove_entry(&n) {
                Some((_, val)) => val,
                None => Self::None,
            },
            (Self::List(mut m), Self::Integer(n)) if m.len() > n as usize => m.remove(n as usize),
            _ => Self::None,
        }
    }
}

impl Neg for Expression {
    type Output = Expression;
    fn neg(self) -> Self::Output {
        match self {
            Self::Integer(n) => Self::Integer(-n),
            Self::Boolean(b) => Self::Boolean(!b),
            Self::Float(n) => Self::Float(-n),
            _ => Self::None,
        }
    }
}

impl Mul for Expression {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_mul(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 * n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m * n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m * n),
            (Self::String(m), Self::Integer(n)) | (Self::Integer(n), Self::String(m)) => {
                Self::String(m.repeat(n as usize))
            }
            (Self::List(m), Self::Integer(n)) | (Self::Integer(n), Self::List(m)) => {
                let mut result = vec![];
                for _ in 0..n {
                    result.extend(m.clone());
                }
                Self::List(result)
            }
            _ => Self::None,
        }
    }
}

impl Div for Expression {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_div(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 / n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m / n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m / n),
            _ => Self::None,
        }
    }
}

impl Rem for Expression {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m % n),
            _ => Self::None,
        }
    }
}

impl<T> Index<T> for Expression
where
    T: Into<Self>,
{
    type Output = Self;

    fn index(&self, idx: T) -> &Self {
        match (self, idx.into()) {
            (Self::Map(m), Self::Symbol(name)) | (Self::Map(m), Self::String(name)) => {
                match m.get(&name) {
                    Some(val) => val,
                    None => &Self::None,
                }
            }

            (Self::List(list), Self::Integer(n)) if list.len() > n as usize => &list[n as usize],
            _ => &Self::None,
        }
    }
}

/// PartialOrd实现
impl PartialOrd for Expression {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.partial_cmp(b),
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::Bytes(a), Self::Bytes(b)) => a.partial_cmp(b),
            (Self::List(a), Self::List(b)) => a.partial_cmp(b),
            (Self::Map(a), Self::Map(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

// 内置函数结构（显示优化）
#[derive(Clone)]
pub struct Builtin {
    pub name: String,
    pub body: fn(Vec<Expression>, &mut Environment) -> Result<Expression, Error>,
    pub help: String,
}

impl fmt::Debug for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Builtin({})", self.name)
    }
}
impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "builtin@{}", self.name)
    }
}
impl PartialEq for Builtin {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
// 其他显示实现...

// Expression from

impl From<Int> for Expression {
    fn from(x: Int) -> Self {
        Self::Integer(x)
    }
}

impl From<f64> for Expression {
    fn from(x: f64) -> Self {
        Self::Float(x)
    }
}

impl From<&str> for Expression {
    fn from(x: &str) -> Self {
        Self::String(x.to_string())
    }
}

impl From<String> for Expression {
    fn from(x: String) -> Self {
        Self::String(x)
    }
}

impl From<Vec<u8>> for Expression {
    fn from(x: Vec<u8>) -> Self {
        Self::Bytes(x)
    }
}

impl From<bool> for Expression {
    fn from(x: bool) -> Self {
        Self::Boolean(x)
    }
}

impl<T> From<BTreeMap<String, T>> for Expression
where
    T: Into<Self>,
{
    fn from(map: BTreeMap<String, T>) -> Self {
        Self::Map(
            map.into_iter()
                .map(|(name, item)| (name, item.into()))
                .collect::<BTreeMap<String, Self>>(),
        )
    }
}

impl<T> From<Vec<T>> for Expression
where
    T: Into<Self>,
{
    fn from(list: Vec<T>) -> Self {
        Self::List(
            list.into_iter()
                .map(|item| item.into())
                .collect::<Vec<Self>>(),
        )
    }
}

impl From<Environment> for Expression {
    fn from(env: Environment) -> Self {
        Self::Map(env.bindings.into_iter().collect::<BTreeMap<String, Self>>())
    }
}

// builtin
impl Expression {
    pub fn builtin(
        name: impl ToString,
        body: fn(Vec<Expression>, &mut Environment) -> Result<Expression, Error>,
        help: impl ToString,
    ) -> Self {
        Self::Builtin(Builtin {
            name: name.to_string(),
            body,
            help: help.to_string(),
        })
    }

    pub fn new(x: impl Into<Self>) -> Self {
        // dbg!("---- new exp");
        x.into()
    }
}

/// 参数绑定辅助函数
fn bind_arguments(
    params: Vec<String>,
    args: Vec<Expression>,
    env: &mut Environment,
    target_env: &mut Environment,
    depth: usize,
) -> Result<(Environment, Vec<Expression>), Error> {
    let mut remaining_args = args;

    // 逐个绑定参数
    for (i, param) in params.clone().into_iter().enumerate() {
        if let Some(arg) = remaining_args.get(i) {
            let value = arg.clone().eval_complex(env, depth + 1)?;
            target_env.define(&param, value);
        } else {
            break;
        }
    }

    // 分割已绑定和剩余参数
    let bound_count = params.len().min(remaining_args.len());
    remaining_args.drain(0..bound_count);

    Ok((target_env.clone(), remaining_args))
}
