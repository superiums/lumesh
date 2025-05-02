use crate::expression::pipe_excutor::handle_command;
use crate::{Environment, Expression, Int, RuntimeError, binary};
use core::option::Option::None;
use regex_lite::Regex;

use crate::STRICT;

const MAX_RECURSION_DEPTH: Option<usize> = Some(800);

impl Expression {
    /// 当返回symbol时，作为命令继续执行。
    pub fn eval_cmd(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        let result = self.clone().eval_mut(true, env, 0);
        return match result {
            // apply symbol cmds
            // Ok(Expression::Symbol(sym)) => {
            //     Expression::Apply(Box::new(Expression::Symbol(sym)), vec![]).eval(env)
            // }
            Ok(other) => Ok(other),
            Err(e) => Err(e),
        };
    }
    /// 当返回symbol时，作为字面量，直接返回。
    pub fn eval(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        self.clone().eval_mut(true, env, 0)
    }
    /// 求值主逻辑（尾递归优化）
    pub fn eval_mut(
        self,
        explain_builtin: bool,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        dbg!("1.--->eval_mut:", &self, &self.type_name());
        if let Some(max) = MAX_RECURSION_DEPTH {
            if depth > max {
                return Err(RuntimeError::RecursionDepth(self));
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
                | Self::Macro(_, _) => {
                    // dbg!("basic type");
                    break Ok(self);
                }
                Self::Builtin(_) => {
                    // dbg!("builtin type");
                    break Ok(self);
                }

                // 符号解析（错误处理优化）
                Self::Symbol(name) => {
                    // dbg!("2.--->symbol----", &name);
                    // bultin
                    if explain_builtin {
                        match binary::get_builtin(&name) {
                            Some(bti) => {
                                // dbg!("found builtin:", &name, bti);
                                return Ok(bti.clone());
                            }
                            _ => {}
                        };
                    }

                    // var
                    let r = match env.get(&name) {
                        Some(expr) => expr,
                        None => Self::Symbol(name),
                        // None => unsafe {
                        //            if STRICT {
                        //                Err(Error::UndeclaredVariable(name.clone()))
                        //            } else {
                        //                Ok(Self::Symbol(name)) // 非严格模式允许未定义符号
                        //            }
                        //        }
                    };

                    // dbg!(&r);
                    return Ok(r);
                }

                // 处理变量声明（仅允许未定义变量）
                Self::Declare(name, expr) => {
                    dbg!("declare---->", &name, &expr.type_name());

                    unsafe {
                        if STRICT && env.has(&name)
                        // && env.get("STRICT") == Some(Expression::Boolean(true))
                        {
                            return Err(RuntimeError::Redeclaration(name.to_string()));
                        }
                    }
                    let value = expr.eval_mut(false, env, depth + 1)?;
                    dbg!("declare---->", &name, &value.type_name());

                    env.define(&name, value); // 新增 declare
                    // dbg!("declare---->", &name, env.get(&name));
                    return Ok(Self::None);
                }

                // Assign 优先修改子环境，未找到则修改父环境
                Self::Assign(name, expr) => {
                    dbg!("assign---->", &name, &expr.type_name());
                    let value = expr.eval_mut(false, env, depth + 1)?;
                    dbg!("assign---->", &name, &value.type_name());
                    if env.has(&name) {
                        env.define(&name, value.clone());
                    } else {
                        // 向上层环境查找并修改（根据语言设计需求）
                        let mut current_env = env.clone();
                        while let Some(parent) = current_env.get_parent_mut() {
                            if parent.has(&name) {
                                parent.define(&name, value.clone());
                                return Ok(value);
                            }
                            current_env = parent.clone();
                        }
                        unsafe {
                            if STRICT
                            // && env.get("STRICT") == Some(Expression::Boolean(true))
                            {
                                return Err(RuntimeError::UndeclaredVariable(name));
                            } else {
                                env.define(&name, value.clone());
                            }
                        }
                    }
                    return Ok(value);
                }

                // TODO 是否只能删除当前env的变量，是否报错
                // del
                Self::Del(name) => {
                    env.undefine(&name);
                    return Ok(Self::None);
                }

                // 元表达式处理
                Self::Group(inner) => {
                    // dbg!("2.--->group:", &inner);
                    return inner.eval_mut(true, env, depth + 1);
                }
                Self::Quote(inner) => return Ok(*inner),

                // 一元运算
                Self::UnaryOp(op, operand, is_prefix) => {
                    let operand_eval = operand.eval(env)?;
                    return match op.as_str() {
                        "!" => Ok(Expression::Boolean(!operand_eval.is_truthy())),
                        "-" => match operand_eval {
                            Expression::Integer(i) => Ok(Expression::Integer(-i)),
                            Expression::Float(i) => Ok(Expression::Float(-i)),
                            _ => {
                                return Err(RuntimeError::CustomError(format!(
                                    "Cannot apply Neg to {operand:?}:{operand_eval:?}"
                                )));
                            }
                        },
                        // 处理 ++a 转换为 a = a + 1
                        "++" | "--" => {
                            // 确保操作数是符号
                            let var_name = operand.to_symbol()?;
                            // 获取当前值
                            let current_val = env
                                .get(var_name)
                                .ok_or(RuntimeError::UndeclaredVariable(var_name.to_string()))?;
                            // 确保操作是合法的，例如整数或浮点数
                            if !matches!(current_val, Expression::Integer(_) | Expression::Float(_))
                            {
                                return Err(RuntimeError::CustomError(format!(
                                    "Cannot apply {op} to {operand:?}:{current_val:?}"
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
                        op if op.starts_with("__") => {
                            if let Some(oper) = env.get(op) {
                                let rs = Expression::Apply(Box::new(oper), vec![operand_eval]);
                                return rs.eval_mut(true, env, depth + 1);
                            }
                            Err(RuntimeError::CustomError(format!(
                                "custom operation {op:?} not defined"
                            )))
                        }
                        _ => Err(RuntimeError::CustomError(format!(
                            "Unknown unary operator: {op}"
                        ))),
                    };
                }
                // 特殊运算符

                // 二元运算
                Self::BinaryOp(operator, lhs, rhs) => {
                    break match operator.as_str() {
                        "+=" => match *lhs {
                            Expression::Symbol(base) => {
                                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                                left += rhs.eval(env)?;
                                env.define(&base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::CustomError(format!(
                                "cannot apply {} to  {}:{} and {}:{}",
                                operator,
                                lhs,
                                lhs.type_name(),
                                rhs,
                                rhs.type_name()
                            ))),
                        },
                        "-=" => match *lhs {
                            Expression::Symbol(base) => {
                                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                                left -= rhs.eval(env)?;
                                env.define(&base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::CustomError(format!(
                                "cannot apply {} to  {}:{} and {}:{}",
                                operator,
                                lhs,
                                lhs.type_name(),
                                rhs,
                                rhs.type_name()
                            ))),
                        },
                        "*=" => match *lhs {
                            Expression::Symbol(base) => {
                                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                                left *= rhs.eval(env)?;
                                env.define(&base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::CustomError(format!(
                                "cannot apply {} to  {}:{} and {}:{}",
                                operator,
                                lhs,
                                lhs.type_name(),
                                rhs,
                                rhs.type_name()
                            ))),
                        },
                        "/=" => match *lhs {
                            Expression::Symbol(base) => {
                                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                                let right = rhs.eval(env)?;
                                if !right.is_truthy() {
                                    return Err(RuntimeError::CustomError(format!(
                                        "can't divide {} by zero",
                                        base
                                    )));
                                };
                                left /= right;
                                env.define(&base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::CustomError(format!(
                                "cannot apply {} to  {}:{} and {}:{}",
                                operator,
                                lhs,
                                lhs.type_name(),
                                rhs,
                                rhs.type_name()
                            ))),
                        },
                        "&&" => Ok(Expression::Boolean(
                            lhs.eval_mut(true, env, depth + 1)?.is_truthy()
                                && rhs.eval_mut(true, env, depth + 1)?.is_truthy(),
                        )),
                        "||" => Ok(Expression::Boolean(
                            lhs.eval_mut(true, env, depth + 1)?.is_truthy()
                                || rhs.eval_mut(true, env, depth + 1)?.is_truthy(),
                        )),
                        _ => {
                            // fmt.red : left is builtin, right never.
                            let l = lhs.eval_mut(true, env, depth + 1)?;
                            let r = rhs.eval_mut(false, env, depth + 1)?;
                            break match operator.as_str() {
                                "+" => Ok(l + r),
                                "-" => Ok(l - r),
                                "*" => Ok(l * r),
                                "/" => {
                                    if !r.is_truthy() {
                                        return Err(RuntimeError::CustomError(format!(
                                            "can't divide {} by zero",
                                            l
                                        )));
                                    };
                                    Ok(l / r)
                                } //no zero
                                "%" => Ok(l % r),
                                "^" => match (l, r) {
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
                                            None => Err(RuntimeError::CustomError(format!(
                                                "overflow when raising int {} to the power {}",
                                                base, exponent
                                            ))),
                                        }
                                    }
                                    (a, b) => Err(RuntimeError::CustomError(format!(
                                        "cannot raise {}:{} to the power {}:{}",
                                        a,
                                        a.type_name(),
                                        b,
                                        b.type_name()
                                    ))),
                                },

                                "==" => Ok(Expression::Boolean(l == r)),
                                "!=" => Ok(Expression::Boolean(l != r)),
                                ">" => Ok(Expression::Boolean(l > r)),
                                "<" => Ok(Expression::Boolean(l < r)),
                                ">=" => Ok(Expression::Boolean(l >= r)),
                                "<=" => Ok(Expression::Boolean(l <= r)),
                                "~~" => {
                                    Ok(Expression::Boolean(l.to_string().contains(&r.to_string())))
                                }
                                "~=" => {
                                    let regex = Regex::new(&r.to_string())
                                        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

                                    Ok(Expression::Boolean(regex.is_match(&l.to_string())))
                                }

                                ".." => match (l, r) {
                                    (Expression::Integer(fr), Expression::Integer(t)) => {
                                        let v = (fr..t)
                                            .map(Expression::from) // 将 i64 转换为 Expression
                                            .collect();
                                        Ok(Expression::List(v))
                                    }
                                    _ => Err(RuntimeError::CustomError(
                                        "not valid range option".into(),
                                    )),
                                },
                                op if op.starts_with("_") => {
                                    if let Some(oper) = env.get(op) {
                                        let rs = Expression::Apply(Box::new(oper), vec![l, r]);
                                        return rs.eval_mut(true, env, depth + 1);
                                    }
                                    Err(RuntimeError::CustomError(format!(
                                        "custom operation {op:?} not defined"
                                    )))
                                }
                                // ----------
                                _ => Err(RuntimeError::InvalidOperator(operator.clone())),
                            };
                        }
                    };
                }

                // 列表求值（内存优化）
                // Self::List(elems) => {
                //     let evaluated = elems
                //         .iter()
                //         .map(|e| e.clone().eval_mut(true,env, depth + 1))
                //         .collect::<Result<Vec<_>, _>>()?;
                //     Ok(Expression::List(evaluated))
                // }
                Self::List(items) => {
                    return Ok(Self::List(
                        items
                            .iter()
                            .map(|e| e.clone().eval_mut(true, env, depth + 1))
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
                    let l = lhs.eval_mut(true, env, depth + 1)?;
                    let r = rhs.eval_mut(false, env, depth + 1)?; //TODO: allow dynamic Key? x.log log=builtin@log

                    return match (l, r) {
                        (Expression::Map(m), n) => Self::index_slm(Expression::Map(m), n),
                        (Self::Symbol(m), Self::Symbol(n)) => {
                            Ok(Self::String(format!("{}.{}", m, n)))
                        }
                        // (Self::String(m), Self::String(n)) => Ok(Self::String(m + &n)),
                        _ => Err(RuntimeError::CustomError("not valid index option".into())),
                    };
                }

                // 执行应用
                Self::Apply(_, _) => break Self::eval_apply(self, env, depth),
                Self::Command(ref cmd, ref args) => {
                    // dbg!(&cmd.type_name());
                    break match binary::get_builtin(&cmd.to_string()) {
                        Some(bti) => {
                            // dbg!("branch to builtin:", &cmd, &bti);
                            bti.clone().apply(args.clone()).eval_apply(env, depth)
                        }
                        _ => handle_command(cmd.to_string(), args, env, depth),
                    };
                }
                // break Self::eval_command(self, env, depth),
                // 其他表达式处理...
                _ => break self.eval_complex(env, depth),
            };
            // depth += 1
        }
    }
}

impl Expression {
    /// 索引访问
    fn index_slm(l: Expression, r: Expression) -> Result<Expression, RuntimeError> {
        match l {
            // 处理列表索引
            Expression::List(list) => {
                if let Expression::Integer(index) = r {
                    list.get(index as usize).cloned().ok_or_else(|| {
                        RuntimeError::IndexOutOfBounds {
                            index: index as Int,
                            len: list.len(),
                        }
                    })
                } else {
                    Err(RuntimeError::TypeError {
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
                    .ok_or_else(|| RuntimeError::KeyNotFound(key))
            }

            // 处理字符串索引
            Expression::String(s) => {
                if let Expression::Integer(index) = r {
                    s.chars()
                        .nth(index as usize)
                        .map(|c| Expression::String(c.to_string()))
                        .ok_or_else(|| RuntimeError::IndexOutOfBounds {
                            index: index as Int,
                            len: s.len(),
                        })
                } else {
                    Err(RuntimeError::TypeError {
                        expected: "integer".into(),
                        found: r.type_name(),
                    })
                }
            }

            _ => Err(RuntimeError::TypeError {
                expected: "indexable type (list/dict/string)".into(),
                found: l.type_name(),
            }),
        }
    }

    pub fn as_list(&self) -> Result<&Vec<Self>, RuntimeError> {
        match self {
            Self::List(v) => Ok(v),
            _ => Err(RuntimeError::TypeError {
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
    ) -> Result<Self, RuntimeError> {
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
    ) -> Result<Option<Int>, RuntimeError> {
        match expr_opt {
            // 无表达式时返回 None
            None => Ok(None),
            // 有表达式时进行求值
            Some(boxed_expr) => {
                // 递归求值表达式
                let evaluated = boxed_expr.eval_mut(true, env, depth)?;

                // 转换为整数
                match evaluated {
                    Self::Integer(i) => Ok(Some(i)),
                    // 处理隐式类型转换
                    Self::Float(f) if f.fract() == 0.0 => Ok(Some(f as Int)),
                    // 处理其他类型错误
                    _ => Err(RuntimeError::TypeError {
                        expected: "integer".into(),
                        found: evaluated.type_name(),
                    }),
                }
            }
        }
    }
}
