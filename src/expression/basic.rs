use super::builtin::Builtin;
use super::{CatchType, Environment, Expression, Int};
use crate::RuntimeError;
// use num_traits::pow;
use std::fmt;
use std::rc::Rc;
use terminal_size::{Width, terminal_size};

use prettytable::{
    Cell,
    Row,
    Table,
    format::consts::{FORMAT_BORDERS_ONLY, FORMAT_BOX_CHARS},
    // format::{LinePosition, LineSeparator},
    row,
};
// const GREEN_BOLD: &str = "\x1b[1;32m";
// // const RED: &str = "\x1b[31m";
// const RESET: &str = "\x1b[0m";
// fn green(s: &str) -> String {
//     format!("{}{}{}", GREEN_BOLD, s, RESET)
// }
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
            Self::Variable(name) => write!($f, "${}", name),

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
            Self::Loop(inner) => write!($f, "(loop {})", inner),

            // Lambda 修改
            Self::Lambda(params, body) if $debug => {
                write!($f, "Lambda{:?} -> {:?}", params, body)
            }
            Self::Lambda(params, body) => write!($f, "({}) -> {}", params.join(", "), body),
            // Self::Macro(params, body) if $debug => write!($f, "{:?} ~> {:?}", params, body),
            // Self::Macro(params, body) => write!($f, "({}) ~> {}", params.join(", "), body),

            // If 修改
            Self::If(cond, true_expr, false_expr) => {
                if $debug {
                    write!($f, "if {:?} {:?} else {:?}", cond, true_expr, false_expr)
                } else {
                    write!($f, "if {} {} else {}", cond, true_expr, false_expr)
                }
            }

            Self::Slice(l, r) => write!($f, "{}[{:?}:{:?}:{:?}]", l, r.start, r.end, r.step),

            // 修正List分支中的变量名错误
            Self::List(exprs) if $debug => {
                write!(
                    $f,
                    "[{}]",
                    exprs
                        .as_ref()
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
                // let mut t = Table::new();
                // let fmt = t.get_format();
                // fmt.padding(1, 1);
                // fmt.borders('│');
                // fmt.column_separator('│');
                // fmt.separator(LinePosition::Top, LineSeparator::new('─', '┬', '┌', '┐'));
                // fmt.separator(LinePosition::Title, LineSeparator::new('─', '┼', '├', '┤'));
                // fmt.separator(LinePosition::Intern, LineSeparator::new('─', '┼', '├', '┤'));
                // fmt.separator(LinePosition::Bottom, LineSeparator::new('─', '┴', '└', '┘'));

                // let mut row = vec![];
                // let mut total_len = 1;
                // for expr in exprs.as_ref().iter() {
                //     let formatted = match expr {
                //         Expression::String(s) => format!("{}", s),
                //         _ => format!("{}", expr),
                //     };
                //     // Get the length of the first line
                //     if formatted.contains('\n') {
                //         let max_line_len = formatted.lines().map(|l| l.len()).max().unwrap_or(0);
                //         total_len += max_line_len + 1;
                //         // let first_line_len = formatted.lines().next().unwrap().len();
                //         // total_len += first_line_len + 1;
                //     } else {
                //         total_len += formatted.len() + 1;
                //     }
                //     row.push(formatted);
                // }
                // // if total_len > specified_width {
                // //     return write!($f, "{:?}", $self);
                // // }
                // // 换行或截断处理
                // if total_len > specified_width {
                //     t.set_format(*prettytable::format::consts::FORMAT_BORDERS_ONLY);
                //     // 或自动换行：
                //     // fmt.set_column_max_width(specified_width / exprs.len());
                // }
                // // let row = Row::new(row.into_iter().map(|x| Cell::new(&x)).collect::<Vec<_>>());
                // // t.add_row(row);

                // let cells = exprs
                //     .iter()
                //     .map(|expr| match expr {
                //         Expression::String(s) => Cell::new(s),
                //         _ => Cell::new(&expr.to_string()),
                //     })
                //     .collect::<Vec<_>>();
                // t.add_row(Row::new(cells));

                let (rows,heads_opt) = TableRow {
                    columns: exprs.as_ref(),
                    max_width: specified_width-10,
                    col_padding: 3+2,
                }.split_into_rows();

                let mut t = Table::new();
                if let Some(heads)=heads_opt {
                    t.set_format(*FORMAT_BORDERS_ONLY);
                    t.set_titles(Row::new(
                        heads.into_iter().map(|x| Cell::new(&x.to_uppercase())).collect()
                    ));

                }else{
                    t.set_format(*FORMAT_BOX_CHARS);

                }
                for row in rows {
                    t.add_row(Row::new(
                        row.into_iter().map(|x| Cell::new(&x)).collect()
                    ));
                }

                write!($f, "{}", t)
            }
            Self::Map(exprs) if $debug => write!(
                $f,
                "{{{}}}",
                exprs
                    .as_ref()
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
                t.set_format(*FORMAT_BORDERS_ONLY);

                // let heads=["Key","Value"];
                // t.set_titles(Row::new(
                //     heads.into_iter().map(|x| Cell::new(&x)).collect()
                // ));
                t.set_titles(row!("KEY","VALUE"));
                // let fmt = t.get_format();
                // fmt.padding(1, 1);
                // // Set width to be 2/3
                // fmt.borders('│');
                // fmt.column_separator('│');
                // fmt.separator(LinePosition::Top, LineSeparator::new('═', '╤', '╒', '╕'));
                // fmt.separator(LinePosition::Title, LineSeparator::new('═', '╪', '╞', '╡'));
                // fmt.separator(LinePosition::Intern, LineSeparator::new('─', '┼', '├', '┤'));
                // fmt.separator(LinePosition::Bottom, LineSeparator::new('─', '┴', '└', '┘'));

                for (key, val) in exprs.as_ref().iter() {
                    match &val {
                        Self::Builtin(Builtin { help, .. }) => {
                            t.add_row(row!(
                                key,
                                format!("{}", val),
                                textwrap::fill(help, specified_width / 2)
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
           Self::BMap(exprs) => {
                let specified_width = $f.width().unwrap_or(
                    terminal_size()
                        .map(|(Width(w), _)| w as usize)
                        .unwrap_or(120),
                );
                let mut t = Table::new();
                t.set_format(*FORMAT_BORDERS_ONLY);

                // let heads=["Key","Value"];
                // t.set_titles(Row::new(
                //     heads.into_iter().map(|x| Cell::new(&x)).collect()
                // ));
                t.set_titles(row!("KEY","VALUE"));
                // let fmt = t.get_format();
                // fmt.padding(1, 1);
                // // Set width to be 2/3
                // fmt.borders('│');
                // fmt.column_separator('│');
                // fmt.separator(LinePosition::Top, LineSeparator::new('═', '╤', '╒', '╕'));
                // fmt.separator(LinePosition::Title, LineSeparator::new('═', '╪', '╞', '╡'));
                // fmt.separator(LinePosition::Intern, LineSeparator::new('─', '┼', '├', '┤'));
                // fmt.separator(LinePosition::Bottom, LineSeparator::new('─', '┴', '└', '┘'));

                for (key, val) in exprs.as_ref().iter() {
                    match &val {
                        Self::Builtin(Builtin { help, .. }) => {
                            t.add_row(row!(
                                key,
                                format!("{}", val),
                                textwrap::fill(help, specified_width / 2)
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
            Self::Function(name, param, pc, body) => match pc {
                Some(collector) => write!(
                    $f,
                    "fn {}({:?},...{}) {{ {:?} }}",
                    name, param, collector, body
                ),
                _ => write!($f, "fn {}({:?}) {{ {:?} }}", name, param, body),
            },
            Self::Return(body) => write!($f, "return {}", body),
            Self::Break(body) => write!($f, "break {}", body),
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

            Self::Match(value, branches) => {
                write!($f, "match {:?} {{ ", value)?;
                for (pat, expr) in branches.iter() {
                    write!($f, "{:?} => {:?}, ", pat, expr)?;
                }
                write!($f, "}}")
            }
            Self::Apply(g, args) if $debug => write!(
                $f,
                "APPLY ☛{:?} {}☚ ",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Apply(g, args) => write!(
                $f,
                "{:?} {}",
                g,
                args.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Command(g, args) if $debug => write!(
                $f,
                "COMMAND ☘{:?} {})☘ ",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Command(g, args) => write!(
                $f,
                "{:?} {}",
                g,
                args.iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Self::Alias(name, cmd) => write!($f, "alias {} {:?}", name, cmd),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!($f, "({} {})", op, v)
                } else {
                    write!($f, "({} {})", v, op)
                }
            }
            Self::BinaryOp(op, l, r) => write!($f, "{:?} {} {:?}", l, op, r),
            Self::Pipe(op, l, r) => write!($f, "{:?} {} {:?}", l, op, r),
            Self::Index(l, r) => write!($f, "{}[{}]", l, r),
            Self::Builtin(builtin) => fmt::Debug::fmt(builtin, $f),
            Self::Catch(body, ctyp, deel) => match ctyp {
                CatchType::Ignore => write!($f, "{:?} ?.", body),
                CatchType::PrintStd => write!($f, "{:?} ?+", body),
                CatchType::PrintErr => write!($f, "{:?} ??", body),
                CatchType::PrintOver => write!($f, "{:?} ?!", body),
                CatchType::Deel => match deel {
                    Some(deelx) => write!($f, "{:?} ?: {}", body, deelx),
                    _ => write!($f, "{:?} ?: {{}}", body),
                },
            }, // Self::Error { code, msg, expr } => {
               //     write!($f, "Error<(code:{}\nmsg:{}\nexpr:{:?})>", code, msg, expr)
               // } // _ => write!($f, "Unreachable"), // 作为兜底逻辑
        }
    };
}

struct TableRow<'a> {
    columns: &'a Vec<Expression>, // 原始数据
    max_width: usize,             // 单行总宽度限制
    col_padding: usize,           // 列间距（通常为3：1边框+2空格）
}

impl<'a> TableRow<'a> {
    /// 智能分Row算法
    fn split_into_rows(&self) -> (Vec<Vec<String>>, Option<Vec<String>>) {
        let mut result = vec![];
        let mut heads = vec![];
        let mut current_row = vec![];
        let mut current_len = 0;

        // 二维表格
        let mut cols = match self.columns.first() {
            Some(Expression::List(a)) => {
                heads = a
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("C{}", i))
                    .collect();
                a.len()
            }
            Some(Expression::Map(a)) => {
                heads = a.iter().map(|(k, _)| k.to_owned()).collect::<Vec<String>>();
                a.keys().len()
            }
            Some(Expression::BMap(a)) => {
                heads = a.iter().map(|(k, _)| k.to_owned()).collect::<Vec<String>>();
                a.keys().len()
            }
            _ => 0,
        };
        if cols > 0 {
            for expr in self.columns.iter() {
                match expr {
                    Expression::List(a) => {
                        for c in a.iter() {
                            current_row.push(c.to_string());
                        }
                    }
                    Expression::Map(a) => {
                        for (_, v) in a.iter() {
                            current_row.push(v.to_string());
                        }
                    }
                    Expression::BMap(a) => {
                        for (_, v) in a.iter() {
                            current_row.push(v.to_string());
                        }
                    }
                    other => current_row.push(other.to_string()),
                };
                if !current_row.is_empty() {
                    result.push(current_row);
                    current_row = vec![];
                }
            }
            return (result, Some(heads));
        }

        // 一唯表格
        for (i, expr) in self.columns.iter().enumerate() {
            let col = match expr {
                Expression::List(a) => a
                    .as_ref()
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                Expression::Map(a) => a
                    .as_ref()
                    .iter()
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                Expression::BMap(a) => a
                    .as_ref()
                    .iter()
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                other => other.to_string(),
            };
            let col_width = col.chars().count() + self.col_padding;

            // 两种情况需要换行：
            // 1. 当前行已有内容且加入新列会超限
            // 2. 单列宽度已超过总限制（需强制拆分列）
            if cols == 0 {
                if !current_row.is_empty() && current_len + col_width > self.max_width {
                    cols = i;
                    // dbg!(&cols);
                    result.push(current_row);
                    current_row = vec![];
                    current_len = 0;
                }
            } else if i % cols == 0 {
                // dbg!(&i);
                result.push(current_row);
                current_row = vec![];
                current_len = 0;
            }
            // 处理超宽列（需拆分成多段）
            if col_width > self.max_width {
                let chunks = self.split_column(&col);
                for chunk in chunks {
                    if !current_row.is_empty() {
                        result.push(current_row);
                        current_row = vec![];
                    }
                    current_row.push(chunk);
                }
                current_len = current_row.last().map(|s| s.len()).unwrap_or(0);
            } else {
                current_row.push(col);
                current_len += col_width;
            }
        }

        if !current_row.is_empty() {
            result.push(current_row);
        }
        (result, None)
    }

    /// 拆分超宽列为多段
    fn split_column(&self, text: &str) -> Vec<String> {
        let max_chunk = self.max_width - self.col_padding;
        text.chars()
            .collect::<Vec<_>>()
            .chunks(max_chunk)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }
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
            Self::BMap(_) => "BMap".into(),
            Self::String(_) => "String".into(),
            Self::Integer(_) => "Integer".into(),
            Self::Symbol(_) => "Symbol".into(),
            Self::Variable(_) => "Variable".into(),

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
            Self::Loop(_) => "Loop".into(),
            Self::Match(_, _) => "Match".into(),
            Self::If(_, _, _) => "If".into(),
            Self::Apply(_, _) => "Apply".into(),
            Self::Command(_, _) => "Command".into(),
            Self::Lambda(..) => "Lambda".into(),
            // Self::Macro(_, _) => "Macro".into(),
            Self::Function(..) => "Function".into(),
            Self::Return(_) => "Return".into(),
            Self::Break(_) => "Break".into(),
            Self::Do(_) => "Do".into(),
            Self::Builtin(_) => "Builtin".into(),
            Self::Quote(_) => "Quote".into(),
            Self::Catch(..) => "Catch".into(),
            Self::Alias(..) => "Alias".into(),
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

    pub fn apply(&self, args: Vec<Self>) -> Self {
        Self::Apply(Rc::new(self.clone()), Rc::new(args))
    }
    // 参数合并方法
    pub fn append_args(&self, args: Vec<Expression>) -> Expression {
        match self {
            Expression::Apply(f, existing_args) => {
                let mut new_vec = Vec::with_capacity(existing_args.len() + args.len());
                new_vec.extend_from_slice(existing_args);
                new_vec.extend_from_slice(&args);
                Expression::Apply(f.clone(), Rc::new(new_vec))
            }
            _ => Expression::Apply(Rc::new(self.clone()), Rc::new(args)), //report error?
        }
    }
    pub fn ensure_apply(&self) -> Expression {
        match self {
            Expression::Symbol(_) => Expression::Apply(Rc::new(self.clone()), Rc::new(vec![])),
            _ => self.clone(), //others, like binop,group,pipe...
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
            Self::List(exprs) => !exprs.as_ref().is_empty(),
            Self::Map(exprs) => !exprs.as_ref().is_empty(),
            Self::Lambda(..) => true,
            // Self::Macro(_, _) => true,
            Self::Builtin(_) => true,
            _ => false,
        }
    }
    // pub fn flatten(args: Vec<Self>) -> Vec<Self> {
    //     let mut result = vec![];
    //     for arg in args {
    //         match arg {
    //             Self::List(exprs) => result.extend(Self::flatten((*exprs).to_vec())), // 解引用并转换为 Vec
    //             Self::Group(expr) => result.extend(Self::flatten(vec![*expr])),
    //             _ => result.push(arg),
    //         }
    //     }
    //     result
    // }
}
