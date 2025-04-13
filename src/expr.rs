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

/// The maximum number of times that `eval` can recursively call itself
/// on a given expression before throwing an error. Even though
/// we could theoretically keep the tail call recursion optimization,
/// we don't really want to do this because it's better to halt.
const MAX_RECURSION_DEPTH: Option<usize> = Some(800);

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
impl Expression {
    pub fn to_symbol(&self) -> Result<&str, Error> {
        if let Self::Symbol(s) = self {
            Ok(s)
        } else {
            Err(Error::UndeclaredVariable(format!(
                "Invalid left symbol: {:?}",
                self
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Bind(String),             // 变量绑定（含_）
    Literal(Box<Expression>), // 字面量匹配
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
    BinaryOp(String, Box<Self>, Box<Self>), // 新增二元运算
    UnaryOp(String, Box<Self>, bool),       // 一元运算符
    Symbol(String),
    // An integer literal
    Integer(Int),
    // A floating point number literal
    Float(f64),
    // A list of bytes
    Bytes(Vec<u8>),
    // A string literal
    String(String),
    // A boolean literal
    Boolean(bool),
    // A list of expressions
    List(Vec<Self>),
    // A map of expressions
    Map(BTreeMap<String, Self>),
    Index(Box<Self>, Box<Self>),   // 索引表达式 table[key]
    Slice(Box<Self>, SliceParams), // 切片表达式 list[start:end:step]

    None,

    Del(String), // 新增删除操作
    Declare(String, Box<Self>),
    // Assign an expression to a variable
    Assign(String, Box<Self>),

    // Control flow
    For(String, Box<Self>, Box<Self>),
    While(Box<Self>, Box<Self>),                 // (条件, 循环体)
    Match(Box<Self>, Vec<(Pattern, Box<Self>)>), // (匹配对象, 模式分支列表)

    // Control flow
    If(Box<Self>, Box<Self>, Box<Self>),

    // Apply a function or macro to an argument
    Apply(Box<Self>, Vec<Self>),

    Lambda(String, Box<Self>, Environment),
    Macro(String, Box<Self>),
    Function(
        String,                            // 函数名
        Vec<(String, Option<Expression>)>, // 参数列表（带默认值）
        Box<Self>,                         // 函数体
        Environment,                       // 定义时的环境（用于默认值）
    ),
    Return(Box<Self>), // 新增返回语句

    Do(Vec<Self>),
    // A builtin function.
    Builtin(Builtin),

    Quote(Box<Self>),
}

#[derive(Clone)]
pub struct Builtin {
    /// name of the function
    pub name: String,
    /// function pointer for executing the function
    pub body: fn(Vec<Expression>, &mut Environment) -> Result<Expression, Error>,
    /// help string
    pub help: String,
}

impl fmt::Debug for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "builtin@{}", self.name)
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

impl Expression {
    /// 获取类型名称用于错误提示
    pub fn type_name(&self) -> String {
        match self {
            Self::List(_) => "list".into(),
            Self::Map(_) => "map".into(),
            Self::String(_) => "string".into(),
            Self::Integer(_) => "integer".into(),
            Self::Symbol(_) => "symbol".into(),
            _ => "expression".into(), // ...其他类型...
        }
    }

    /// 统一转换为字符串用于字典键
    pub fn to_string(&self) -> String {
        match self {
            Self::Symbol(s) => s.clone(),
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string(),
            _ => format!("{}", self), // 其他类型按显示形式转换
        }
    }
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Quote(inner) => write!(f, "'{:?}", inner),
            Self::Group(inner) => write!(f, "({:?})", inner),
            Self::Symbol(name) => write!(f, "{}", name),
            Self::Integer(i) => write!(f, "{}", *i),
            Self::Float(n) => write!(f, "{}", *n),
            Self::Bytes(b) => write!(f, "b{:?}", b),
            Self::String(s) => write!(f, "{:?}", s),
            Self::Boolean(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Self::List(exprs) => write!(
                f,
                "[{}]",
                exprs
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::While(cond, body) => write!(f, "while {:?} {:?}", cond, body),

            Self::Map(exprs) => write!(
                f,
                "{{{}}}",
                exprs
                    .iter()
                    .map(|(k, e)| format!("{}: {:?}", k, e))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),

            Self::None => write!(f, "None"),
            Self::Lambda(param, body, _) => write!(f, "{} -> {:?}", param, body),
            Self::Macro(param, body) => write!(f, "{} ~> {:?}", param, body),
            Self::Function(name, param, body, _) => {
                write!(f, "fn {}({:?}) {{ {:?} }}", name, param, body)
            }
            Self::Return(body) => write!(f, "return {}", body),
            Self::For(name, list, body) => write!(f, "for {} in {:?} {:?}", name, list, body),
            Self::Do(exprs) => write!(
                f,
                "{{ {} }}",
                exprs
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join("; ")
            ),

            Self::Del(name) => write!(f, "del {}", name),
            Self::Declare(name, expr) => write!(f, "let {} = {:?}", name, expr),
            Self::Assign(name, expr) => write!(f, "{} = {:?}", name, expr),
            Self::If(cond, true_expr, false_expr) => {
                write!(f, "if {:?} {:?} else {:?}", cond, true_expr, false_expr)
            }
            Self::Match(value, branches) => {
                write!(f, "match {:?} {{ ", value)?;
                for (pat, expr) in branches.iter() {
                    write!(f, "{:?} => {:?}, ", pat, expr)?;
                }
                write!(f, "}}")
            }
            Self::Apply(g, args) => write!(
                f,
                "{:?} {}",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!(f, "({} {})", op, v)
                } else {
                    write!(f, "({} {})", v, op)
                }
            }
            Self::BinaryOp(op, l, r) => write!(f, "({:?} {} {:?})", l, op, r),
            Self::Index(l, r) => write!(f, "({}[{}]", l, r),
            Self::Slice(l, r) => write!(f, "({}[{:?}:{:?}:{:?}])", l, r.start, r.end, r.step),

            Self::Builtin(builtin) => fmt::Debug::fmt(builtin, f),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let specified_width = f.width().unwrap_or(
            terminal_size()
                .map(|(Width(w), _)| w as usize)
                .unwrap_or(120),
        );
        // let width = match terminal_size() {
        //     Some((Width(width), _)) => Some(width as usize),
        //     _ => None,
        // }

        match self {
            Self::Quote(inner) => write!(f, "'{:?}", inner),
            Self::Group(inner) => write!(f, "({:?})", inner),
            Self::Symbol(name) => write!(f, "{}", name),
            Self::Integer(i) => write!(f, "{}", *i),
            Self::Float(n) => write!(f, "{}", *n),
            Self::Bytes(b) => write!(f, "b{:?}", b),
            Self::String(s) => write!(f, "{}", s),
            Self::Boolean(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Self::List(exprs) => {
                // Create a table with one column
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
                for expr in exprs {
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
                    return write!(f, "{:?}", self);
                }
                let row = Row::new(row.into_iter().map(|x| Cell::new(&x)).collect::<Vec<_>>());
                t.add_row(row);

                write!(f, "{}", t)
            }
            Self::Map(exprs) => {
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
                write!(f, "{}", t)
            }

            Self::None => write!(f, "None"),
            Self::Lambda(param, body, _) => write!(f, "{} -> {:?}", param, body),
            Self::Macro(param, body) => write!(f, "{} ~> {:?}", param, body),
            Self::Function(name, param, body, _) => {
                write!(f, "fn {}({:?}) {{ {:?} }}", name, param, body)
            }
            Self::Return(body) => write!(f, "return {}", body),

            Self::For(name, list, body) => write!(f, "for {} in {:?} {:?}", name, list, body),
            Self::While(cond, body) => write!(f, "while {:?} {:?}", cond, body),

            Self::Do(exprs) => write!(
                f,
                "{{ {} }}",
                exprs
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join("; ")
            ),

            Self::Del(name) => write!(f, "del {}", name),
            Self::Declare(name, expr) => write!(f, "let {} = {:?}", name, expr),
            Self::Assign(name, expr) => write!(f, "{} = {:?}", name, expr),
            Self::If(cond, true_expr, false_expr) => {
                write!(f, "if {:?} {:?} else {:?}", cond, true_expr, false_expr)
            }
            Expression::Match(value, branches) => {
                write!(f, "match {:?} {{ ", value)?;
                for (pat, expr) in branches.iter() {
                    write!(f, "{:?} => {:?}, ", pat, expr)?;
                }
                write!(f, "}}")
            }
            Self::Apply(g, args) => write!(
                f,
                "{:?} {}",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!(f, "({} {})", op, v)
                } else {
                    write!(f, "({} {})", v, op)
                }
            }
            Self::BinaryOp(op, l, r) => write!(f, "({:?} {} {:?})", l, op, r),
            Self::Index(l, r) => write!(f, "({}[{}]", l, r),
            Self::Slice(l, r) => write!(f, "({}[{:?}:{:?}:{:?}])", l, r.start, r.end, r.step),

            Self::Builtin(builtin) => fmt::Display::fmt(builtin, f),
        }
    }
}

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
// 在 expr.rs.txt 的 Expression 实现中添加以下方法
impl Expression {
    /// 将表达式选项转换为整数选项
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

    // 处理负数索引和越界...
    pub fn as_list(&self) -> Result<&Vec<Self>, Error> {
        match self {
            Self::List(v) => Ok(v),
            _ => Err(Error::TypeError {
                expected: "list".into(),
                found: self.type_name(),
            }),
        }
    }

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
}

impl Expression {
    pub fn builtin(
        name: impl ToString,
        body: fn(Vec<Self>, &mut Environment) -> Result<Self, Error>,
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
                Self::List(exprs) => result.extend(Self::flatten(exprs)),
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

            Self::Do(exprs) | Self::List(exprs) => {
                let mut result = vec![];
                for expr in exprs {
                    result.extend(expr.get_used_symbols())
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

    pub fn eval(&self, env: &mut Environment) -> Result<Self, Error> {
        self.clone().eval_mut(env, 0)
    }

    fn eval_mut(mut self, env: &mut Environment, mut depth: usize) -> Result<Self, Error> {
        loop {
            if let Some(max_depth) = MAX_RECURSION_DEPTH {
                if depth > max_depth {
                    return Err(Error::RecursionDepth(self));
                }
            }

            match self {
                Self::Quote(inner) => return Ok(*inner),
                Self::Group(inner) => return inner.eval_mut(env, depth + 1),

                Self::Symbol(name) => {
                    return Ok(match env.get(&name) {
                        Some(expr) => expr,
                        None => Self::Symbol(name.clone()),
                    });
                }
                Self::Del(name) => {
                    env.undefine(&name);
                    return Ok(Self::None);
                }
                // 处理变量声明（仅允许未定义变量）
                Self::Declare(name, expr) => {
                    unsafe {
                        if env.is_defined(&name) && STRICT
                        // && env.get("STRICT") == Some(Expression::Boolean(true))
                        {
                            return Err(Error::Redeclaration(name));
                        }
                    }
                    let value = expr.eval_mut(env, depth + 1)?;
                    env.define(&name, value); // 新增 declare 方法
                    return Ok(Self::None);
                }
                Self::Assign(name, expr) => {
                    // TODO: enable check while in strict mode.
                    // if !env.is_defined(&name) {
                    //     return Err(Error::UndeclaredVariable(name));
                    // }
                    let x = expr.eval_mut(env, depth + 1)?;
                    env.define(&name, x);
                    return Ok(Self::None);
                }
                // 处理 ++a 转换为 a = a + 1
                Self::UnaryOp(op, operand, is_prefix) => {
                    let operand_eval = operand.eval(env)?;
                    return match op.as_str() {
                        "!" => Ok(Expression::Boolean(!operand_eval.is_truthy())),
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
                // 处理 a++ 转换为 (tmp = a, a = a + 1, tmp)
                // Expression::PostfixOp { op: "++", operand } => {
                //     let old_val = operand.eval(env)?;
                //     let new_val = old_val.clone() + Expression::Integer(1);
                //     env.define(operand.to_symbol(), new_val);
                //     Ok(old_val)
                // }
                Self::Slice(list, slice_params) => {
                    let listo = list.eval(env)?;
                    let start_int = Expression::eval_to_int_opt(slice_params.start, env, depth)?;
                    let end_int = Expression::eval_to_int_opt(slice_params.end, env, depth)?;
                    let step_int =
                        Expression::eval_to_int_opt(slice_params.step, env, depth)?.unwrap_or(1); // 默认步长1

                    return Self::slice(listo, start_int, end_int, step_int);
                }
                Self::Index(lhs, rhs) => {
                    let l = lhs.eval(env)?;
                    let r = rhs.eval(env)?;
                    return Self::index_slm(l, r);
                }
                // 处理二元运算
                Self::BinaryOp(op, lhs, rhs) => {
                    let l = lhs.eval(env)?;
                    let r = rhs.eval(env)?;
                    return match op.as_str() {
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

                        "&&" => Ok(Expression::Boolean(
                            l.is_truthy() && rhs.eval(env)?.is_truthy(),
                        )),
                        "||" => Ok(Expression::Boolean(
                            l.is_truthy() || rhs.eval(env)?.is_truthy(),
                        )),
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
                        "|" => {
                            let left_output = l.to_string(); // 执行左侧并捕获输出
                            let mut new_env = env.fork();
                            new_env.define("stdin", Expression::String(left_output));
                            r.eval(&mut new_env)
                        }

                        "<<" => {
                            // 从文件读取输入（此处需实现文件读取逻辑）
                            use std::path::PathBuf;
                            let mut path = PathBuf::from(env.get_cwd());
                            path = path.join(r.to_string());

                            match std::fs::read_to_string(&path) {
                                // First, try to read the contents as a string.
                                Ok(contents) => Ok(contents.into()),
                                // If that fails, try to read them as a list of bytes.
                                Err(_) => match std::fs::read(&path) {
                                    Ok(contents) => Ok(Expression::Bytes(contents)),
                                    Err(_) => Err(Error::CustomError(format!(
                                        "could not read file {}",
                                        r
                                    ))),
                                },
                            }
                        }
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

                Self::For(name, list, body) => {
                    let mut new_env = env.fork();
                    if let Expression::List(items) =
                        list.clone().eval_mut(&mut new_env, depth + 1)?
                    {
                        let mut results = vec![];
                        for item in items {
                            new_env.define(&name, item);
                            results.push(body.clone().eval_mut(&mut new_env, depth + 1)?);
                        }
                        return Ok(Self::List(results));
                        // return Ok(Self::List(
                        //     items
                        //         .into_iter()
                        //         .map(|item| {
                        //             env.define(&name, item);
                        //             body.clone().eval_mut(env, depth + 1)
                        //         })
                        //         .collect::<Result<Vec<Self>, Error>>()?,
                        // ));
                    } else {
                        return Err(Error::ForNonList(*list));
                    }
                }
                Self::While(cond, body) => {
                    let mut results = vec![];
                    let mut new_env = env.fork();
                    while cond.clone().eval_mut(&mut new_env, depth + 1)?.is_truthy() {
                        results.push(body.clone().eval_mut(&mut new_env, depth + 1)?);
                    }
                    // return Ok(Self::List(results));
                    return Ok(Expression::None);
                }
                Self::If(cond, true_expr, false_expr) => {
                    let mut new_env = env.fork();
                    return if cond.eval_mut(&mut new_env, depth + 1)?.is_truthy() {
                        true_expr
                    } else {
                        false_expr
                    }
                    .eval_mut(&mut new_env, depth + 1);
                }
                Self::Match(ref value, ref branches) => {
                    let mut new_env = env.fork();
                    let evaluated_value = value.clone().eval_mut(&mut new_env, depth + 1)?;
                    for (pattern, expr) in branches {
                        if Self::matches_pattern(&evaluated_value, pattern, env)? {
                            return expr.clone().eval_mut(&mut new_env, depth + 1);
                        }
                    }
                    return Err(Error::NoMatchingBranch(value.to_string()));
                }

                Self::Apply(ref f, ref args) => match f.clone().eval_mut(env, depth + 1)? {
                    Self::Symbol(name) | Self::String(name) => {
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

                    Self::Lambda(param, body, old_env) if args.len() == 1 => {
                        let mut new_env = old_env;
                        new_env.set_cwd(env.get_cwd());
                        new_env.define(&param, args[0].clone().eval_mut(env, depth + 1)?);
                        return body.eval_mut(&mut new_env, depth + 1);
                    }

                    Self::Lambda(param, body, old_env) if args.len() > 1 => {
                        let mut new_env = old_env.clone();
                        new_env.set_cwd(env.get_cwd());
                        new_env.define(&param, args[0].clone().eval_mut(env, depth + 1)?);
                        self = Self::Apply(
                            Box::new(body.eval_mut(&mut new_env, depth + 1)?),
                            args[1..].to_vec(),
                        );
                    }

                    Self::Macro(param, body) if args.len() == 1 => {
                        let x = args[0].clone().eval_mut(env, depth + 1)?;
                        env.define(&param, x);
                        self = *body;
                    }

                    Self::Macro(param, body) if args.len() > 1 => {
                        let x = args[0].clone().eval_mut(env, depth + 1)?;
                        env.define(&param, x);
                        self = Self::Apply(
                            Box::new(body.eval_mut(env, depth + 1)?),
                            args[1..].to_vec(),
                        );
                    }

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
                        for symbol in body.get_used_symbols() {
                            if !def_env.is_defined(&symbol) {
                                if let Some(val) = env.get(&symbol) {
                                    new_env.define(&symbol, val)
                                }
                            }
                        }
                        // dbg!(&new_env);
                        return match body.eval_mut(&mut new_env, depth + 1) {
                            Ok(v) => Ok(v),
                            Err(Error::ReturnValue(v)) => Ok(*v), // 捕获函数体内的return
                            Err(e) => Err(e),
                        };
                    }
                    // 处理return语句
                    // Self::Return(expr) => {
                    //     let value = expr.eval_mut(env, depth + 1)?;
                    //     return Err(Error::ReturnValue(Box::new(value)));
                    // }
                    Self::Builtin(Builtin { body, .. }) => {
                        return body(args.clone(), env);
                    }

                    _ => return Err(Error::CannotApply(*f.clone(), args.clone())),
                },

                // // Apply a function or macro to an argument
                Self::Lambda(param, body, captured) => {
                    let mut tmp_env = captured.clone();
                    tmp_env.define(&param, Expression::None);
                    tmp_env.set_cwd(env.get_cwd());
                    for symbol in body.get_used_symbols() {
                        if symbol != param && !captured.is_defined(&symbol) {
                            if let Some(val) = env.get(&symbol) {
                                tmp_env.define(&symbol, val)
                            }
                        }
                    }
                    return Ok(Self::Lambda(param.clone(), body, tmp_env));
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
                // 处理return语句
                Self::Return(expr) => {
                    // dbg!(&env.bindings);
                    // dbg!(env.get("r"));
                    let value = expr.eval_mut(env, depth + 1)?;
                    // dbg!(&value);
                    return Err(Error::ReturnValue(Box::new(value)));
                }

                Self::List(exprs) => {
                    return Ok(Self::List(
                        exprs
                            .into_iter()
                            .map(|x| x.eval_mut(env, depth + 1))
                            .collect::<Result<Vec<Self>, Error>>()?,
                    ));
                }
                Self::Map(exprs) => {
                    return Ok(Self::Map(
                        exprs
                            .into_iter()
                            .map(|(n, x)| Ok((n, x.eval_mut(env, depth + 1)?)))
                            .collect::<Result<BTreeMap<String, Self>, Error>>()?,
                    ));
                }
                // Self::Do(exprs) => {
                //     if exprs.is_empty() {
                //         return Ok(Self::None);
                //     }
                //     for expr in &exprs[..exprs.len() - 1] {
                //         expr.clone().eval_mut(env, depth + 1)?;
                //     }
                //     self = exprs[exprs.len() - 1].clone();
                // }
                // 修改Do块处理逻辑
                // expr.rs 修改后的Do处理逻辑
                Expression::Do(exprs) => {
                    let mut last = Expression::None;
                    for expr in exprs {
                        match expr.eval_mut(env, depth + 1) {
                            Ok(v) => last = v,
                            // 直接捕获ReturnValue并返回
                            Err(Error::ReturnValue(v)) => return Ok(*v),
                            Err(e) => return Err(e),
                        }
                    }
                    return Ok(last);
                }
                Self::None
                | Self::Integer(_)
                | Self::Float(_)
                | Self::Boolean(_)
                | Self::Bytes(_)
                | Self::String(_)
                | Self::Macro(_, _)
                | Self::Builtin(_) => return Ok(self.clone()),
            }
            depth += 1;
        }
    }
}

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
