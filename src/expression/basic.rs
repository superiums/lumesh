use super::{CatchType, Expression};
use crate::expression::{ChainCall, DestructurePattern};
use crate::libs::{is_lib, is_top_or_se};
use crate::{RuntimeError, RuntimeErrorKind};
use std::borrow::Cow;
// use num_traits::pow;
use std::fmt;
use std::rc::Rc;

impl fmt::Display for DestructurePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rest(s) => write!(f, "*{s}"),
            Self::Identifier(s) => write!(f, "{s}"),
            Self::Renamed((k, n)) => write!(f, "{k}:{n}"),
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
// Display 实现 - 改进为代码格式化
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_display_indent(f, 0)
    }
}
impl Expression {
    fn fmt_display_indent(&self, f: &mut fmt::Formatter, i: usize) -> fmt::Result {
        match self {
            // 基础类型 - 支持缩进
            Self::Symbol(name) => write!(f, "{}{name}", idt(i)),
            Self::Variable(name) => write!(f, "{}${name}", idt(i)),
            Self::Integer(it) => write!(f, "{}{it}", idt(i)),
            Self::Float(n) => write!(f, "{}{n}", idt(i)),
            Self::String(s) => write!(f, "{}{s}", idt(i)),
            Self::StringTemplate(segments) => {
                write!(f, "{}`", idt(i))?;
                for seg in segments.iter() {
                    match seg {
                        Self::String(s) => write!(f, "{s}")?,
                        Self::Variable(v) => write!(f, "${v}")?,
                        other => write!(f, "${{{other}}}")?,
                    }
                }
                write!(f, "`")
            }
            Self::Boolean(b) => write!(f, "{}{}", idt(i), if *b { "true" } else { "false" }),
            Self::Bytes(b) => write!(f, "{}b\"{}\"", idt(i), String::from_utf8_lossy(b)),
            Self::DateTime(n) => write!(f, "{}{}", idt(i), n.format("%Y-%m-%d %H:%M:%S")),
            Self::FileSize(fsz) => write!(f, "{}{}", idt(i), fsz.to_human_readable()),
            Self::None => write!(f, "{}", idt(i)),

            Self::Sequence(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}{{", idt(i))?;
                    for expr in exprs.iter() {
                        expr.fmt_display_indent(f, i + 1)?;
                        writeln!(f)?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}{{ ", idt(i))?;
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, "; ")?;
                        }
                        expr.fmt_display_indent(f, 0)?;
                    }
                    write!(f, " }}")
                }
            }
            Self::SetParent(name, expr) => {
                if f.alternate() {
                    write!(f, "{}set {} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}set {} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, 0)
                }
            }
            Self::Export(name, expr) => {
                write!(f, "{}export {}", idt(i), name)?;
                if let Some(exp) = expr {
                    write!(f, " = ")?;
                    if f.alternate() {
                        exp.fmt_display_indent(f, i + 1)?;
                    } else {
                        exp.fmt_display_indent(f, 0)?;
                    }
                }
                Ok(())
            }
            // 声明和赋值
            Self::Declare(name, expr) => {
                if f.alternate() {
                    write!(f, "{}let {} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}let {} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, 0)
                }
            }
            Self::DestructureAssign(pattern, expr) => {
                write!(f, "{}let ", idt(i))?;
                for (i, p) in pattern.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, " = ")?;
                expr.fmt_display_indent(f, if f.alternate() { i + 1 } else { 0 })
            }
            Self::Assign(name, expr) => {
                if f.alternate() {
                    write!(f, "{}{} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}{} = ", idt(i), name)?;
                    expr.fmt_display_indent(f, 0)
                }
            }

            // 引用和分组 - 修复括号对齐
            Self::Quote(inner) => {
                write!(f, "{}'", idt(i))?;
                inner.fmt_display_indent(f, 0)
            }
            Self::Group(inner) => {
                if f.alternate() {
                    write!(f, "{}(", idt(i))?;
                    inner.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{})", idt(i))
                } else {
                    write!(f, "{}(", idt(i))?;
                    inner.fmt_display_indent(f, 0)?;
                    write!(f, ")")
                }
            }

            // 控制流 - 修复括号对齐
            Self::If(cond, true_expr, false_expr) => {
                if f.alternate() {
                    write!(f, "{}if ", idt(i))?;
                    cond.fmt_display_indent(f, 0)?;
                    writeln!(f, " {{")?;
                    true_expr.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}} else {{\n", idt(i))?;
                    false_expr.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}}", idt(i))
                } else {
                    write!(f, "{}if ", idt(i))?;
                    cond.fmt_display_indent(f, 0)?;
                    write!(f, " {{ ")?;
                    true_expr.fmt_display_indent(f, 0)?;
                    write!(f, " }} else {{ ")?;
                    false_expr.fmt_display_indent(f, 0)?;
                    write!(f, " }}")
                }
            }

            Self::While(cond, body) => {
                if f.alternate() {
                    write!(f, "{}while ", idt(i))?;
                    cond.fmt_display_indent(f, 0)?;
                    writeln!(f, " {{")?;
                    body.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}}", idt(i))
                } else {
                    write!(f, "{}while ", idt(i))?;
                    cond.fmt_display_indent(f, 0)?;
                    write!(f, " {{ ")?;
                    body.fmt_display_indent(f, 0)?;
                    write!(f, " }}")
                }
            }

            Self::Loop(body) => {
                if f.alternate() {
                    writeln!(f, "{}loop {{", idt(i))?;
                    body.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}}", idt(i))
                } else {
                    write!(f, "{}loop {{ ", idt(i))?;
                    body.fmt_display_indent(f, 0)?;
                    write!(f, " }}")
                }
            }

            Self::For(name, ind, list, body) => {
                if let Some(index) = ind {
                    write!(f, "{}for {},{} in ", idt(i), index, name)?;
                } else {
                    write!(f, "{}for {} in ", idt(i), name)?;
                }
                list.fmt_display_indent(f, 0)?;
                writeln!(f, " {{")?;
                if f.alternate() {
                    body.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}}", idt(i))
                } else {
                    body.fmt_display_indent(f, 0)?;
                    write!(f, " }}")
                }
            }

            Self::Match(value, branches) => {
                if f.alternate() {
                    write!(f, "{}match ", idt(i))?;
                    value.fmt_display_indent(f, 0)?;
                    writeln!(f, " {{")?;
                    for (pat, expr) in branches.iter() {
                        write!(
                            f,
                            "{}{} => ",
                            idt(i + 1),
                            pat.iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        )?;
                        expr.fmt_display_indent(f, 0)?;
                        writeln!(f, ",")?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}match ", idt(i))?;
                    value.fmt_display_indent(f, 0)?;
                    write!(f, " {{ ")?;
                    for (i, (pat, expr)) in branches.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(
                            f,
                            "{} => ",
                            pat.iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        )?;
                        expr.fmt_display_indent(f, 0)?;
                    }
                    write!(f, " }}")
                }
            }

            // 函数定义 - 修复括号对齐
            Self::Lambda(params, body, _) => {
                if f.alternate() {
                    write!(f, "{}({}) -> ", idt(i), params.join(", "))?;
                    if matches!(body.as_ref(), Self::Block(_)) {
                        writeln!(f)?;
                        body.fmt_display_indent(f, i + 1)
                    } else {
                        body.fmt_display_indent(f, i + 1)
                    }
                } else {
                    write!(f, "{}({}) -> ", idt(i), params.join(", "))?;
                    body.fmt_display_indent(f, 0)
                }
            }

            Self::Function(name, params, collector, body, _) => {
                if f.alternate() {
                    write!(f, "{}fn {}(", idt(i), name)?;
                    for (i, (param, default)) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{param}")?;
                        if let Some(def) = default {
                            write!(f, " = {def}")?;
                        }
                    }
                    if let Some(coll) = collector {
                        if !params.is_empty() {
                            write!(f, ", ")?;
                        }
                        write!(f, "*{coll}")?;
                    }
                    writeln!(f, ") {{")?;
                    body.fmt_display_indent(f, i + 1)?;
                    write!(f, "\n{}}}", idt(i))
                } else {
                    write!(f, "{}fn {}(", idt(i), name)?;
                    for (i, (param, default)) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{param}")?;
                        if let Some(def) = default {
                            write!(f, " = {def}")?;
                        }
                    }
                    if let Some(coll) = collector {
                        if !params.is_empty() {
                            write!(f, ", ")?;
                        }
                        write!(f, "*{coll}")?;
                    }
                    write!(f, ") {{ ")?;
                    body.fmt_display_indent(f, 0)?;
                    write!(f, " }}")
                }
            }

            // 代码块 - 修复括号对齐和元素缩进
            Self::Block(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}{{", idt(i))?;
                    for expr in exprs.iter() {
                        expr.fmt_display_indent(f, i + 1)?;
                        writeln!(f)?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}{{ ", idt(i))?;
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, "; ")?;
                        }
                        expr.fmt_display_indent(f, 0)?;
                    }
                    write!(f, " }}")
                }
            }

            // 集合类型 - 修复缩进累积问题
            Self::List(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}[", idt(i))?;
                    for expr in exprs.iter() {
                        expr.fmt_display_indent(f, i + 1)?;
                        writeln!(f, ",")?;
                    }
                    write!(f, "{}]", idt(i))
                } else {
                    write!(f, "{}[", idt(i))?;
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        expr.fmt_display_indent(f, 0)?;
                    }
                    write!(f, "]")
                }
            }
            Self::BSet(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}S{{", idt(i))?;
                    for expr in exprs.iter() {
                        expr.fmt_display_indent(f, i + 1)?;
                        writeln!(f, ",")?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}{{", idt(i))?;
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        expr.fmt_display_indent(f, 0)?;
                    }
                    write!(f, "}}")
                }
            }

            Self::Map(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}{{", idt(i))?;
                    for (k, v) in exprs.iter() {
                        write!(f, "{}{k}: ", idt(i + 1))?; // key 使用 i+1
                        match v {
                            Self::Symbol(_)
                            | Self::Integer(_)
                            | Self::Float(_)
                            | Self::Boolean(_)
                            | Self::String(_) => {
                                v.fmt_display_indent(f, 0)?;
                            }
                            _ => {
                                writeln!(f)?;
                                v.fmt_display_indent(f, i + 2)?
                            }
                        }
                        writeln!(f, ",")?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}{{", idt(i))?;
                    for (i, (k, v)) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{k}: ")?;
                        v.fmt_display_indent(f, 0)?;
                    }
                    write!(f, "}}")
                }
            }

            Self::HMap(exprs) => {
                if f.alternate() {
                    writeln!(f, "{}{{", idt(i))?;
                    for (k, v) in exprs.iter() {
                        write!(f, "{}{k}: ", idt(i + 1))?; // key 使用 i+1
                        v.fmt_display_indent(f, 0)?;
                        writeln!(f, ",")?;
                    }
                    write!(f, "{}}}", idt(i))
                } else {
                    write!(f, "{}{{", idt(i))?;
                    for (i, (k, v)) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{k}: ")?;
                        v.fmt_display_indent(f, 0)?;
                    }
                    write!(f, "}}")
                }
            }

            // 操作符
            Self::BinaryOp(op, l, r) => {
                write!(f, "{}", idt(i))?;
                l.fmt_display_indent(f, 0)?;
                write!(f, " {op} ")?;
                r.fmt_display_indent(f, 0)
            }

            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!(f, "{}{op}", idt(i))?;
                    v.fmt_display_indent(f, 0)
                } else {
                    write!(f, "{}", idt(i))?;
                    v.fmt_display_indent(f, 0)?;
                    write!(f, "{op}")
                }
            }

            Self::RangeOp(op, l, r, step) => {
                write!(f, "{}", idt(i))?;
                l.fmt_display_indent(f, 0)?;
                write!(f, "{op}")?;
                r.fmt_display_indent(f, 0)?;
                if let Some(st) = step {
                    write!(f, ":{st}")?;
                }
                Ok(())
            }

            Self::Pipe(op, l, r) => {
                write!(f, "{}", idt(i))?;
                l.fmt_display_indent(f, 0)?;
                write!(f, " {op} ")?;
                r.fmt_display_indent(f, 0)
            }

            // 函数调用
            Self::Apply(func, args) => {
                write!(f, "{}", idt(i))?;
                func.fmt_display_indent(f, 0)?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.fmt_display_indent(f, 0)?;
                }
                write!(f, ")")
            }

            Self::Command(cmd, args) | Self::CommandRaw(cmd, args) => {
                write!(f, "{}", idt(i))?;
                cmd.fmt_display_indent(f, 0)?;
                for arg in args.iter() {
                    write!(f, " ")?;
                    arg.fmt_display_indent(f, 0)?;
                }
                Ok(())
            }

            Self::ModuleCall(mo, func) => {
                write!(f, "{}{}::{}", idt(i), mo.join("::"), func)
            }

            // 索引和切片
            Self::Index(l, r) => {
                write!(f, "{}", idt(i))?;
                l.fmt_display_indent(f, 0)?;
                write!(f, "[")?;
                r.fmt_display_indent(f, 0)?;
                write!(f, "]")
            }
            Self::Property(l, r) => {
                write!(f, "{}", idt(i))?;
                l.fmt_display_indent(f, 0)?;
                write!(f, ".{}", r)
            }

            // 其他构造
            Self::Return(expr) => {
                if f.alternate() {
                    write!(f, "{}return ", idt(i))?;
                    expr.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}return ", idt(i))?;
                    expr.fmt_display_indent(f, 0)
                }
            }

            Self::Break(expr) => {
                if f.alternate() {
                    write!(f, "{}break ", idt(i))?;
                    expr.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}break ", idt(i))?;
                    expr.fmt_display_indent(f, 0)
                }
            }
            Self::Continue => {
                writeln!(f, "{}continue", idt(i))
            }
            Self::Range(range, step) => {
                write!(f, "{}{}..{}", idt(i), range.start, range.end)?;
                if *step != 1 {
                    write!(f, ":{step}")?;
                }
                Ok(())
            }

            Self::Chain(base, calls) => {
                write!(f, "{}", idt(i))?;
                base.fmt_display_indent(f, 0)?;
                for call in calls {
                    write!(f, ".{}(", call.method)?;
                    for (i, arg) in call.args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        arg.fmt_display_indent(f, 0)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }

            Self::PipeMethod(method, args) => {
                write!(f, "{}.{}(", idt(i), method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.fmt_display_indent(f, 0)?;
                }
                write!(f, ")")
            }

            Self::Catch(body, ctyp, deel) => {
                write!(f, "{}", idt(i))?;
                body.fmt_display_indent(f, 0)?;
                match ctyp {
                    CatchType::Ignore => write!(f, " ?."),
                    CatchType::PrintStd => write!(f, " ?+"),
                    CatchType::PrintErr => write!(f, " ??"),
                    CatchType::PrintOver => write!(f, " ?>"),
                    CatchType::Terminate => write!(f, " ?!"),
                    CatchType::ToBoolean => write!(f, " ?~"),
                    CatchType::Deel => {
                        write!(f, " ?: ")?;
                        if let Some(handler) = deel {
                            handler.fmt_display_indent(f, 0)?;
                        } else {
                            write!(f, "{{}}")?;
                        }
                        Ok(())
                    }
                }
            }

            Self::Use(name, path) => {
                write!(
                    f,
                    "{}use {} as {}",
                    idt(i),
                    path,
                    name.as_deref().unwrap_or("_")
                )
            }

            Self::Del(name) => write!(f, "{}del {}", idt(i), name),

            Self::AliasDef(name, cmd) => {
                if f.alternate() {
                    write!(f, "{}alias {} = ", idt(i), name)?;
                    cmd.fmt_display_indent(f, i + 1)
                } else {
                    write!(f, "{}alias {} = ", idt(i), name)?;
                    cmd.fmt_display_indent(f, 0)
                }
            }

            Self::RegexDef(s) => write!(f, "{}r'{s}'", idt(i)),
            Self::Regex(r) => write!(f, "{}r'{}'", idt(i), r.regex.as_str()),
            Self::TimeDef(t) => write!(f, "{}t'{t}'", idt(i)),
            Self::Blank => write!(f, "{}_", idt(i)),
            Self::Table(t) => write!(f, "{}{t:?}", idt(i)),
        }
    }
}

// Expression 辅助函数
impl Expression {
    fn fmt_indent(&self, f: &mut fmt::Formatter, indent: usize) -> fmt::Result {
        let prefix = idt(indent);
        match &self {
            // 基础类型 - 统一格式
            Self::Symbol(s) => write!(f, "{}Symbol〈{s:?}〉", prefix),
            Self::Variable(s) => write!(f, "{}Variable〈{s:?}〉", prefix),
            Self::String(s) => write!(f, "{}String〈{s:?}〉", prefix),
            Self::Integer(s) => write!(f, "{}Integer〈{s:?}〉", prefix),
            Self::Float(s) => write!(f, "{}Float〈{s:?}〉", prefix),
            Self::Boolean(s) => write!(f, "{}Boolean〈{s:?}〉", prefix),
            Self::DateTime(s) => write!(f, "{}DateTime〈{s:?}〉", prefix),
            Self::FileSize(s) => write!(f, "{}FileSize〈{s:?}〉", prefix),
            Self::Range(s, st) => write!(f, "{}Range〈{s:?}:{st}〉", prefix),
            Self::None => write!(f, "{}None", prefix),
            Self::Blank => write!(f, "{}_", prefix),

            // 字符串相关
            Self::StringTemplate(segments) => {
                write!(f, "{}StringTemplate〈`", prefix)?;
                for seg in segments.iter() {
                    match seg {
                        Self::String(s) => write!(f, "{s}")?,
                        Self::Variable(v) => write!(f, "${v}")?,
                        other => write!(f, "${{{other}}}")?,
                    }
                }
                write!(f, "`〉")
            }
            Self::Bytes(b) => write!(f, "{}Bytes〈{:?}〉", prefix, String::from_utf8_lossy(b)),
            Self::RegexDef(s) => write!(f, "{}RegexDef〈{s:?}〉", prefix),
            Self::Regex(s) => write!(f, "{}Regex〈{:?}〉", prefix, s.regex.as_str()),
            Self::TimeDef(s) => write!(f, "{}TimeDef〈{s:?}〉", prefix),

            // 复合表达式
            Self::Group(inner) => {
                write!(f, "{}Group\n{}(", prefix, idt(indent + 1))?;
                inner.fmt_indent(f, indent + 2)?;
                write!(f, "\n{})", idt(indent + 1))
            }

            Self::Quote(inner) => write!(f, "{}Quote〈{:?}〉", prefix, inner),

            // 声明和赋值
            Self::Declare(name, expr) => {
                write!(f, "{}Declare〈{}〉 = ", prefix, name)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::DestructureAssign(pattern, expr) => {
                write!(f, "{}DestructureAssign〈{:?}〉 = ", prefix, pattern)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::Assign(name, expr) => {
                write!(f, "{}Assign〈{}〉 = ", prefix, name)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::SetParent(name, expr) => {
                write!(f, "{}Set〈{}〉 = ", prefix, name)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::Export(name, expr) => {
                write!(f, "{}Export〈{}〉", prefix, name)?;
                if let Some(exp) = expr {
                    write!(f, " = ")?;
                    exp.fmt_indent(f, indent + 1)?;
                }
                Ok(())
            }

            // 控制流
            Self::If(cond, true_expr, false_expr) => {
                writeln!(f, "{}If", prefix)?;
                cond.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Then", idt(indent + 1))?;
                true_expr.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Else", idt(indent + 1))?;
                false_expr.fmt_indent(f, indent + 1)
            }
            Self::While(cond, body) => {
                writeln!(f, "{}While", prefix)?;
                cond.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Body", idt(indent + 1))?;
                body.fmt_indent(f, indent + 1)
            }
            Self::Loop(body) => {
                writeln!(f, "{}Loop", prefix)?;
                body.fmt_indent(f, indent + 1)
            }
            Self::For(name, ind, list, body) => {
                if let Some(index) = ind {
                    writeln!(f, "{}For〈{},{}〉", prefix, index, name)?;
                } else {
                    writeln!(f, "{}For〈{}〉", prefix, name)?;
                }
                list.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Body", idt(indent + 1))?;
                body.fmt_indent(f, indent + 1)
            }
            Self::Match(value, branches) => {
                writeln!(f, "{}Match", prefix)?;
                value.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Branches", idt(indent + 1))?;
                for (pat, expr) in branches.iter() {
                    writeln!(
                        f,
                        "{}{} =>",
                        idt(indent + 2),
                        pat.iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    )?;
                    expr.fmt_indent(f, indent + 3)?;
                }
                Ok(())
            }

            // 函数相关
            Self::Lambda(params, body, _) => {
                writeln!(f, "{}Lambda〈{}〉", prefix, params.join(", "))?;
                body.fmt_indent(f, indent + 1)
            }
            Self::Function(name, params, collector, body, decorators) => {
                // 装饰器
                for (deco, args) in decorators {
                    writeln!(
                        f,
                        "{}@{}({})",
                        prefix,
                        deco,
                        match args {
                            Some(a) => a
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            _ => String::new(),
                        }
                    )?;
                }

                let collector_str = match collector {
                    Some(x) => format!(",*{}", x),
                    _ => String::new(),
                };

                writeln!(
                    f,
                    "{}Function〈{}({}{})〉",
                    prefix,
                    name,
                    params
                        .iter()
                        .map(|(p, v)| match v {
                            Some(vv) => format!("{p}={vv}"),
                            _ => p.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                    collector_str
                )?;
                body.fmt_indent(f, indent + 1)
            }

            // 集合类型
            Self::List(exprs) => {
                writeln!(f, "{}List", prefix)?;
                for expr in exprs.iter() {
                    expr.fmt_indent(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                Ok(())
            }
            Self::BSet(exprs) => {
                writeln!(f, "{}Set", prefix)?;
                for expr in exprs.iter() {
                    expr.fmt_indent(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                Ok(())
            }
            Self::Map(exprs) => {
                writeln!(f, "{}Map", prefix)?;
                for (k, v) in exprs.iter() {
                    writeln!(f, "{}{}:", idt(indent + 1), k)?;
                    v.fmt_indent(f, indent + 2)?;
                    writeln!(f)?;
                }
                Ok(())
            }
            Self::HMap(exprs) => {
                writeln!(f, "{}HMap", prefix)?;
                for (k, v) in exprs.iter() {
                    writeln!(f, "{}{}:", idt(indent + 1), k)?;
                    v.fmt_indent(f, indent + 2)?;
                    writeln!(f)?;
                }
                Ok(())
            }
            Self::Block(exprs) => {
                writeln!(f, "{}Block", prefix)?;
                for expr in exprs.iter() {
                    expr.fmt_indent(f, indent + 1)?;
                    writeln!(f)?;
                }
                Ok(())
            }
            Self::Sequence(exprs) => {
                writeln!(f, "{}Sequence", prefix)?;
                for expr in exprs.iter() {
                    expr.fmt_indent(f, indent + 1)?;
                    writeln!(f)?;
                }
                Ok(())
            }

            // 操作符
            Self::BinaryOp(op, l, r) => {
                writeln!(f, "{}BinaryOp〈{}〉", prefix, op)?;
                l.fmt_indent(f, indent + 1)?;
                writeln!(f)?;
                r.fmt_indent(f, indent + 1)
            }
            Self::UnaryOp(op, expr, is_prefix) => {
                writeln!(f, "{}UnaryOp〈{}, prefix:{}〉", prefix, op, is_prefix)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::RangeOp(op, l, r, step) => {
                writeln!(f, "{}RangeOp〈{}〉", prefix, op)?;
                l.fmt_indent(f, indent + 1)?;
                writeln!(f)?;
                r.fmt_indent(f, indent + 1)?;
                if let Some(step_expr) = step {
                    write!(f, "\n{}Step", idt(indent + 1))?;
                    step_expr.fmt_indent(f, indent + 2)?;
                }
                Ok(())
            }
            Self::Pipe(op, l, r) => {
                writeln!(f, "{}Pipe〈{}〉", prefix, op)?;
                l.fmt_indent(f, indent + 1)?;
                writeln!(f)?;
                r.fmt_indent(f, indent + 1)
            }

            // 函数调用
            Self::Apply(func, args) => {
                writeln!(f, "{}Apply", prefix)?;
                func.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Args", idt(indent + 1))?;
                for arg in args.iter() {
                    arg.fmt_indent(f, indent + 2)?;
                    writeln!(f)?;
                }
                Ok(())
            }
            Self::Command(cmd, args) | Self::CommandRaw(cmd, args) => {
                writeln!(f, "{}Command", prefix)?;
                cmd.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}Args", idt(indent + 1))?;
                for arg in args.iter() {
                    arg.fmt_indent(f, indent + 2)?;
                    writeln!(f)?;
                }
                Ok(())
            }

            // 索引和属性
            Self::Index(obj, index) => {
                writeln!(f, "{}Index", prefix)?;
                obj.fmt_indent(f, indent + 1)?;
                write!(f, "\n{}[", idt(indent + 1))?;
                index.fmt_indent(f, indent + 2)?;
                write!(f, "\n{}]", idt(indent + 1))
            }
            Self::Property(obj, prop) => {
                writeln!(f, "{}Property", prefix)?;
                obj.fmt_indent(f, indent + 1)?;
                write!(f, ".{}", prop)
            }

            // 链式调用
            Self::Chain(base, calls) => {
                writeln!(f, "{}Chain", prefix)?;
                base.fmt_indent(f, indent + 1)?;
                for call in calls {
                    write!(f, "\n{}.{}(", idt(indent + 1), call.method)?;
                    for (i, arg) in call.args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg:?}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Self::PipeMethod(method, args) => {
                writeln!(f, "{}PipeMethod〈{}〉", prefix, method)?;
                for arg in args.iter() {
                    arg.fmt_indent(f, indent + 1)?;
                    writeln!(f)?;
                }
                Ok(())
            }

            // 其他
            Self::Return(expr) => {
                writeln!(f, "{}Return", prefix)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::Break(expr) => {
                writeln!(f, "{}Break", prefix)?;
                expr.fmt_indent(f, indent + 1)
            }
            Self::Continue => {
                writeln!(f, "{}Continue", prefix)
            }
            Self::Catch(body, ctyp, deel) => {
                writeln!(f, "{}Catch〈{:?}〉", prefix, ctyp)?;
                body.fmt_indent(f, indent + 1)?;
                if let Some(handler) = deel {
                    write!(f, "\n{}Handler", idt(indent + 1))?;
                    handler.fmt_indent(f, indent + 2)?;
                }
                Ok(())
            }
            Self::AliasDef(name, cmd) => {
                writeln!(f, "{}AliasDef〈{}〉", prefix, name)?;
                cmd.fmt_indent(f, indent + 1)
            }
            Self::Use(name, path) => {
                writeln!(
                    f,
                    "{}Use〈{} as {}〉",
                    prefix,
                    path,
                    name.as_ref().map_or("_", |n| n.as_str())
                )?;
                Ok(())
            }
            Self::Del(name) => write!(f, "{}Del〈{}〉", prefix, name),
            Self::ModuleCall(mo, func) => {
                write!(f, "{}ModuleCall〈{}::{}〉", prefix, mo.join("::"), func)
            }
            Self::Table(t) => write!(f, "{prefix}Table〈\n{t:#}\n{prefix}〉"),
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Self::List(_) => "List".into(),
            Self::BSet(_) => "Set".into(),
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
            Self::Property(_, _) => "Property".into(),
            Self::Del(_) => "Del".into(),
            Self::Declare(_, _) => "Declare".into(),
            Self::SetParent(_, _) => "Set".into(),
            Self::Export(_, _) => "Export".into(),
            Self::Assign(_, _) => "Assign".into(),
            Self::For(..) => "For".into(),
            Self::While(_, _) => "While".into(),
            Self::Loop(_) => "Loop".into(),
            Self::Match(_, _) => "Match".into(),
            Self::If(_, _, _) => "If".into(),
            Self::Apply(_, _) => "Apply".into(),
            Self::Command(_, _) => "Command".into(),
            Self::CommandRaw(_, _) => "CommandRaw".into(),
            Self::ModuleCall(_, _) => "ModuleCall".into(),
            Self::Lambda(..) => "Lambda".into(),
            // Self::Macro(_, _) => "Macro".into(),
            Self::Function(..) => "Function".into(),
            Self::Return(_) => "Return".into(),
            Self::Break(_) => "Break".into(),
            Self::Continue => "Continue".into(),
            Self::Block(_) => "Block".into(),
            Self::Sequence(_) => "Sequence".into(),
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
            Self::Blank => "Blank".into(),
            Self::Table(_) => "Table".into(),
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
                    expected: "Symbol".into(),
                    sym: self.to_string(),
                    found: self.type_name(),
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
    // pub fn replace_or_append_arg(&self, arg: Expression) -> Expression {
    //     let mut found = false;
    //     match self {
    //         Expression::Apply(f, existing_args) => {
    //             let new_args = existing_args
    //                 .iter()
    //                 .map(|a| match a {
    //                     Self::Blank => {
    //                         found = true;
    //                         arg.clone()
    //                     }
    //                     _ => a.clone(),
    //                 })
    //                 .collect();
    //             if found {
    //                 Expression::Apply(f.clone(), Rc::new(new_args))
    //             } else {
    //                 self.append_args(vec![arg])
    //             }
    //         }
    //         Expression::Command(f, existing_args) => {
    //             let new_args = existing_args
    //                 .iter()
    //                 .map(|a| match a {
    //                     Self::Blank => {
    //                         found = true;
    //                         arg.clone()
    //                     }
    //                     _ => a.clone(),
    //                 })
    //                 .collect();
    //             if found {
    //                 Expression::Command(f.clone(), Rc::new(new_args))
    //             } else {
    //                 self.append_args(vec![arg])
    //             }
    //         }
    //         Expression::Chain(base, calls) => {
    //             if calls.is_empty() {
    //                 Expression::Chain(base.clone(), calls.clone())
    //             } else {
    //                 let (call, others) = calls.split_at(1);
    //                 let mut new_args: Vec<Expression> = call[0]
    //                     .args
    //                     .iter()
    //                     .map(|a| match a {
    //                         Self::Blank => {
    //                             found = true;
    //                             arg.clone()
    //                         }
    //                         _ => a.clone(),
    //                     })
    //                     .collect();
    //                 if !found {
    //                     new_args.push(arg);
    //                 }
    //                 let mut new_calls = vec![ChainCall {
    //                     method: call[0].method.clone(),
    //                     args: new_args,
    //                 }];
    //                 new_calls.extend_from_slice(others);
    //                 Expression::Chain(base.clone(), new_calls)
    //             }
    //         }
    //         _ => Expression::Command(Rc::new(self.clone()), Rc::new(vec![arg])), //report error?
    //     }
    // }
    /// please make sure only use with Apply/Command
    #[inline]
    pub fn append_args(&self, args: &[Expression]) -> Expression {
        match self {
            Expression::Apply(f, existing_args) => {
                let mut new_vec = Vec::with_capacity(existing_args.len() + args.len());
                new_vec.extend_from_slice(existing_args);
                new_vec.extend_from_slice(args);
                Expression::Apply(f.clone(), Rc::new(new_vec))
            }
            Expression::Command(f, existing_args) => {
                let mut new_vec = Vec::with_capacity(existing_args.len() + args.len());
                new_vec.extend_from_slice(existing_args);
                new_vec.extend_from_slice(args);
                Expression::Command(f.clone(), Rc::new(new_vec))
            }
            Expression::Chain(base, calls) => {
                if calls.is_empty() {
                    Expression::Chain(base.clone(), calls.clone())
                } else {
                    let (call, others) = calls.split_at(1);

                    let mut new_vec = Vec::with_capacity(call[0].args.len() + args.len());
                    new_vec.extend_from_slice(&call[0].args);
                    new_vec.extend_from_slice(args);
                    let mut new_calls = vec![ChainCall {
                        method: call[0].method.clone(),
                        args: new_vec,
                    }];
                    new_calls.extend_from_slice(others);
                    Expression::Chain(base.clone(), new_calls)
                }
            }
            _ => unreachable!(), // _ => Expression::Command(Rc::new(self.clone()), Rc::new(args)), //report error?
        }
    }
    // /// please make sure only use with Apply/Command
    // pub fn inject_arg(&self, arg: Expression) -> Expression {
    //     match self {
    //         //for func: add default receiver to at head if not exist
    //         Expression::Apply(f, existing_args) => {

    //                 let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
    //                 new_vec.push(arg);
    //                 new_vec.extend_from_slice(existing_args);
    //                 Expression::Apply(f.clone(), Rc::new(new_vec))

    //         }
    //         // for cmd: never add, default is pipeout to stdio
    //         // only accept if user request
    //         // Expression::Command(f, existing_args) => {
    //         //     Cow::Borrowed(self)
    //         // if existing_args.iter().any(|a| a == &Expression::Blank) {
    //         // } else {
    //         //     let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
    //         //     new_vec.push(Expression::Blank);
    //         //     new_vec.extend_from_slice(existing_args);
    //         //     Cow::Owned(Expression::Command(f.clone(), Rc::new(new_vec)))
    //         // }
    //         // }
    //         // for chain: only add to head of first call if user not request.
    //         Expression::Chain(base, calls) => {
    //             if calls.is_empty() || calls[0].args.contains(&Expression::Blank) {
    //                 Cow::Borrowed(self)
    //             } else {
    //                 let (call, others) = calls.split_at(1);

    //                 let mut new_vec = Vec::with_capacity(call[0].args.len() + 1);
    //                 new_vec.push(Expression::Blank);
    //                 new_vec.extend_from_slice(&call[0].args);

    //                 let mut new_calls = Vec::with_capacity(calls.len());
    //                 new_calls.push(ChainCall {
    //                     method: call[0].method.clone(),
    //                     args: new_vec,
    //                 });
    //                 new_calls.extend_from_slice(others);
    //                 Cow::Owned(Expression::Chain(base.clone(), new_calls))
    //             }
    //         }
    //         _ => Cow::Borrowed(self), //others, like binop,group,pipe...
    //     }
    // }
    /// used in right of pipe.
    /// this ensure symbo as a cmd with blank receiver.
    #[inline]
    pub fn ensure_sym_as_cmd<'a>(&'a self) -> Cow<'a, Expression> {
        match self {
            // Expression::Function(..) | Expression::Lambda(..) => Cow::Owned(self.apply(vec![arg])),
            // property maybe a map property or chaincall, let user deside.
            Expression::Property(base, method) => Cow::Owned(Self::Chain(
                base.clone(),
                vec![ChainCall {
                    method: method.to_string(),
                    args: vec![Expression::Blank],
                }],
            )),
            // symbol maybe alias, but also maybe var/string, so let user decide.
            // Blank injection is handled by ensure_has_receiver for builtins only.
            Expression::Symbol(_) => {
                Cow::Owned(Expression::Command(Rc::new(self.clone()), Rc::new(vec![])))
            }
            _ => Cow::Borrowed(self), //others, like binop,group,pipe...
        }
    }
    #[inline]
    pub fn ensure_fn_apply<'a>(&'a self) -> Cow<'a, Expression> {
        match self {
            Expression::Function(..) | Expression::Lambda(..) => Cow::Owned(self.apply(vec![])),
            // property maybe a map property or chaincall, let user deside.
            // Expression::Property(base, method) => Cow::Owned(Self::Chain(
            //     base.clone(),
            //     vec![ChainCall {
            //         method: method.to_string(),
            //         args: vec![],
            //     }],
            // )),
            // symbol maybe alias, but also maybe var/string, so let user decide.
            // Expression::Symbol(_) => Expression::Command(Rc::new(self.clone()), Rc::new(vec![])),
            _ => Cow::Borrowed(self), //others, like binop,group,pipe...
        }
    }
    pub fn ensure_has_receiver<'a>(&'a self) -> Cow<'a, Expression> {
        match self {
            //for func: add default receiver to at head if not exist
            Expression::Apply(f, existing_args) => {
                if existing_args.iter().any(|a| a == &Expression::Blank) {
                    Cow::Borrowed(self)
                } else {
                    let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
                    new_vec.push(Expression::Blank);
                    new_vec.extend_from_slice(existing_args);
                    Cow::Owned(Expression::Apply(f.clone(), Rc::new(new_vec)))
                }
            }
            // for cmd: never add, default is pipeout to stdio
            // Except lib func like: ui.pick
            // cmd without args, goes to: ensure_sym_as_cmd
            // only accept if user request
            Expression::Command(f, existing_args) => {
                if existing_args.iter().any(|a| a == &Expression::Blank) {
                    Cow::Borrowed(self)
                } else {
                    match f.as_ref() {
                        // like 'ui.pick args'
                        Expression::Property(base, _) => {
                            if let Expression::Symbol(name) = base.as_ref()
                                && is_lib(name)
                            {
                                let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
                                new_vec.push(Expression::Blank);
                                new_vec.extend_from_slice(existing_args);
                                Cow::Owned(Expression::Command(f.clone(), Rc::new(new_vec)))
                            } else {
                                Cow::Borrowed(self)
                            }
                        }
                        // like 'print args'
                        Expression::Symbol(sym) if is_top_or_se(sym) => {
                            let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
                            new_vec.push(Expression::Blank);
                            new_vec.extend_from_slice(existing_args);
                            Cow::Owned(Expression::Command(f.clone(), Rc::new(new_vec)))
                        }
                        _ => Cow::Borrowed(self),
                    }
                }
            }
            // for chain: only add to head of first call if user not request.
            Expression::Chain(base, calls) => {
                if calls.is_empty() || calls[0].args.contains(&Expression::Blank) {
                    Cow::Borrowed(self)
                } else {
                    let (call, others) = calls.split_at(1);

                    let mut new_vec = Vec::with_capacity(call[0].args.len() + 1);
                    new_vec.push(Expression::Blank);
                    new_vec.extend_from_slice(&call[0].args);

                    let mut new_calls = Vec::with_capacity(calls.len());
                    new_calls.push(ChainCall {
                        method: call[0].method.clone(),
                        args: new_vec,
                    });
                    new_calls.extend_from_slice(others);
                    Cow::Owned(Expression::Chain(base.clone(), new_calls))
                }
            }
            // for catch: inject receiver into body and handler
            Expression::Catch(body, ctyp, handler) => {
                let body = body.ensure_has_receiver();

                match body {
                    Cow::Borrowed(_) => Cow::Borrowed(self),
                    body => Cow::Owned(Expression::Catch(
                        Rc::new(body.into_owned()),
                        ctyp.clone(),
                        handler.clone(),
                    )),
                }
            }
            _ => Cow::Borrowed(self), //others, like binop,group,pipe...
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
            Self::List(exprs) => !exprs.is_empty(),
            Self::BSet(exprs) => !exprs.is_empty(),
            Self::HMap(exprs) => !exprs.is_empty(),
            Self::Map(exprs) => !exprs.is_empty(),
            Self::Range(exprs, _) => !exprs.is_empty(),
            Self::Regex(r) => !r.regex.as_str().is_empty(),
            Self::Lambda(..) => true,
            Self::Function(..) => true,
            Self::DateTime(..) => true,
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
