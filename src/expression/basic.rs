use super::Expression;
use super::builtin::Builtin;
use super::{Environment, Int};
use crate::RuntimeError;
// use num_traits::pow;
use std::fmt;
use terminal_size::{Width, terminal_size};

use prettytable::{
    Cell, Row, Table,
    format::{LinePosition, LineSeparator},
    row,
};

// 错误处理宏（优化点）
macro_rules! type_error {
    ($expected:expr, $found:expr) => {
        Err(RuntimeError::TypeError {
            expected: $expected.into(),
            found: $found.type_name(),
        })
    };
}
// 宏定义（可放在 impl 块外）
macro_rules! fmt_shared {
    ($self:ident, $f:ident, $debug:expr) => {
        match $self {
            Self::Symbol(name) => write!($f, "{}", name),

            Self::String(s) if $debug => write!($f, "{:?}", s),
            Self::String(s) => write!($f, "{}", s),

            Self::Integer(i) => write!($f, "{}", *i),
            Self::Float(n) => write!($f, "{}", *n),
            Self::Bytes(b) => write!($f, "b{:?}", b),
            Self::Boolean(b) => write!($f, "{}", if *b { "True" } else { "False" }),

            Self::Declare(name, expr) => write!($f, "let {} = {:?}", name, expr),
            Self::Assign(name, expr) => write!($f, "{} = {:?}", name, expr),

            // Quote 修改
            Self::Quote(inner) if $debug => write!($f, "'{:?}", inner),
            Self::Quote(inner) => write!($f, "'{}", inner),

            // Group 修改
            Self::Group(inner) if $debug => write!($f, "Group({:?})", inner),
            Self::Group(inner) => write!($f, "({})", inner),

            // While 修改
            Self::While(cond, body) if $debug => write!($f, "while {:?} {:?}", cond, body),
            Self::While(cond, body) => write!($f, "while {} {}", cond, body),

            // Lambda 修改
            Self::Lambda(params, body, _) if $debug => {
                write!($f, "Lambda{:?} -> {:?}", params, body)
            }
            Self::Lambda(params, body, _) => write!($f, "({}) -> {}", params.join(", "), body),
            Self::Macro(params, body) if $debug => write!($f, "{:?} ~> {:?}", params, body),
            Self::Macro(params, body) => write!($f, "({}) ~> {}", params.join(", "), body),

            // If 修改
            Self::If(cond, true_expr, false_expr) => {
                if $debug {
                    write!($f, "if {:?} {:?} else {:?}", cond, true_expr, false_expr)
                } else {
                    write!($f, "if {} {} else {}", cond, true_expr, false_expr)
                }
            }

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
            Self::Function(name, param, body, _) => {
                write!($f, "fn {}({:?}) {{ {:?} }}", name, param, body)
            }
            Self::Return(body) => write!($f, "return {}", body),
            Self::For(name, list, body) => write!($f, "for {} in {:?} {:?}", name, list, body),
            Self::Do(exprs) => write!(
                $f,
                "BLOCK{{ {} }}",
                exprs
                    .iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join("; ")
            ),

            Self::Del(name) => write!($f, "del {}", name),

            Self::Match(value, branches) => {
                write!($f, "match {:?} {{ ", value)?;
                for (pat, expr) in branches.iter() {
                    write!($f, "{:?} => {:?}, ", pat, expr)?;
                }
                write!($f, "}}")
            }
            Self::Apply(g, args) => write!(
                $f,
                "APPLY({:?} {})!",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Command(g, args) => write!(
                $f,
                "COMMAND({:?} {})!",
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
            Self::BinaryOp(op, l, r) => write!($f, "BinaryOp<({:?} {} {:?})>", l, op, r),
            Self::Pipe(op, l, r) => write!($f, "Pipe<({:?} {} {:?})>", l, op, r),
            Self::Index(l, r) => write!($f, "({}[{}]", l, r),
            Self::Builtin(builtin) => fmt::Debug::fmt(builtin, $f),
            Self::Catch(body, _, deel) => match deel {
                Some(deelx) => write!($f, "Catch<({:?} ? {})>", body, deelx),
                _ => write!($f, "Catch<({:?} ?)>", body),
            },
            // Self::Error { code, msg, expr } => {
            //     write!($f, "Error<(code:{}\nmsg:{}\nexpr:{:?})>", code, msg, expr)
            // } // _ => write!($f, "Unreachable"), // 作为兜底逻辑
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
            Self::List(_) => "List".into(),
            Self::Map(_) => "Map".into(),
            Self::String(_) => "String".into(),
            Self::Integer(_) => "Integer".into(),
            Self::Symbol(_) => "Symbol".into(),

            Self::Float(_) => "Float".into(),
            Self::Boolean(_) => "Boolean".into(),
            Self::Group(_) => "Group".into(),
            Self::BinaryOp(_, _, _) => "BinaryOp".into(),
            Self::Pipe(_, _, _) => "Pipe".into(),
            Self::UnaryOp(..) => "UnaryOp".into(),
            Self::Bytes(_) => "Bytes".into(),
            Self::Index(_, _) => "Index".into(),
            Self::Slice(_, _) => "Slice".into(),
            Self::Del(_) => "Del".into(),
            Self::Declare(_, _) => "Declare".into(),
            Self::Assign(_, _) => "Assign".into(),
            Self::For(_, _, _) => "For".into(),
            Self::While(_, _) => "While".into(),
            Self::Match(_, _) => "Match".into(),
            Self::If(_, _, _) => "If".into(),
            Self::Apply(_, _) => "Apply".into(),
            Self::Command(_, _) => "Command".into(),
            Self::Lambda(_, _, _) => "Lambda".into(),
            Self::Macro(_, _) => "Macro".into(),
            Self::Function(_, _, _, _) => "Function".into(),
            Self::Return(_) => "Return".into(),
            Self::Do(_) => "Do".into(),
            Self::Builtin(_) => "Builtin".into(),
            Self::Quote(_) => "Quote".into(),
            Self::Catch(..) => "Catch".into(),
            // Self::Error { .. } => "Error".into(),
            Self::None => "None".into(),
            // _ => format!("{:?}", self).split('(').next().unwrap().into(),
        }
    }

    /// 符号转换
    pub fn to_symbol(&self) -> Result<&str, RuntimeError> {
        if let Self::Symbol(s) = self {
            Ok(s)
        } else {
            type_error!("symbol", self)
        }
    }

    pub fn apply(self, args: Vec<Self>) -> Self {
        Self::Apply(Box::new(self), args)
    }
    // 新增参数合并方法
    pub fn append_args(self, args: Vec<Expression>) -> Expression {
        match self {
            Expression::Apply(f, existing_args) => {
                Expression::Apply(f, [existing_args, args].concat())
            }
            _ => Expression::Apply(Box::new(self), args), //report error?
        }
    }
    pub fn ensure_apply(self) -> Expression {
        match self {
            Expression::Symbol(f) => Expression::Apply(Box::new(Expression::Symbol(f)), vec![]),
            _ => self, //others, like binop,group,pipe...
        }
    }

    pub fn set_status_code(&self, code: Int, env: &mut Environment) {
        env.define("STATUS", Expression::Integer(code));
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
}
