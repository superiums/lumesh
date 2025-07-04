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
// 宏定义（可放在 impl 块外）
macro_rules! fmt_shared {
    ($self:ident, $f:ident, $debug:expr) => {
        match $self {
            Self::Symbol(name) => write!($f, "{}", name),
            Self::Variable(name) => write!($f, "${}", name),

            Self::FileSize(fsz) => write!($f, "{}", fsz.to_human_readable()),

            // Self::String(s) if $debug => write!($f, "{:?}", s),
            Self::String(s) => write!($f, "{}", s),
            Self::StringTemplate(s) => write!($f, "`{}`", s),

            Self::Integer(i) => write!($f, "{}", *i),
            Self::Float(n) => write!($f, "{}", *n),
            Self::Bytes(b) => write!($f, "b{}", String::from_utf8_lossy(b)),
            Self::Boolean(b) => write!($f, "{}", if *b { "True" } else { "False" }),
            Self::DateTime(n) => write!($f, "{}", n.format("%Y-%m-%d %H:%M")),

            Self::Declare(name, expr) => write!($f, "let {} = {}", name, expr),
            Self::DestructureAssign(name, expr) => write!($f, "let {:?} = {}", name, expr),
            Self::Assign(name, expr) => write!($f, "{} = {}", name, expr),

            // Quote 修改
            Self::Quote(inner) => write!($f, "'{}", inner),

            // Group 修改
            Self::Group(inner) => write!($f, "({})", inner),

            // While 修改
            Self::While(cond, body) => write!($f, "while {}\n\t{}", cond, body),
            Self::Loop(inner) => write!($f, "loop {}", inner),

            // Lambda 修改
            Self::Lambda(params, body) => write!($f, "({}) -> {}", params.join(", "), body),
            // Self::Macro(params, body) if $debug => write!($f, "{:?} ~> {:?}", params, body),
            // Self::Macro(params, body) => write!($f, "({}) ~> {}", params.join(", "), body),

            // If 修改
            Self::If(cond, true_expr, false_expr) => {
                write!($f, "if {}\n\t{}\nelse\n\t{}", cond, true_expr, false_expr)
            }

            Self::Slice(l, r) => write!(
                $f,
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

            // 修正List分支中的变量名错误
            Self::List(exprs) => {
                write!(
                    $f,
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
                    $f,
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
                    $f,
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
            Self::Chain(t, c) => write!($f, "{}.{:?}", t, c),
            Self::PipeMethod(t, a) => write!($f, ".{}({:?})", t, a),
            Self::Function(name, param, pc, body, _) => match pc {
                Some(collector) => write!(
                    $f,
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
                    $f,
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
            Self::Return(body) => write!($f, "return {}", body),
            Self::Break(body) => write!($f, "break {}", body),
            Self::For(name, list, body) => write!($f, "for {} in {}\n\t{}", name, list, body),
            Self::Do(exprs) => write!(
                $f,
                "{{\n\t{}\n\t}}",
                exprs
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(";\n")
            ),

            Self::Del(name) => write!($f, "del {}", name),

            Self::Match(value, branches) => {
                write!($f, "match {} {{", value)?;
                for (pat, expr) in branches.iter() {
                    write!(
                        $f,
                        "\n\t{} => {}, ",
                        pat.iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        expr
                    )?;
                }
                write!($f, "\n}}")
            }

            Self::Apply(g, args) => write!(
                $f,
                "{}({})",
                g,
                args.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::Command(g, args) => write!(
                $f,
                "{}  {}",
                g,
                args.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),

            Self::AliasOp(name, cmd) => write!($f, "alias {} = {}", name, cmd),
            Self::UnaryOp(op, v, is_prefix) => {
                if *is_prefix {
                    write!($f, "({} {})", op, v)
                } else {
                    write!($f, "({} {})", v, op)
                }
            }
            Self::Range(r, st) => write!($f, "{}..<{}:{}", r.start, r.end, st),
            Self::BinaryOp(op, l, r) => write!($f, "{} {} {}", l, op, r),
            Self::RangeOp(op, l, r, step) => match step {
                Some(st) => write!($f, "{}{}{}:{}", l, op, r, st),
                _ => write!($f, "{}{}{}", l, op, r),
            },
            Self::Pipe(op, l, r) => write!($f, "{} {} {}", l, op, r),
            Self::Index(l, r) => write!($f, "{}[{}]", l, r),
            Self::Builtin(builtin) => fmt::Debug::fmt(builtin, $f),
            Self::Use(n, p) => write!(
                $f,
                "use {} as {}",
                p,
                match n {
                    Some(name) => name.as_str(),
                    _ => "_",
                }
            ),
            Self::ModuleEnv(_) => write!($f, "module env"),
            Self::Catch(body, ctyp, deel) => match ctyp {
                CatchType::Ignore => write!($f, "{} ?.", body),
                CatchType::PrintStd => write!($f, "{} ?+", body),
                CatchType::PrintErr => write!($f, "{} ??", body),
                CatchType::PrintOver => write!($f, "{} ?>", body),
                CatchType::Terminate => write!($f, "{} ?!", body),
                CatchType::Deel => match deel {
                    Some(deelx) => write!($f, "{} ?: {}", body, deelx),
                    _ => write!($f, "{} ?: {{}}", body),
                },
            }, // Self::Error { code, msg, expr } => {
               //     write!($f, "Error<(code:{}\nmsg:{}\nexpr:{:?})>", code, msg, expr)
               // } // _ => write!($f, "Unreachable"), // 作为兜底逻辑
        }
    };
}
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
fn idt(indent: usize) -> String {
    "  ".repeat(indent)
}
// Display 实现
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_shared!(self, f, false)
    }
}

// Expression 辅助函数
impl Expression {
    fn fmt_indent(&self, f: &mut fmt::Formatter, indent: usize) -> fmt::Result {
        let i = if f.alternate() { indent + 1 } else { indent };
        write!(f, "{}", idt(i))?;
        match &self {
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
            Self::None => write!(f, "None"),

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
                    let _ = write!(f, "\n{}{:?}:{:?}\n", idt(i + 1), k, v);
                });
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Map(exprs) => {
                write!(f, "\n{}{{\n", idt(i))?;
                exprs.iter().for_each(|(k, v)| {
                    let _ = write!(f, "\n{}{:?}:{:?}\n", idt(i + 1), k, v);
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

            Self::BinaryOp(op, l, r) | Self::Pipe(op, l, r) => {
                let _ = writeln!(f);
                let _ = l.fmt_indent(f, i + 1);
                let _ = write!(f, "\n{}{}\n", idt(i + 1), op);
                r.fmt_indent(f, i + 1)
            }

            Self::If(cond, true_expr, false_expr) => {
                write!(f, "\n{}if ({:?}) {{\n", idt(i), cond)?;
                let _ = true_expr.fmt_indent(f, i + 1);
                let _ = write!(f, "\n{}}}else{{\n", idt(i));
                let _ = false_expr.fmt_indent(f, i + 1);
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Match(value, branches) => {
                write!(f, "\n{}match {} {{", idt(i), value)?;
                for (pat, expr) in branches.iter() {
                    write!(
                        f,
                        "\n{}{} => {:?}, ",
                        idt(i + 1),
                        pat.iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        expr
                    )?;
                }
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::For(name, list, body) => {
                write!(f, "\n{}for {} in {:?} {{\n", idt(i), name, list)?;
                let _ = body.fmt_indent(f, i + 1);
                write!(f, "\n{}}}\n", idt(i))
            }

            Self::While(cond, body) => {
                write!(f, "\n{}while {:?} {{\n", idt(i), cond)?;
                let _ = body.as_ref().fmt_indent(f, i + 1);
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Loop(body) => {
                write!(f, "\n{}loop {{\n", idt(i))?;
                let _ = body.as_ref().fmt_indent(f, i + 1);
                write!(f, "\n{}}}\n", idt(i))
            }
            Self::Lambda(params, body) => {
                write!(
                    f,
                    "\n{}Lambda ({}) ->\n",
                    idt(i),
                    params.iter().cloned().collect::<Vec<_>>().join(",")
                )?;
                body.as_ref().fmt_indent(f, i + 1)
            }
            Self::Function(name, param, pc, body, _) => {
                let _ = write!(
                    f,
                    "\n{}fn {}({},*{}) {{\n",
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
                    pc.clone().unwrap_or("None".to_string()),
                );
                let _ = body.fmt_indent(f, i + 1);
                write!(f, "{})\n", idt(i))
            }
            Self::Apply(func, args) => {
                write!(f, "\n{}Apply 〈{:?}〉\n{}(\n", idt(i), func, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = write!(f, "\n");
                });
                write!(f, "{})\n", idt(i))
            }

            Self::Command(cmd, args) => {
                write!(f, "\n{}Cmd 〈{:?}〉\n{}〖\n", idt(i), cmd, idt(i))?;
                args.iter().for_each(|e| {
                    let _ = e.fmt_indent(f, i + 1);
                    let _ = writeln!(f);
                });
                write!(f, "{}〗\n", idt(i))
            }
            _ => {
                // let _ = write!(f, "\n{}", idt(i));
                fmt_shared!(self, f, true)
            }
        }
    }

    /// 类型名称
    pub fn get_module_name(&self) -> Cow<'static, str> {
        match self {
            Self::List(_) | Self::Range(..) => "List".into(),
            Self::Map(_) | Self::HMap(_) => "Map".into(),
            Self::String(_) | Self::StringTemplate(_) | Self::Bytes(_) => "String".into(),
            Self::Integer(_) | Self::Float(_) => "Math".into(),
            Self::DateTime(_) => "Time".into(),
            Self::Symbol(_) => "Symbol".into(),
            _ => "otherModule".into(),
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

            Self::AliasOp(..) => "AliasOp".into(),
            Self::Range(..) => "Range".into(),
            Self::Chain(_, _) => "Chain".into(),
            Self::PipeMethod(_, _) => "PipeMethod".into(),
            Self::DestructureAssign(_, _) => "DestructureAssign".into(),

            // Self::Error { .. } => "Error".into(),
            Self::Use(..) => "Use".into(),
            Self::ModuleEnv(..) => "ModuleEnv".into(),
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
