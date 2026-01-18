use super::{CatchType, Expression};
use crate::expression::{ChainCall, DestructurePattern};
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
            // 基础类型 - 无需缩进
            Self::Symbol(name) => write!(f, "{name}"),
            Self::Variable(name) => write!(f, "${name}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
            Self::StringTemplate(s) => write!(f, "`{s}`"),
            Self::Boolean(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Self::Bytes(b) => write!(f, "b\"{}\"", String::from_utf8_lossy(b)),
            Self::DateTime(n) => write!(f, "{}", n.format("%Y-%m-%d %H:%M:%S")),
            Self::FileSize(fsz) => write!(f, "{}", fsz.to_human_readable()),
            Self::None => Ok(()),

            // 声明和赋值
            Self::Declare(name, expr) => {
                write!(f, "{}let {} = ", idt(i), name)?;
                expr.fmt_display_indent(f, 0)
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
                expr.fmt_display_indent(f, 0)
            }
            Self::Assign(name, expr) => {
                write!(f, "{}{} = ", idt(i), name)?;
                expr.fmt_display_indent(f, 0)
            }

            // 引用和分组
            Self::Quote(inner) => {
                write!(f, "'")?;
                inner.fmt_display_indent(f, 0)
            }
            Self::Group(inner) => {
                write!(f, "{}(", idt(i))?;
                inner.fmt_display_indent(f, i + 1)?;
                write!(f, "{})", idt(i))
            }

            // 控制流 - 使用缩进
            Self::If(cond, true_expr, false_expr) => {
                write!(f, "{}if ", idt(i))?;
                cond.fmt_display_indent(f, 0)?;
                writeln!(f, " {{")?;
                true_expr.fmt_display_indent(f, i + 1)?;
                write!(f, "\n{}}} else {{\n", idt(i))?;
                false_expr.fmt_display_indent(f, i + 1)?;
                write!(f, "\n{}}}", idt(i))
            }

            Self::While(cond, body) => {
                write!(f, "{}while ", idt(i))?;
                cond.fmt_display_indent(f, 0)?;
                writeln!(f, " {{")?;
                body.fmt_display_indent(f, i + 1)?;
                write!(f, "\n{}}}", idt(i))
            }

            Self::Loop(body) => {
                writeln!(f, "{}loop {{", idt(i))?;
                body.fmt_display_indent(f, i + 1)?;
                write!(f, "\n{}}}", idt(i))
            }

            Self::For(name, list, body) => {
                write!(f, "{}for {} in ", idt(i), name)?;
                list.fmt_display_indent(f, 0)?;
                writeln!(f, " {{")?;
                body.fmt_display_indent(f, i + 1)?;
                write!(f, "\n{}}}", idt(i))
            }

            Self::Match(value, branches) => {
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
            }

            // 函数定义
            Self::Lambda(params, body) => {
                write!(f, "{}({}) -> ", idt(i), params.join(", "))?;
                if matches!(body.as_ref(), Self::Do(_)) {
                    writeln!(f)?;
                    body.fmt_display_indent(f, i + 1)
                } else {
                    body.fmt_display_indent(f, 0)
                }
            }

            Self::Function(name, params, collector, body, _) => {
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
            }

            // 代码块
            Self::Do(exprs) => {
                writeln!(f, "{}{{", idt(i))?;
                for expr in exprs.iter() {
                    expr.fmt_display_indent(f, i + 1)?;
                    writeln!(f)?;
                }
                write!(f, "{}}}", idt(i))
            }

            // 集合类型 - 紧凑格式
            Self::List(exprs) => {
                write!(f, "[")?;
                for (i, expr) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    expr.fmt_display_indent(f, 0)?;
                }
                write!(f, "]")
            }

            Self::Map(exprs) => {
                write!(f, "{}{{", idt(i))?;
                for (i, (k, v)) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: ")?;
                    v.fmt_display_indent(f, 0)?;
                }
                write!(f, "{}}}", idt(i))
            }
            Self::HMap(exprs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: ")?;
                    v.fmt_display_indent(f, 0)?;
                }
                write!(f, "}}")
            }

            // 操作符
            Self::BinaryOp(op, l, r) => {
                write!(f, "{l} {op} {r}")
            }

            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!(f, "{op}{v}")
                } else {
                    write!(f, "{v}{op}")
                }
            }

            Self::RangeOp(op, l, r, step) => {
                write!(f, "{l}{op}{r}")?;
                if let Some(st) = step {
                    write!(f, ":{st}")?;
                }
                Ok(())
            }

            Self::Pipe(op, l, r) => {
                write!(f, "{l} {op} {r}")
            }

            // 函数调用
            Self::Apply(func, args) => {
                write!(
                    f,
                    "\n{}{func}({})",
                    idt(i),
                    args.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }

            Self::Command(cmd, args) | Self::CommandRaw(cmd, args) => {
                write!(
                    f,
                    "\n{}{cmd} {}",
                    idt(i),
                    args.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
            Self::ModuleCall(mo, func) => {
                write!(f, "{}::{}", mo.join("::"), func)
            }
            // 索引和切片
            Self::Index(l, r) => {
                l.fmt_display_indent(f, 0)?;
                write!(f, "[")?;
                r.fmt_display_indent(f, 0)?;
                write!(f, "]")
            }

            // 其他构造
            Self::Return(expr) => {
                write!(f, "{}return ", idt(i))?;
                expr.fmt_display_indent(f, 0)
            }

            Self::Break(expr) => {
                write!(f, "{}break ", idt(i))?;
                expr.fmt_display_indent(f, 0)
            }

            Self::Range(range, step) => {
                write!(f, "{}..{}", range.start, range.end)?;
                if *step != 1 {
                    write!(f, ":{step}")?;
                }
                Ok(())
            }

            Self::Chain(base, calls) => {
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
                write!(f, ".{method}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.fmt_display_indent(f, 0)?;
                }
                write!(f, ")")
            }

            Self::Catch(body, ctyp, deel) => {
                body.fmt_display_indent(f, 0)?;
                match ctyp {
                    CatchType::Ignore => write!(f, " ?."),
                    CatchType::PrintStd => write!(f, " ?+"),
                    CatchType::PrintErr => write!(f, " ??"),
                    CatchType::PrintOver => write!(f, " ?>"),
                    CatchType::Terminate => write!(f, " ?!"),
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
                write!(f, "{}alias {} = ", idt(i), name)?;
                cmd.fmt_display_indent(f, 0)
            }

            Self::RegexDef(s) => write!(f, "r'{s}'"),
            Self::Regex(r) => write!(f, "r'{}'", r.regex.as_str()),
            Self::TimeDef(t) => write!(f, "t'{t}'"),
            Self::Blank => write!(f, "_"),
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
    writeln!(f)?;
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
            Self::Symbol(s) => write!(f, "Symbol〈{s:?}〉"),
            Self::Variable(s) => write!(f, "Variable〈{s:?}〉"),
            Self::String(s) => write!(f, "String〈{s:?}〉"),
            Self::Integer(s) => write!(f, "Integer〈{s:?}〉"),
            Self::Float(s) => write!(f, "Float〈{s:?}〉"),
            Self::Boolean(s) => write!(f, "Boolean〈{s:?}〉"),
            Self::DateTime(s) => write!(f, "DateTime〈{s:?}〉"),
            Self::FileSize(s) => write!(f, "FileSize〈{s:?}〉"),
            Self::Range(s, st) => write!(f, "Range〈{s:?}:{st}〉"),
            Self::Quote(inner) => write!(f, "Quote〈{inner:?}〉"),
            Self::TimeDef(s) => write!(f, "TimeDef〈{s:?}〉"),
            Self::RegexDef(s) => write!(f, "RegexDef〈{s:?}〉"),
            Self::Regex(s) => write!(f, "Regex〈{:?}〉", s.regex.as_str()),
            Self::None => write!(f, "None"),
            Self::Blank => write!(f, "_"),
            Self::ModuleCall(s, func) => write!(f, "ModuleCall〈{}::{}〉", s.join("::"), func),

            // 新增：字符串模板和字节数据
            Self::StringTemplate(s) => write!(f, "StringTemplate〈`{s}`〉"),
            Self::Bytes(b) => write!(f, "Bytes〈{:?}〉", String::from_utf8_lossy(b)),

            Self::Group(inner) => {
                write!(f, "\n{}Group\n{}(", idt(i), idt(i + 1))?;
                inner.fmt_indent(f, i + 2)?;
                write!(f, "\n{})", idt(i + 1))
            }

            // 新增：声明和赋值操作
            Self::Declare(name, expr) => {
                write!(f, "\n{}Declare〈{}〉 =", idt(i), name)?;
                expr.fmt_indent(f, i + 1)
            }
            Self::DestructureAssign(pattern, expr) => {
                write!(f, "\n{}DestructureAssign〈{:?}〉 =", idt(i), pattern)?;
                expr.fmt_indent(f, i + 1)
            }
            Self::Assign(name, expr) => {
                write!(f, "\n{}Assign〈{}〉 =", idt(i), name)?;
                expr.fmt_indent(f, i + 1)
            }

            // 新增：删除和控制流语句
            Self::Del(name) => write!(f, "Del〈{name}〉"),
            Self::Return(expr) => {
                write!(f, "\n{}Return", idt(i))?;
                expr.fmt_indent(f, i + 1)
            }
            Self::Break(expr) => {
                write!(f, "\n{}Break", idt(i))?;
                expr.fmt_indent(f, i + 1)
            }

            // 新增：操作符
            Self::UnaryOp(op, expr, is_prefix) => {
                write!(f, "\n{}UnaryOp〈{}, prefix:{}〉", idt(i), op, is_prefix)?;
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
                        write!(f, "{arg:?}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Self::PipeMethod(method, args) => {
                write!(f, "\n{}PipeMethod〈{}〉\n{}(\n", idt(i), method, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                writeln!(f, "{})", idt(i))
            }

            // 新增：别名操作
            Self::AliasDef(name, cmd) => {
                write!(f, "\n{}AliasDef〈{}〉 =\n", idt(i), name)?;
                cmd.fmt_indent(f, i + 1)
            }

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
                    let _ = writeln!(f, ",");
                });
                writeln!(f, "{}]", idt(i))
            }
            Self::HMap(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|(k, v)| {
                    let _ = write!(f, "\n{}{:?}:", idt(i + 1), k);
                    let _ = v.fmt_indent(f, i + 2);
                    let _ = writeln!(f);
                });
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Map(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|(k, v)| {
                    let _ = write!(f, "\n{}{:?}:", idt(i + 1), k);
                    let _ = v.fmt_indent(f, i + 2);
                    let _ = writeln!(f);
                });
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Do(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                writeln!(f, "{}}}", idt(i))
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
                write!(f, "\n{}match ", idt(i))?;
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
                write!(f, "\n{}for {} in ", idt(i), name)?;
                list.fmt_indent(f, i + 1)?;
                write!(f, "\n{}{{\n", idt(i))?;
                body.fmt_indent(f, i + 1)?;
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::While(cond, body) => {
                write!(f, "\n{}while", idt(i))?;
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
                write!(f, "\n{}Lambda ({}) ->", idt(i), params.to_vec().join(","))?;
                body.as_ref().fmt_indent(f, i + 1)
            }
            Self::Function(name, param, pc, body, decos) => {
                for (deco, args) in decos {
                    write!(
                        f,
                        "\n{}@{}({})",
                        idt(i),
                        deco,
                        match args {
                            Some(a) => Cow::Owned(
                                a.iter()
                                    .map(|x| x.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            ),
                            _ => Cow::Borrowed(""),
                        }
                    )?;
                }

                let collector = match pc {
                    Some(x) => Cow::Owned(format!(",*{}", x)),
                    _ => Cow::Borrowed(""),
                };
                write!(
                    f,
                    "\n{}fn {}({}{collector})\n",
                    idt(i),
                    name,
                    param
                        .iter()
                        .map(|(p, v)| match v {
                            Some(vv) => format!("{p}={vv}"),
                            _ => p.to_string(),
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                )?;
                body.fmt_indent(f, i + 1)
            }
            Self::Apply(func, args) => {
                write!(f, "\n{}Apply〈{}〉\n{}(\n", idt(i), func, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                writeln!(f, "{})", idt(i))
            }
            Self::Command(cmd, args) | Self::CommandRaw(cmd, args) => {
                write!(f, "\n{}Command〈{}〉\n{}(\n", idt(i), cmd, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                writeln!(f, "{})", idt(i))
            }
        }
    }

    /// 类型名称
    pub fn get_belong_lib_name(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::List(_) | Self::Range(..) => Some("List".into()),
            Self::Map(_) | Self::HMap(_) => Some("Map".into()),
            Self::String(_) | Self::StringTemplate(_) | Self::Bytes(_) => Some("String".into()),
            Self::Integer(_) | Self::Float(_) => Some("Math".into()),
            Self::DateTime(_) => Some("Time".into()),
            Self::Boolean(_) => Some("Boolean".into()),
            Self::Regex(_) => Some("Regex".into()),
            Self::FileSize(_) => Some("Filesize".into()),
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
            // Self::Slice(_, _) => "Slice".into(),
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
            Self::CommandRaw(_, _) => "CommandRaw".into(),
            Self::ModuleCall(_, _) => "ModuleCall".into(),
            Self::Lambda(..) => "Lambda".into(),
            // Self::Macro(_, _) => "Macro".into(),
            Self::Function(..) => "Function".into(),
            Self::Return(_) => "Return".into(),
            Self::Break(_) => "Break".into(),
            Self::Do(_) => "Do".into(),
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
                    Expression::Chain(base.clone(), calls.clone())
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
                    Expression::Chain(base.clone(), new_calls)
                }
            }
            _ => unreachable!(), // _ => Expression::Command(Rc::new(self.clone()), Rc::new(args)), //report error?
        }
    }
    /// please make sure only use with Apply/Command
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

    pub fn ensure_fn_apply<'a>(&'a self) -> Cow<'a, Expression> {
        match self {
            Expression::Function(..) | Expression::Lambda(..) => Cow::Owned(self.apply(vec![])),
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
            // only accept if user request
            // Expression::Command(f, existing_args) => {
            //     Cow::Borrowed(self)
            // if existing_args.iter().any(|a| a == &Expression::Blank) {
            // } else {
            //     let mut new_vec = Vec::with_capacity(existing_args.len() + 1);
            //     new_vec.push(Expression::Blank);
            //     new_vec.extend_from_slice(existing_args);
            //     Cow::Owned(Expression::Command(f.clone(), Rc::new(new_vec)))
            // }
            // }
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
            Self::List(exprs) => !exprs.as_ref().is_empty(),
            Self::HMap(exprs) => !exprs.as_ref().is_empty(),
            Self::Map(exprs) => !exprs.as_ref().is_empty(),
            Self::Range(exprs, _) => !exprs.is_empty(),
            Self::Lambda(..) => true,
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
