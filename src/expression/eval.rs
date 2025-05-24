use super::Pattern;
use super::cmd_excutor::eval_command;
use crate::STRICT;
use crate::expression::alias;
use crate::{Environment, Expression, Int, RuntimeError, modules::get_builtin};
use core::option::Option::None;
use regex_lite::Regex;
use smallstr::SmallString;
use std::collections::{BTreeMap, HashMap};
use std::io::ErrorKind;
use std::io::Write;
use std::rc::Rc;

const MAX_RECURSION_DEPTH: Option<usize> = Some(800);

#[derive(Debug, Clone)]
pub struct State(u8, Option<Expression>);

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub const SKIP_BUILTIN_SEEK: u8 = 1 << 1; // 0b00000010
    pub const IN_PIPE: u8 = 1 << 2; // 0b00000100
    // pub const PIPE_IN: u8 = 1 << 3; // 0b00001000

    // 创建一个新的 State 实例
    pub fn new() -> Self {
        State(0, None)
    }

    // 设置标志
    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }

    // 清除标志
    pub fn clear(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    // 检查标志是否被设置
    pub fn contains(&self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn pipe_in(&mut self, data: Expression) {
        self.1 = Some(data);
    }

    pub fn pipe_out(&mut self) -> Option<Expression> {
        let p = self.1.clone();
        self.1 = None;
        p
    }
}

impl Expression {
    /// 交互命令入口
    pub fn eval_cmd(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        let result = self.eval_mut(&mut State::new(), env, 0);
        // dbg!(&result);
        match result {
            // apply symbol cmds
            // Ok(Expression::Symbol(sym)) => {
            //     Expression::Apply(Box::new(Expression::Symbol(sym)), vec![]).eval(env)
            // }
            Ok(other) => {
                // dbg!(other.type_name());
                Ok(other)
            }
            Err(e) => Err(e),
        }
    }
    /// 脚本计算入口
    pub fn eval(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        self.eval_mut(&mut State::new(), env, 0)
    }
    /// 求值主逻辑
    #[inline]
    pub fn eval_mut(
        &self,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        // dbg!("1.--->eval_mut:", &self, &self.type_name());
        if let Some(max) = MAX_RECURSION_DEPTH {
            if depth > max {
                return Err(RuntimeError::RecursionDepth(self.clone()));
            }
        }
        let mut job = self;
        loop {
            // println!();
            // dbg!("------", &job);
            match job {
                // 基础类型直接返回
                Self::String(_)
                | Self::Boolean(_)
                | Self::Integer(_)
                | Self::None
                | Self::Float(_)
                | Self::Bytes(_)
                | Self::Range(_)
                | Self::DateTime(_) => {
                    // dbg!("basic type");
                    return Ok(job.clone());
                }
                Self::Builtin(_) => {
                    // dbg!("builtin type");
                    break Ok(job.clone());
                }

                // 符号解析（错误处理优化）
                Self::Symbol(name) => {
                    // dbg!("2.--->symbol----", &name);
                    // bultin
                    if !state.contains(State::SKIP_BUILTIN_SEEK) {
                        if let Some(b) = get_builtin(name) {
                            // dbg!("found builtin:", &name, bti);
                            return Ok(b.clone());
                        };
                    }

                    // var
                    unsafe {
                        if STRICT {
                            return Ok(job.clone());
                        } else {
                            return match env.get(name) {
                                Some(expr) => Ok(expr),
                                None => Ok(job.clone()),
                            };
                        }
                    }
                }
                Self::Variable(name) => {
                    // dbg!("2.--->variable----", &name);
                    // var
                    return match env.get(name) {
                        Some(expr) => Ok(expr),
                        None => Err(RuntimeError::UndeclaredVariable(name.clone())),
                    };
                }

                // 处理变量声明（仅允许未定义变量）
                Self::Declare(name, expr) => {
                    // dbg!("declare---->", &name, &expr.type_name());
                    unsafe {
                        if STRICT && env.has(name)
                        // && env.get("STRICT") == Some(Expression::Boolean(true))
                        {
                            return Err(RuntimeError::Redeclaration(name.to_owned()));
                        }
                    }
                    if let Expression::Command(..) | Expression::Group(..) | Expression::Pipe(..) =
                        expr.as_ref()
                    {
                        // env.define("__ALWAYSPIPE", Expression::Boolean(true));
                        let is_in_pipe = state.contains(State::IN_PIPE);
                        state.set(State::IN_PIPE);
                        let value = expr.as_ref().eval_mut(state, env, depth + 1)?;
                        if !is_in_pipe {
                            state.clear(State::IN_PIPE);
                        }
                        // env.undefine("__ALWAYSPIPE");
                        env.define(name, value); // 新增 declare
                    } else {
                        let value = expr.as_ref().eval_mut(state, env, depth + 1)?;
                        env.define(name, value); // 新增 declare
                    }
                    // dbg!("declare---->", &name, &value.type_name());
                    return Ok(Self::None);
                }

                // Assign 优先修改子环境，未找到则修改父环境
                Self::Assign(name, expr) => {
                    // dbg!("assign---->", &name, &expr.type_name());
                    let need_clear = match expr.as_ref() {
                        Expression::Command(..) | Expression::Group(..) | Expression::Pipe(..) => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            true && is_in_pipe
                        }
                        _ => false,
                    };

                    let value = expr.as_ref().eval_mut(state, env, depth + 1)?;

                    // dbg!("assign---->", &name, &value.type_name());
                    if env.has(name) {
                        env.define(name, value.clone());
                    } else {
                        // 向上层环境查找并修改（根据语言设计需求）
                        let mut current_env = env.clone();
                        while let Some(parent) = current_env.get_parent_mut() {
                            if parent.has(name) {
                                parent.define(name, value.clone());
                                return Ok(value);
                            }
                            current_env = parent.clone();
                        }
                        unsafe {
                            if STRICT
                            // && env.get("STRICT") == Some(Expression::Boolean(true))
                            {
                                return Err(RuntimeError::UndeclaredVariable(
                                    SmallString::from_str(name),
                                ));
                            } else {
                                env.define(name, value.clone());
                            }
                        }
                    }
                    if need_clear {
                        // env.undefine("__ALWAYSPIPE");
                        state.clear(State::IN_PIPE);
                    }
                    return Ok(value);
                }

                // del
                Self::Del(name) => {
                    env.undefine(name);
                    return Ok(Self::None);
                }

                // 处理变量声明（仅允许未定义变量）
                Self::Alias(name, expr) => {
                    // dbg!("alias---->", &name, &expr.type_name());
                    alias::set_alias(name.clone(), expr.as_ref().clone()); // 新增 declare
                    return Ok(Self::None);
                }

                // 元表达式处理
                Self::Group(inner) => {
                    // dbg!("2.--->group:", &inner);
                    // return inner.as_ref().eval_mut(state, env, depth + 1);
                    job = inner.as_ref();
                    continue;
                }
                // Self::Quote(inner) => return Ok(inner.as_ref().clone()),
                Self::Quote(inner) => {
                    job = inner.as_ref();
                    continue;
                }

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
                            let current_val = env.get(var_name).ok_or(
                                RuntimeError::UndeclaredVariable(SmallString::from_str(var_name)),
                            )?;
                            // 确保操作是合法的，例如整数或浮点数
                            if !matches!(current_val, Expression::Integer(_) | Expression::Float(_))
                            {
                                return Err(RuntimeError::CustomError(format!(
                                    "Cannot apply {op} to {operand:?}:{current_val:?}"
                                )));
                            }
                            // 计算新值
                            let step = if op == "++" { 1 } else { -1 };
                            let new_val = (current_val.clone() + Expression::Integer(step))?;
                            env.define(var_name, new_val.clone());
                            Ok(if is_prefix == &true {
                                new_val
                            } else {
                                current_val
                            })
                        }
                        op if op.starts_with("__") => {
                            if let Some(oper) = env.get(op) {
                                let rs =
                                    Expression::Apply(Rc::new(oper), Rc::new(vec![operand_eval]));
                                return rs.eval_mut(state, env, depth + 1);
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
                        "+=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left += rhs.eval(env)?;
                                env.define(base, left.clone());
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
                        "-=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left -= rhs.eval(env)?;
                                env.define(base, left.clone());
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
                        "*=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left *= rhs.eval(env)?;
                                env.define(base, left.clone());
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
                        "/=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                let right = rhs.eval(env)?;
                                if !right.is_truthy() {
                                    return Err(RuntimeError::CustomError(format!(
                                        "can't divide {} by zero",
                                        base
                                    )));
                                };
                                left /= right;
                                env.define(base, left.clone());
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
                            lhs.as_ref().eval_mut(state, env, depth + 1)?.is_truthy()
                                && rhs.as_ref().eval_mut(state, env, depth + 1)?.is_truthy(),
                        )),
                        "||" => Ok(Expression::Boolean(
                            lhs.as_ref().eval_mut(state, env, depth + 1)?.is_truthy()
                                || rhs.as_ref().eval_mut(state, env, depth + 1)?.is_truthy(),
                        )),
                        _ => {
                            // fmt.red : left is builtin, right never.
                            let l = lhs.as_ref().eval_mut(state, env, depth + 1)?;
                            let r = rhs.as_ref().eval_mut(state, env, depth + 1)?;
                            break match operator.as_str() {
                                "+" => l + r,
                                "-" => l - r,
                                "*" => l * r,
                                "/" => l / r, //no zero
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
                                        // 确保 exponent 是非负的
                                        if exponent < 0 {
                                            return Err(RuntimeError::CustomError(format!(
                                                "cannot raise {} to a negative power {}",
                                                base, exponent
                                            )));
                                        }

                                        // 使用 checked_pow 进行幂运算
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
                                "~=" => Ok(Expression::Boolean(l.to_string() == r.to_string())),
                                "!=" => Ok(Expression::Boolean(l != r)),
                                ">" => Ok(Expression::Boolean(l > r)),
                                "<" => Ok(Expression::Boolean(l < r)),
                                ">=" => Ok(Expression::Boolean(l >= r)),
                                "<=" => Ok(Expression::Boolean(l <= r)),
                                "~:" => {
                                    let br = match l {
                                        Expression::String(left) => left.contains(&r.to_string()),
                                        Expression::Range(left) => {
                                            if let Expression::Integer(i) = r {
                                                left.contains(&i)
                                            } else {
                                                false
                                            }
                                        }
                                        Expression::List(left) => left.contains(&r),
                                        Expression::Map(left) => left.contains_key(r.to_symbol()?),
                                        _ => false,
                                    };
                                    Ok(Expression::Boolean(br))
                                    // Ok(Expression::Boolean(l.to_string().contains(&r.to_string())))
                                }
                                "~~" => {
                                    let regex = Regex::new(&r.to_string())
                                        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

                                    Ok(Expression::Boolean(regex.is_match(&l.to_string())))
                                }

                                ".." => match (l, r) {
                                    (Expression::Integer(fr), Expression::Integer(t)) => {
                                        // let v = (fr..t)
                                        //     .map(Expression::from) // 将 i64 转换为 Expression
                                        //     .collect::<Vec<_>>();
                                        // Ok(Expression::from(v))
                                        Ok(Expression::Range(fr..t))
                                    }
                                    _ => Err(RuntimeError::CustomError(
                                        "not valid range option".into(),
                                    )),
                                },
                                "..=" => match (l, r) {
                                    (Expression::Integer(fr), Expression::Integer(t)) => {
                                        // let v = (fr..=t)
                                        //     .map(Expression::from) // 将 i64 转换为 Expression
                                        //     .collect::<Vec<_>>();
                                        // Ok(Expression::from(v))
                                        Ok(Expression::Range(fr..t + 1))
                                    }
                                    _ => Err(RuntimeError::CustomError(
                                        "not valid range option".into(),
                                    )),
                                },
                                op if op.starts_with("_") => {
                                    if let Some(oper) = env.get(op) {
                                        let rs =
                                            Expression::Apply(Rc::new(oper), Rc::new(vec![l, r]));
                                        return rs.eval_mut(state, env, depth + 1);
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
                // 管道
                Self::Pipe(operator, lhs, rhs) => {
                    match operator.as_str() {
                        "|" => {
                            // let bindings = env.get_bindings_map();
                            // let always_pipe = env.has("__ALWAYSPIPE");
                            //dbg!(&always_pipe, &lhs, &rhs);
                            // if always_pipe {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.ensure_apply();
                            let left_output = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }
                            state.pipe_in(left_output);

                            match rhs.as_ref() {
                                Expression::Symbol(s) => {
                                    return Expression::Apply(
                                        Rc::new(Expression::Symbol(s.clone())),
                                        Rc::new(vec![]),
                                    )
                                    .eval_apply(state, env, depth);
                                }
                                r => {
                                    job = r;
                                    continue;
                                }
                            }

                            // let r_func = rhs.ensure_apply();
                            // let pipe_result = r_func.eval_mut(state, env, depth + 1);
                            // dbg!(&pipe_result);
                            // pipe_result
                        }
                        "|>" => {
                            // 执行左侧表达式
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.as_ref().ensure_apply();
                            let left_output = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }

                            // 执行右侧表达式，获取函数或命令
                            // 将左侧结果作为最后一个参数传递给右侧
                            let args = vec![left_output];
                            return rhs
                                .as_ref()
                                .append_args(args)
                                .eval_mut(state, env, depth + 1);
                        }
                        ">>" => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.as_ref().ensure_apply();
                            let left_output = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }

                            let mut path = std::env::current_dir()?;
                            path = path
                                .join(rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string());
                            if !path.exists() {
                                std::fs::File::create(path.clone())?;
                            }
                            match std::fs::OpenOptions::new().append(true).open(&path) {
                                Ok(mut file) => {
                                    // use std::io::prelude::*;
                                    let result =
                                        if let Expression::Bytes(bytes) = left_output.clone() {
                                            // std::fs::write(path, bytes)
                                            file.write_all(&bytes)
                                        } else {
                                            // Otherwise, convert the contents to a pretty string and write that.
                                            // std::fs::write(path, contents.to_string())
                                            file.write_all(left_output.to_string().as_bytes())
                                        };

                                    return match result {
                                        Ok(()) => Ok(left_output),
                                        Err(e) => Err(RuntimeError::CustomError(format!(
                                            "could not append to file {}: {:?}",
                                            rhs, e
                                        ))),
                                    };
                                }
                                Err(e) => {
                                    return Err(match e.kind() {
                                        ErrorKind::PermissionDenied => {
                                            RuntimeError::PermissionDenied(rhs.as_ref().clone())
                                        }
                                        _ => RuntimeError::CustomError(format!(
                                            "could not open file {}: {:?}",
                                            path.display(),
                                            e
                                        )),
                                    });
                                }
                            }
                        }
                        ">>!" => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.as_ref().ensure_apply();
                            let l = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }

                            // dbg!("-->> left=", &l);
                            let mut path = std::env::current_dir()?;
                            path = path
                                .join(rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string());
                            // If the contents are bytes, write the bytes directly to the file.
                            let result = if let Expression::Bytes(bytes) = l.clone() {
                                std::fs::write(path, bytes)
                            } else {
                                // Otherwise, convert the contents to a pretty string and write that.
                                std::fs::write(path, l.to_string())
                            };

                            return match result {
                                Ok(()) => Ok(l),
                                Err(e) => Err(RuntimeError::CustomError(format!(
                                    "could not write to file {}: {:?}",
                                    rhs, e
                                ))),
                            };
                        }
                        "<<" => {
                            // 输入重定向处理
                            // handle_stdin_redirect(lhs, rhs, state, env, depth, true)
                            let path = rhs.eval_mut(state, env, depth + 1)?;
                            let contents = std::fs::read_to_string(path.to_string())
                                .map(Self::String)
                                .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

                            state.pipe_in(contents);

                            let left_func = lhs.ensure_apply();
                            let result = left_func.eval_mut(state, env, depth + 1)?;
                            return Ok(result);
                        }
                        _ => unreachable!(),
                    }
                }
                // 列表求值（内存优化）
                // Self::List(elems) => {
                //     let evaluated = elems
                //         .iter()
                //         .map(|e| e.eval_mut(true,env, depth + 1))
                //         .collect::<Result<Vec<_>, _>>()?;
                //     Ok(Expression::List(evaluated))
                // }
                Self::List(items) => {
                    let evaluated = items
                        .as_ref()
                        .iter()
                        .map(|e| e.eval_mut(state, env, depth + 1))
                        .collect::<Result<Vec<_>, _>>()?;
                    return Ok(Expression::from(evaluated));
                }
                Self::HMap(items) => {
                    let evaluated = items
                        .iter()
                        .map(|(k, e)| Ok((k.clone(), e.eval_mut(state, env, depth + 1)?)))
                        .collect::<Result<HashMap<_, _>, RuntimeError>>()?;
                    return Ok(Expression::from(evaluated));
                }
                Self::Map(items) => {
                    let evaluated = items
                        .iter()
                        .map(|(k, e)| Ok((k.clone(), e.eval_mut(state, env, depth + 1)?)))
                        .collect::<Result<BTreeMap<_, _>, RuntimeError>>()?;
                    return Ok(Expression::from(evaluated));
                }

                // 其他复杂类型
                Self::Slice(list, slice_params) => {
                    let listo = list.eval(env)?;
                    let start_int =
                        Expression::eval_to_int_opt(&slice_params.start, state, env, depth)?;
                    let end_int =
                        Expression::eval_to_int_opt(&slice_params.end, state, env, depth)?;
                    let step_int =
                        Expression::eval_to_int_opt(&slice_params.step, state, env, depth)?
                            .unwrap_or(1); // 默认步长1

                    return Self::slice(listo, start_int, end_int, step_int);
                }
                Self::Index(lhs, rhs) => {
                    let l = lhs.as_ref().eval_mut(state, env, depth + 1)?;
                    state.set(State::SKIP_BUILTIN_SEEK);
                    let r = rhs.as_ref().eval_mut(state, env, depth + 1)?; //TODO: allow dynamic Key? x.log log=builtin@log
                    state.clear(State::SKIP_BUILTIN_SEEK);

                    return match (l, r) {
                        // (Expression::List(m), Expression::Integer(n)) => {
                        //     Self::index_slm(Expression::List(m), Expression::Integer(n))
                        // }
                        // (Expression::Map(m), n) => Self::index_slm(Expression::Map(m), n),
                        (Self::Symbol(m), Self::Symbol(n)) => {
                            Ok(Self::String(format!("{}.{}", m, n)))
                        }
                        // (Self::String(m), Self::String(n)) => Ok(Self::String(m + &n)),
                        // _ => Err(RuntimeError::CustomError("not valid index option".into())),
                        (left, right) => Self::index_slm(left, right),
                    };
                }

                // 执行应用
                Self::Apply(_, _) => break Self::eval_apply(job, state, env, depth),
                Self::Command(cmd, args) => {
                    return eval_command(cmd, args, state, env, depth);
                }
                // break Self::eval_command(self, env, depth),
                // 简单控制流表达式
                Self::If(cond, true_expr, false_expr) => {
                    match cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy() {
                        true => job = true_expr.as_ref(),
                        false => job = false_expr.as_ref(),
                    };
                    continue;
                }

                Self::Match(value, branches) => {
                    // 模式匹配求值
                    let val = value.as_ref().eval_mut(state, env, depth + 1)?;
                    let mut matched = false;
                    for (pat, expr) in branches {
                        if matches_pattern(&val, pat, env)? {
                            job = expr.as_ref();
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        continue;
                    }
                    return Err(RuntimeError::NoMatchingBranch(val.to_string()));
                }
                // 其他表达式处理...
                _ => break job.eval_complex(state, env, depth),
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
                    list.as_ref().get(index as usize).cloned().ok_or_else(|| {
                        RuntimeError::IndexOutOfBounds {
                            index: index as Int,
                            len: list.as_ref().len(),
                        }
                    })
                } else {
                    Err(RuntimeError::TypeError {
                        expected: "integer".into(),
                        found: r.to_string(),
                        found_type: r.type_name(),
                    })
                }
            }

            // 处理字典键访问
            Expression::HMap(map) => {
                let key = SmallString::from(r.to_string()); // 自动转换Symbol/字符串
                map.as_ref()
                    .get(&key)
                    .cloned()
                    .ok_or(RuntimeError::KeyNotFound(key))
            }
            Expression::Map(map) => {
                let key = SmallString::from(r.to_string()); // 自动转换Symbol/字符串
                map.as_ref()
                    .get(&key)
                    .cloned()
                    .ok_or(RuntimeError::KeyNotFound(key))
            }

            // 处理字符串索引
            Expression::String(s) => {
                if let Expression::Integer(index) = r {
                    s.chars()
                        .nth(index as usize)
                        .map(|c| Expression::String(c.to_string()))
                        .ok_or(RuntimeError::IndexOutOfBounds {
                            index: index as Int,
                            len: s.len(),
                        })
                } else {
                    Err(RuntimeError::TypeError {
                        expected: "integer".into(),
                        found: r.to_string(),
                        found_type: r.type_name(),
                    })
                }
            }

            _ => Err(RuntimeError::TypeError {
                expected: "indexable type (list/dict/string)".into(),
                found: l.to_string(),
                found_type: l.type_name(),
            }),
        }
    }

    pub fn as_list(&self) -> Result<&Vec<Expression>, RuntimeError> {
        match self {
            Self::List(v) => Ok(v.as_ref()),
            _ => Err(RuntimeError::TypeError {
                expected: "list".into(),
                found: self.to_string(),
                found_type: self.type_name(),
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
        Ok(Self::from(result))
    }

    /// 辅助方法：将表达式求值为整数选项
    pub fn eval_to_int_opt(
        expr_opt: &Option<Rc<Self>>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Option<Int>, RuntimeError> {
        match expr_opt {
            // 无表达式时返回 None
            None => Ok(None),
            // 有表达式时进行求值
            Some(boxed_expr) => {
                // 递归求值表达式
                let evaluated = boxed_expr.as_ref().eval_mut(state, env, depth)?;

                // 转换为整数
                match evaluated {
                    Self::Integer(i) => Ok(Some(i)),
                    // 处理隐式类型转换
                    Self::Float(f) if f.fract() == 0.0 => Ok(Some(f as Int)),
                    // 处理其他类型错误
                    _ => Err(RuntimeError::TypeError {
                        expected: "integer".into(),
                        found: evaluated.to_string(),
                        found_type: evaluated.type_name(),
                    }),
                }
            }
        }
    }
}

impl Expression {
    // 列表追加示例（写时复制）
    pub fn list_push(&self, item: Self) -> Result<Expression, RuntimeError> {
        match self {
            Self::List(items) => {
                let mut new_vec = Vec::with_capacity(items.len() + 1);
                new_vec.extend_from_slice(items);
                new_vec.push(item);
                Ok(Self::List(Rc::new(new_vec)))
            }
            s => Err(RuntimeError::TypeError {
                expected: "List".into(),
                found: s.to_string(),
                found_type: s.type_name(),
            }),
        }
    }
    pub fn list_append(&self, other: Rc<Vec<Expression>>) -> Result<Expression, RuntimeError> {
        match self {
            Self::List(items) => {
                let mut new_vec = Vec::with_capacity(items.len() + other.len());
                new_vec.extend_from_slice(items);
                new_vec.extend_from_slice(&other);
                Ok(Self::List(Rc::new(new_vec)))
            }
            s => Err(RuntimeError::TypeError {
                expected: "List".into(),
                found: s.to_string(),
                found_type: s.type_name(),
            }),
        }
    }

    // 映射插入示例
    pub fn map_insert(
        &self,
        key: SmallString<[u8; 16]>,
        value: Self,
    ) -> Result<Expression, RuntimeError> {
        match self {
            Self::HMap(map) => {
                let mut new_map = HashMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.insert(key, value);
                Ok(Self::from(new_map))
            }
            Self::Map(map) => {
                let mut new_map = BTreeMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.insert(key, value);
                Ok(Self::Map(Rc::new(new_map)))
            }
            s => Err(RuntimeError::TypeError {
                expected: "Map".into(),
                found: s.to_string(),
                found_type: s.type_name(),
            }),
        }
    }

    pub fn map_append(
        &self,
        other: Rc<HashMap<SmallString<[u8; 16]>, Expression>>,
    ) -> Result<Expression, RuntimeError> {
        match self {
            Self::HMap(map) => {
                let mut new_map = HashMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.extend(other.iter().map(|(k, v)| (k.clone(), v.clone())));
                Ok(Self::HMap(Rc::new(new_map)))
            }
            Self::Map(map) => {
                let mut new_map = BTreeMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.extend(other.iter().map(|(k, v)| (k.clone(), v.clone())));
                Ok(Self::Map(Rc::new(new_map)))
            }
            s => Err(RuntimeError::TypeError {
                expected: "Map".into(),
                found: s.to_string(),
                found_type: s.type_name(),
            }),
        }
    }
    pub fn bmap_append(
        &self,
        other: Rc<BTreeMap<SmallString<[u8; 16]>, Expression>>,
    ) -> Result<Expression, RuntimeError> {
        match self {
            Self::HMap(map) => {
                let mut new_map = HashMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.extend(other.iter().map(|(k, v)| (k.clone(), v.clone())));
                Ok(Self::HMap(Rc::new(new_map)))
            }
            Self::Map(map) => {
                let mut new_map = BTreeMap::new();
                new_map.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
                new_map.extend(other.iter().map(|(k, v)| (k.clone(), v.clone())));
                Ok(Self::Map(Rc::new(new_map)))
            }
            s => Err(RuntimeError::TypeError {
                expected: "Map".into(),
                found: s.to_string(),
                found_type: s.type_name(),
            }),
        }
    }
}

/// match的比对
fn matches_pattern(
    value: &Expression,
    pattern: &Pattern,
    env: &mut Environment,
) -> Result<bool, RuntimeError> {
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
