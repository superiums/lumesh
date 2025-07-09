use super::{CatchType, Expression};
use crate::expression::{ChainCall, DestructurePattern};
use crate::{RuntimeError, RuntimeErrorKind};
use std::borrow::Cow;
// use num_traits::pow;
use std::fmt;
use std::rc::Rc;
// use terminal_size::{Width, terminal_size};

// const GREEN_BOLD: &str = "\x1b[1;32m";
// // const RED: &str = "\x1b[31m";
// const RESET: &str = "\x1b[0m";
// fn green(s: &str) -> String {
//     format!("{}{}{}", GREEN_BOLD, s, RESET)
// }
// 错误处理宏（优化点）
// macro_rules! type_error {
//     ($expected:expr, $found:expr) => {
//         Err(RuntimeError::TypeError {
//             expected: $expected.into(),
//             sym: $found.to_string(),
//             found: $found.type_name().into(),
//         })
//     };
// }

impl fmt::Display for DestructurePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rest(s) => write!(f, "...{}", s),
            Self::Identifier(s) => write!(f, "{}", s),
            Self::Renamed((k, n)) => write!(f, "{}:{}", k, n),
        }
    }
}
// Debug 实现
impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_indent(f, 0)
    }
}
// 优化缩进函数，缓存常用缩进
fn idt(indent: usize) -> &'static str {
    const INDENTS: [&str; 16] = [
        "",
        "  ",
        "    ",
        "      ",
        "        ",
        "          ",
        "            ",
        "              ",
        "                ",
        "                  ",
        "                    ",
        "                      ",
        "                        ",
        "                          ",
        "                            ",
        "                              ",
    ];

    if indent < INDENTS.len() {
        INDENTS[indent]
    } else {
        // 对于超过缓存范围的缩进，回退到原来的实现
        Box::leak("  ".repeat(indent).into_boxed_str())
    }
}
// Display 实现
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Symbol(name) => write!(f, "{}", name),
            Self::Variable(name) => write!(f, "${}", name),
            Self::FileSize(fsz) => write!(f, "{}", fsz.to_human_readable()),
            Self::String(s) => write!(f, "{}", s),
            Self::StringTemplate(s) => write!(f, "`{}`", s),
            Self::Integer(i) => write!(f, "{}", *i),
            Self::Float(n) => write!(f, "{}", *n),
            Self::Bytes(b) => write!(f, "b{}", String::from_utf8_lossy(b)),
            Self::Boolean(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Self::DateTime(n) => write!(f, "{}", n.format("%Y-%m-%d %H:%M")),
            Self::TimeDef(s) => write!(f, "t'{}'", s),
            Self::RegexDef(s) => write!(f, "r'{}'", s),
            Self::Regex(s) => write!(f, "r'{}'", s.regex.as_str()),

            Self::Declare(name, expr) => write!(f, "let {} = {}", name, expr),
            Self::DestructureAssign(pattern, expr) => {
                write!(
                    f,
                    "let {} = {}",
                    pattern
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    expr
                )
            }
            Self::Assign(name, expr) => write!(f, "{} = {}", name, expr),

            Self::Quote(inner) => write!(f, "'{}", inner),
            Self::Group(inner) => write!(f, "({})", inner),

            Self::While(cond, body) => write!(f, "while {}\n\t{}", cond, body),
            Self::Loop(inner) => write!(f, "loop {}", inner),

            Self::Lambda(params, body) => write!(f, "({}) -> {}", params.join(", "), body),

            Self::If(cond, true_expr, false_expr) => {
                write!(f, "if {}\n\t{}\nelse\n\t{}", cond, true_expr, false_expr)
            }

            Self::Slice(l, r) => write!(
                f,
                "{}[{}:{}:{}]",
                l,
                match &r.start {
                    Some(s) => s.as_ref(),
                    _ => &Expression::None,
                },
                match &r.end {
                    Some(s) => s.as_ref(),
                    _ => &Expression::None,
                },
                match &r.step {
                    Some(s) => s.as_ref(),
                    _ => &Expression::None,
                },
            ),

            Self::List(exprs) => {
                write!(
                    f,
                    "[{}]",
                    exprs
                        .as_ref()
                        .iter()
                        .map(|e| format!("{}", e))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }

            Self::HMap(exprs) => {
                write!(
                    f,
                    "{{{}}}",
                    exprs
                        .as_ref()
                        .iter()
                        .map(|(k, v)| format!("{}:{}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::Map(exprs) => {
                write!(
                    f,
                    "{{{}}}",
                    exprs
                        .as_ref()
                        .iter()
                        .map(|(k, v)| format!("{}:{}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }

            Self::None => Ok(()),
            Self::Chain(t, c) => {
                write!(f, "{}", t)?;
                for call in c {
                    write!(
                        f,
                        ".{}({})",
                        call.method,
                        call.args
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    )?;
                }
                Ok(())
            }
            Self::PipeMethod(t, a) => write!(
                f,
                ".{}({})",
                t,
                a.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::Function(name, param, pc, body, _) => match pc {
                Some(collector) => write!(
                    f,
                    "fn {}({},...{}) {{\n\t{}\n}}",
                    name,
                    param
                        .iter()
                        .map(|(p, v)| match v {
                            Some(vv) => format!("{}={}", p, vv),
                            _ => p.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                    collector,
                    body
                ),
                _ => write!(
                    f,
                    "fn {}({}) {{\n\t{}\n}}",
                    name,
                    param
                        .iter()
                        .map(|(p, v)| match v {
                            Some(vv) => format!("{}={}", p, vv),
                            _ => p.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                    body
                ),
            },
            Self::Return(body) => write!(f, "return {}", body),
            Self::Break(body) => write!(f, "break {}", body),
            Self::For(name, list, body) => write!(f, "for {} in {}\n\t{}", name, list, body),
            Self::Do(exprs) => write!(
                f,
                "{{\n\t{}\n\t}}",
                exprs
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(";\n")
            ),

            Self::Del(name) => write!(f, "del {}", name),

            Self::Match(value, branches) => {
                write!(f, "match {} {{", value)?;
                for (pat, expr) in branches.iter() {
                    write!(
                        f,
                        "\n\t{} => {}, ",
                        pat.iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        expr
                    )?;
                }
                write!(f, "\n}}")
            }

            Self::Apply(g, args) => write!(
                f,
                "{}({})",
                g,
                args.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::Command(g, args) => write!(
                f,
                "{}  {}",
                g,
                args.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),

            Self::AliasDef(name, cmd) => write!(f, "alias {} = {}", name, cmd),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!(f, "({} {})", op, v)
                } else {
                    write!(f, "({} {})", v, op)
                }
            }
            Self::Range(r, st) => write!(f, "{}..<{}:{}", r.start, r.end, st),
            Self::BinaryOp(op, l, r) => write!(f, "{} {} {}", l, op, r),
            Self::RangeOp(op, l, r, step) => match step {
                Some(st) => write!(f, "{}{}{}:{}", l, op, r, st),
                _ => write!(f, "{}{}{}", l, op, r),
            },
            Self::Pipe(op, l, r) => write!(f, "{} {} {}", l, op, r),
            Self::Index(l, r) => write!(f, "{}[{}]", l, r),
            Self::Builtin(builtin) => write!(f, "{}", builtin),
            Self::Use(n, p) => write!(
                f,
                "use {} as {}",
                p,
                match n {
                    Some(name) => name.as_str(),
                    _ => "_",
                }
            ),
            Self::Catch(body, ctyp, deel) => match ctyp {
                CatchType::Ignore => write!(f, "{} ?.", body),
                CatchType::PrintStd => write!(f, "{} ?+", body),
                CatchType::PrintErr => write!(f, "{} ??", body),
                CatchType::PrintOver => write!(f, "{} ?>", body),
                CatchType::Terminate => write!(f, "{} ?!", body),
                CatchType::Deel => match deel {
                    Some(deelx) => write!(f, "{} ?: {}", body, deelx),
                    _ => write!(f, "{} ?: {{}}", body),
                },
            },
        }
    }
}

fn fmt_binary_op(
    f: &mut fmt::Formatter,
    op_name: &str,
    op: &str,
    left: &Expression,
    right: &Expression,
    step: &Option<Rc<Expression>>,
    i: usize,
) -> fmt::Result {
    // let i = if f.alternate() { indent + 1 } else { indent };
    write!(f, "\n{}{}〈{}〉\n", idt(i), op_name, op)?;
    left.fmt_indent(f, i + 1)?;
    write!(f, "\n")?;
    right.fmt_indent(f, i + 1)?;
    if let Some(step_expr) = step {
        write!(f, "\n{}step:\n", idt(i + 1))?;
        step_expr.fmt_indent(f, i + 2)?;
    }
    Ok(())
}

// Expression 辅助函数
impl Expression {
    fn fmt_indent(&self, f: &mut fmt::Formatter, indent: usize) -> fmt::Result {
        let i = if f.alternate() { indent + 1 } else { indent };
        write!(f, "{}", idt(i))?;
        match &self {
            // 基础类型 - 保持原有实现
            Self::Symbol(s) => write!(f, "Symbol〈{:?}〉", s),
            Self::Variable(s) => write!(f, "Variable〈{:?}〉", s),
            Self::String(s) => write!(f, "String〈{:?}〉", s),
            Self::Integer(s) => write!(f, "Integer〈{:?}〉", s),
            Self::Float(s) => write!(f, "Float〈{:?}〉", s),
            Self::Boolean(s) => write!(f, "Boolean〈{:?}〉", s),
            Self::DateTime(s) => write!(f, "DateTime〈{:?}〉", s),
            Self::FileSize(s) => write!(f, "FileSize〈{:?}〉", s),
            Self::Range(s, st) => write!(f, "Range〈{:?},{}〉", s, st),
            Self::Quote(inner) => write!(f, "Quote〈{:?}〉", inner),
            Self::Group(inner) => write!(f, "Group〈{:?}〉", inner),
            Self::TimeDef(s) => write!(f, "TimeDef〈{:?}〉", s),
            Self::RegexDef(s) => write!(f, "RegexDef〈{:?}〉", s),
            Self::Regex(s) => write!(f, "Regex〈{:?}〉", s.regex.as_str()),
            Self::None => write!(f, "None"),

            // 新增：字符串模板和字节数据
            Self::StringTemplate(s) => write!(f, "StringTemplate〈`{}`〉", s),
            Self::Bytes(b) => write!(f, "Bytes〈{:?}〉", String::from_utf8_lossy(b)),

            // 新增：声明和赋值操作
            Self::Declare(name, expr) => {
                write!(f, "\n{}Declare〈{}〉 =\n", idt(i), name)?;
                expr.fmt_indent(f, i + 1)
            }
            Self::DestructureAssign(pattern, expr) => {
                write!(f, "\n{}DestructureAssign〈{:?}〉 =\n", idt(i), pattern)?;
                expr.fmt_indent(f, i + 1)
            }
            Self::Assign(name, expr) => {
                write!(f, "\n{}Assign〈{}〉 =\n", idt(i), name)?;
                expr.fmt_indent(f, i + 1)
            }

            // 新增：删除和控制流语句
            Self::Del(name) => write!(f, "Del〈{}〉", name),
            Self::Return(expr) => {
                write!(f, "\n{}Return\n", idt(i))?;
                expr.fmt_indent(f, i + 1)
            }
            Self::Break(expr) => {
                write!(f, "\n{}Break\n", idt(i))?;
                expr.fmt_indent(f, i + 1)
            }

            // 新增：操作符
            Self::UnaryOp(op, expr, is_prefix) => {
                write!(f, "\n{}UnaryOp〈{}, prefix:{}〉\n", idt(i), op, is_prefix)?;
                expr.fmt_indent(f, i + 1)
            }
            Self::RangeOp(op, l, r, step) => fmt_binary_op(f, "RangeOp", op, l, r, step, i),

            // 新增：索引和切片操作
            Self::Index(expr, index) => {
                write!(f, "\n{}Index\n", idt(i))?;
                expr.fmt_indent(f, i + 1)?;
                write!(f, "\n{}[\n", idt(i))?;
                index.fmt_indent(f, i + 1)?;
                write!(f, "\n{}]\n", idt(i))
            }
            Self::Slice(expr, params) => {
                write!(f, "\n{}Slice\n", idt(i))?;
                expr.fmt_indent(f, i + 1)?;
                write!(
                    f,
                    "\n{}[{}:{}:{}]\n",
                    idt(i),
                    params
                        .start
                        .as_ref()
                        .map_or("None".to_string(), |s| format!("{:?}", s)),
                    params
                        .end
                        .as_ref()
                        .map_or("None".to_string(), |s| format!("{:?}", s)),
                    params
                        .step
                        .as_ref()
                        .map_or("None".to_string(), |s| format!("{:?}", s))
                )
            }

            // 新增：链式调用和管道方法
            Self::Chain(expr, calls) => {
                write!(f, "\n{}Chain\n", idt(i))?;
                expr.fmt_indent(f, i + 1)?;
                for call in calls {
                    write!(f, "\n{}.{}(", idt(i + 1), call.method)?;
                    for (idx, arg) in call.args.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{:?}", arg)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Self::PipeMethod(method, args) => {
                write!(f, "\n{}PipeMethod〈{}〉\n{}(\n", idt(i), method, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = write!(f, "\n");
                });
                write!(f, "{})\n", idt(i))
            }

            // 新增：别名操作
            Self::AliasDef(name, cmd) => {
                write!(f, "\n{}AliasDef〈{}〉 =\n", idt(i), name)?;
                cmd.fmt_indent(f, i + 1)
            }

            // 新增：内置函数
            Self::Builtin(builtin) => write!(f, "Builtin〈{:?}〉", builtin),

            // 新增：错误捕获
            Self::Catch(body, ctyp, deel) => {
                write!(f, "\n{}Catch〈{:?}〉\n", idt(i), ctyp)?;
                body.fmt_indent(f, i + 1)?;
                if let Some(deel_expr) = deel {
                    write!(f, "\n{}handler:\n", idt(i + 1))?;
                    deel_expr.fmt_indent(f, i + 2)?;
                }
                Ok(())
            }

            // 新增：模块相关
            Self::Use(name, path) => {
                write!(
                    f,
                    "Use〈{} as {}〉",
                    path,
                    name.as_ref().map_or("_", |n| n.as_str())
                )
            }

            // 集合类型 - 保持原有实现
            Self::List(exprs) => {
                write!(f, "\n{}[\n", idt(i))?;
                exprs.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = write!(f, ",\n");
                });
                write!(f, "{}]\n", idt(i))
            }
            Self::HMap(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|(k, v)| {
                    let _ = write!(f, "\n{}{:?}:", idt(i + 1), k);
                    let _ = v.fmt_indent(f, i + 2);
                    let _ = write!(f, "\n");
                });
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Map(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|(k, v)| {
                    let _ = write!(f, "\n{}{:?}:", idt(i + 1), k);
                    let _ = v.fmt_indent(f, i + 2);
                    let _ = write!(f, "\n");
                });
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Do(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = write!(f, "\n");
                });
                write!(f, "{}}}\n", idt(i))
            }

            // 二元操作 - 保持原有实现
            Self::BinaryOp(op, l, r) => fmt_binary_op(f, "BinaryOp", op, l, r, &None, i),
            Self::Pipe(op, l, r) => fmt_binary_op(f, "Pipe", op, l, r, &None, i),

            // 控制流 - 保持原有实现
            Self::If(cond, true_expr, false_expr) => {
                write!(f, "\n{}if\n", idt(i))?;
                cond.fmt_indent(f, i + 1)?;
                write!(f, "\n{}then\n", idt(i))?;
                true_expr.fmt_indent(f, i + 1)?;
                write!(f, "\n{}else\n", idt(i))?;
                false_expr.fmt_indent(f, i + 1)
            }
            Self::Match(value, branches) => {
                write!(f, "\n{}match\n", idt(i))?;
                value.fmt_indent(f, i + 1)?;
                write!(f, "\n{}{{\n", idt(i))?;
                for (pat, expr) in branches.iter() {
                    write!(
                        f,
                        "\n{}{} =>\n",
                        idt(i + 1),
                        pat.iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    )?;
                    expr.fmt_indent(f, i + 2)?;
                }
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::For(name, list, body) => {
                write!(f, "\n{}for {} in\n", idt(i), name)?;
                list.fmt_indent(f, i + 1)?;
                write!(f, "\n{}{{\n", idt(i))?;
                body.fmt_indent(f, i + 1)?;
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::While(cond, body) => {
                write!(f, "\n{}while\n", idt(i))?;
                cond.fmt_indent(f, i + 1)?;
                write!(f, "\n{}{{\n", idt(i))?;
                body.as_ref().fmt_indent(f, i + 1)?;
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Loop(body) => {
                write!(f, "\n{}loop {{\n", idt(i))?;
                body.as_ref().fmt_indent(f, i + 1)?;
                write!(f, "\n{}}}\n", idt(i))
            }

            // 函数相关 - 保持原有实现
            Self::Lambda(params, body) => {
                write!(
                    f,
                    "\n{}Lambda ({})\n",
                    idt(i),
                    params.iter().cloned().collect::<Vec<_>>().join(",")
                )?;
                body.as_ref().fmt_indent(f, i + 1)
            }
            Self::Function(name, param, pc, body, _) => {
                write!(
                    f,
                    "\n{}fn {}({},*{})\n",
                    idt(i),
                    name,
                    param
                        .iter()
                        .map(|(p, v)| match v {
                            Some(vv) => format!("{}={}", p, vv),
                            _ => p.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                    pc.clone().unwrap_or("None".to_string())
                )?;
                body.fmt_indent(f, i + 1)
            }
            Self::Apply(func, args) => {
                write!(f, "\n{}Apply\n", idt(i))?;
                func.fmt_indent(f, i + 1)?;
                write!(f, "\n{}(\n", idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = write!(f, "\n");
                });
                write!(f, "{})\n", idt(i))
            }
            Self::Command(cmd, args) => {
                write!(f, "\n{}Cmd\n", idt(i))?;
                cmd.fmt_indent(f, i + 1)?;
                write!(f, "\n{}〖\n", idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                write!(f, "{}〗\n", idt(i))
            }
        }
    }

    /// 类型名称
    pub fn get_module_name(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::List(_) | Self::Range(..) => Some("List".into()),
            Self::Map(_) | Self::HMap(_) => Some("Map".into()),
            Self::String(_) | Self::StringTemplate(_) | Self::Bytes(_) => Some("String".into()),
            Self::Integer(_) | Self::Float(_) => Some("Math".into()),
            Self::DateTime(_) => Some("Time".into()),
            Self::Boolean(_) => Some("Boolean".into()),
            Self::Regex(_) => Some("Regex".into()),
            _ => None,
        }
    }
    pub fn type_name(&self) -> String {
        match self {
            Self::List(_) => "List".into(),
            Self::HMap(_) => "HMap".into(),
            Self::FileSize(_) => "FileSize".into(),
            Self::Map(_) => "Map".into(),
            Self::String(_) => "String".into(),
            Self::StringTemplate(_) => "StringTemplate".into(),
            Self::Integer(_) => "Integer".into(),
            Self::DateTime(_) => "DateTime".into(),
            Self::Symbol(_) => "Symbol".into(),
            Self::Variable(_) => "Variable".into(),

            Self::Float(_) => "Float".into(),
            Self::Boolean(_) => "Boolean".into(),
            Self::Group(_) => "Group".into(),
            Self::BinaryOp(_, _, _) => "BinaryOp".into(),
            Self::RangeOp(..) => "RangeOp".into(),
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

            Self::AliasDef(..) => "AliasDef".into(),
            Self::Range(..) => "Range".into(),
            Self::Chain(_, _) => "Chain".into(),
            Self::PipeMethod(_, _) => "PipeMethod".into(),
            Self::DestructureAssign(_, _) => "DestructureAssign".into(),

            // Self::Error { .. } => "Error".into(),
            Self::Use(..) => "Use".into(),
            Self::TimeDef(..) => "TimeDef".into(),
            Self::RegexDef(..) => "RegexDef".into(),
            Self::Regex(..) => "Regex".into(),

            Self::None => "None".into(),
            // _ => format!("{:?}", self).split('(').next().unwrap().into(),
        }
    }

    /// 符号转换
    pub fn to_symbol(&self) -> Result<&str, RuntimeError> {
        if let Self::Symbol(s) = self {
            Ok(s)
        } else {
            // type_error!("symbol", self)
            //     ($expected:expr, $found:expr) => {
            Err(RuntimeError {
                kind: RuntimeErrorKind::TypeError {
                    expected: "symbol".to_string(),
                    sym: self.to_string(),
                    found: self.type_name().into(),
                },
                context: self.clone(),
                depth: 0,
            })
            // };
        }
    }

    pub fn apply(&self, args: Vec<Self>) -> Self {
        Self::Apply(Rc::new(self.clone()), Rc::new(args))
    }
    pub fn execute(&self, args: Vec<Self>) -> Self {
        Self::Command(Rc::new(self.clone()), Rc::new(args))
    }
    // 参数合并方法
    pub fn replace_or_append_arg(&self, arg: Expression) -> Expression {
        let mut found = false;
        match self {
            Expression::Apply(f, existing_args) => {
                let new_args = existing_args
                    .iter()
                    .map(|a| match a {
                        Self::Symbol(inner) if inner == "_" => {
                            found = true;
                            arg.clone()
                        }
                        _ => a.clone(),
                    })
                    .collect();
                if found {
                    Expression::Apply(f.clone(), Rc::new(new_args))
                } else {
                    self.append_args(vec![arg])
                }
            }
            Expression::Command(f, existing_args) => {
                let new_args = existing_args
                    .iter()
                    .map(|a| match a {
                        Self::Symbol(inner) if inner == "_" => {
                            found = true;
                            arg.clone()
                        }
                        _ => a.clone(),
                    })
                    .collect();
                if found {
                    Expression::Command(f.clone(), Rc::new(new_args))
                } else {
                    self.append_args(vec![arg])
                }
            }
            Expression::Chain(base, calls) => {
                if calls.is_empty() {
                    return Expression::Chain(base.clone(), calls.clone());
                } else {
                    let (call, others) = calls.split_at(1);
                    let mut new_args: Vec<Expression> = call[0]
                        .args
                        .iter()
                        .map(|a| match a {
                            Self::Symbol(inner) if inner == "_" => {
                                found = true;
                                arg.clone()
                            }
                            _ => a.clone(),
                        })
                        .collect();
                    if !found {
                        new_args.push(arg);
                    }
                    let mut new_calls = vec![ChainCall {
                        method: call[0].method.clone(),
                        args: new_args,
                    }];
                    new_calls.extend_from_slice(others);
                    return Expression::Chain(base.clone(), new_calls);
                }
            }
            _ => Expression::Command(Rc::new(self.clone()), Rc::new(vec![arg])), //report error?
        }
    }
    /// please make sure only use with Apply/Command
    pub fn append_args(&self, args: Vec<Expression>) -> Expression {
        match self {
            Expression::Apply(f, existing_args) => {
                let mut new_vec = Vec::with_capacity(existing_args.len() + args.len());
                new_vec.extend_from_slice(existing_args);
                new_vec.extend_from_slice(&args);
                Expression::Apply(f.clone(), Rc::new(new_vec))
            }
            Expression::Command(f, existing_args) => {
                let mut new_vec = Vec::with_capacity(existing_args.len() + args.len());
                new_vec.extend_from_slice(existing_args);
                new_vec.extend_from_slice(&args);
                Expression::Command(f.clone(), Rc::new(new_vec))
            }
            Expression::Chain(base, calls) => {
                if calls.is_empty() {
                    return Expression::Chain(base.clone(), calls.clone());
                } else {
                    let (call, others) = calls.split_at(1);

                    let mut new_vec = Vec::with_capacity(call[0].args.len() + args.len());
                    new_vec.extend_from_slice(&call[0].args);
                    new_vec.extend_from_slice(&args);
                    let mut new_calls = vec![ChainCall {
                        method: call[0].method.clone(),
                        args: new_vec,
                    }];
                    new_calls.extend_from_slice(others);
                    return Expression::Chain(base.clone(), new_calls);
                }
            }
            _ => unreachable!(), // _ => Expression::Command(Rc::new(self.clone()), Rc::new(args)), //report error?
        }
    }
    pub fn ensure_fn_apply(&self) -> Expression {
        match self {
            Expression::Function(..) | Expression::Lambda(..) | Expression::Builtin(..) => {
                self.apply(vec![])
            }
            // symbol maybe alias, but also maybe var/string, so let user decide.
            // Expression::Symbol(_) => Expression::Command(Rc::new(self.clone()), Rc::new(vec![])),
            _ => self.clone(), //others, like binop,group,pipe...
        }
    }

    // pub fn set_status_code(&self, code: Int, env: &mut Environment) {
    //     env.define("STATUS", Expression::Integer(code));
    // }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Integer(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.is_empty(),
            Self::Bytes(b) => !b.is_empty(),
            Self::FileSize(b) => b.size != 0,
            Self::Boolean(b) => *b,
            Self::List(exprs) => !exprs.as_ref().is_empty(),
            Self::HMap(exprs) => !exprs.as_ref().is_empty(),
            Self::Map(exprs) => !exprs.as_ref().is_empty(),
            Self::Range(exprs, _) => !exprs.is_empty(),
            Self::Lambda(..) => true,
            Self::DateTime(..) => true,
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
