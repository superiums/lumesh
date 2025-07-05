use crate::expression::alias;
use crate::expression::cmd_excutor::expand_home;

use crate::RuntimeErrorKind;
use crate::expression::eval2::ifs_split;
use crate::expression::render::render_template;
use crate::{Environment, Expression, Int, RuntimeError, modules::get_builtin};
use core::option::Option::None;
use regex_lite::Regex;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use std::rc::Rc;

const MAX_RECURSION_DEPTH: usize = 800;

#[derive(Debug, Clone)]
pub struct State(u8, Option<Expression>);

impl Default for State {
    fn default() -> Self {
        Self::new(false)
    }
}

impl State {
    pub const STRICT: u8 = 1;
    pub const SKIP_BUILTIN_SEEK: u8 = 1 << 1; // 0b00000010
    pub const IN_PIPE: u8 = 1 << 2; // 0b00000100
    pub const PTY_MODE: u8 = 1 << 3; // 0b00001000
    pub const IN_ASSIGN: u8 = 1 << 4; // 0b00010000
    pub const NO_ENV_FORK: u8 = 1 << 5;
    pub const NO_RE_DECLARE: u8 = 1 << 6;

    // 创建一个新的 State 实例
    pub fn new(strict: bool) -> Self {
        if strict {
            State(State::STRICT, None)
        } else {
            State(0, None)
        }
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

fn is_strict(env: &mut Environment) -> bool {
    match env.get("STRICT") {
        Some(Expression::Boolean(b)) => b,
        _ => false,
    }
}
impl Expression {
    /// 交互命令入口
    pub fn eval_cmd(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        let strict = is_strict(env);
        let result = self.eval_mut(&mut State::new(strict), env, 0);
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
        let strict = is_strict(env);
        self.eval_mut(&mut State::new(strict), env, 0)
    }
    /// builtin args eval in pipe.
    pub fn eval_in_pipe(&self, env: &mut Environment) -> Result<Self, RuntimeError> {
        let strict = is_strict(env);
        let mut state = State::new(strict);
        state.set(State::IN_PIPE);
        self.eval_mut(&mut state, env, 0)
    }
    /// 求值主逻辑
    #[inline]
    pub fn eval_mut(
        &self,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        // dbg!("1.--->eval_mut:", &self, &self.type_name(), &state);
        if depth > MAX_RECURSION_DEPTH {
            return Err(RuntimeError::new(
                RuntimeErrorKind::RecursionDepth(self.clone()),
                self.clone(),
                depth,
            ));
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
                | Self::Range(..)
                | Self::FileSize(_)
                | Self::DateTime(_) => {
                    // dbg!("basic type");
                    return Ok(job.clone());
                }
                Self::Builtin(_) => {
                    // dbg!("builtin type");
                    return Ok(job.clone());
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

                    if state.contains(State::STRICT) {
                        return Ok(job.clone());
                    } else {
                        return match env.get(name) {
                            Some(expr) => Ok(expr),
                            None => Ok(job.clone()),
                        };
                    }
                }
                Self::StringTemplate(template) => {
                    return Ok(Expression::String(render_template(template, env)));
                }
                Self::Variable(name) => {
                    // dbg!("2.--->variable----", &name);
                    // var
                    return match env.get(name) {
                        Some(expr) => Ok(expr),
                        None => Err(RuntimeError::new(
                            RuntimeErrorKind::UndeclaredVariable(name.clone()),
                            self.clone(),
                            depth,
                        )),
                    };
                }

                // 处理变量声明（仅允许未定义变量）
                Self::Declare(name, expr) => {
                    // dbg!("declare---->", &name, &expr.type_name());

                    if state.contains(State::STRICT) && env.has(name)
                    // && env.get("STRICT") == Some(Expression::Boolean(true))
                    {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::Redeclaration(name.to_string()),
                            self.clone(),
                            depth,
                        ));
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
                        state.set(State::IN_ASSIGN);
                        let value = expr.as_ref().eval_mut(state, env, depth + 1)?;
                        state.clear(State::IN_ASSIGN);
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
                            !is_in_pipe
                        }
                        _ => false,
                    };
                    state.set(State::IN_ASSIGN);
                    let value = expr.as_ref().eval_mut(state, env, depth + 1)?;
                    state.clear(State::IN_ASSIGN);

                    // dbg!("assign---->", &name, &value.type_name());
                    if env.has(name) {
                        env.define(name, value.clone());
                    } else {
                        // 向上层环境查找并修改（根据语言设计需求）
                        // let mut current_env = env.clone();
                        // while let Some(parent) = current_env.get_parent_mut() {
                        //     if parent.has(name) {
                        //         parent.define(name, value.clone());
                        //         return Ok(value);
                        //     }
                        //     current_env = parent.clone();
                        // }

                        if state.contains(State::STRICT)
                        // && env.get("STRICT") == Some(Expression::Boolean(true))
                        {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::UndeclaredVariable(name.clone()),
                                self.clone(),
                                depth,
                            ));
                        }

                        env.define(name, value.clone());
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
                Self::AliasOp(name, expr) => {
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
                Self::Quote(inner) => return Ok(inner.as_ref().clone()),

                // 一元运算
                Self::UnaryOp(op, operand, is_prefix) => {
                    let operand_eval = operand.eval(env)?;
                    return match op.as_str() {
                        "!" => Ok(Expression::Boolean(!operand_eval.is_truthy())),
                        "-" => match operand_eval {
                            Expression::Integer(i) => Ok(Expression::Integer(-i)),
                            Expression::Float(i) => Ok(Expression::Float(-i)),
                            _ => {
                                return Err(RuntimeError::common(
                                    format!("Cannot apply Neg to {operand:?}:{operand_eval:?}")
                                        .into(),
                                    self.clone(),
                                    depth,
                                ));
                            }
                        },
                        // 处理 ++a 转换为 a = a + 1
                        "++" | "--" => {
                            // 确保操作数是符号
                            let var_name = operand.to_symbol()?;
                            // 获取当前值
                            let current_val = env.get(var_name).ok_or(RuntimeError::new(
                                RuntimeErrorKind::UndeclaredVariable(var_name.to_string()),
                                self.clone(),
                                depth,
                            ))?;

                            // 确保操作是合法的，例如整数或浮点数
                            if !matches!(current_val, Expression::Integer(_) | Expression::Float(_))
                            {
                                return Err(RuntimeError::common(
                                    format!("Cannot apply {op} to {operand:?}:{current_val:?}")
                                        .into(),
                                    self.clone(),
                                    depth,
                                ));
                            }
                            // 计算新值
                            let step = if op == "++" { 1 } else { -1 };
                            let new_val = (current_val.clone() + Expression::Integer(step))
                                .map_err(|e| RuntimeError::new(e, self.clone(), depth))?;
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

                            Err(RuntimeError::common(
                                format!("custom operation {op:?} not defined").into(),
                                self.clone(),
                                depth,
                            ))
                        }
                        _ => Err(RuntimeError::common(
                            format!("Unknown unary operator: {op}").into(),
                            self.clone(),
                            depth,
                        )),
                    };
                }
                // 特殊运算符

                // 二元运算
                Self::BinaryOp(operator, lhs, rhs) => {
                    return match operator.as_str() {
                        "+=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left += rhs.eval(env)?;
                                env.define(base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::common(
                                format!(
                                    "cannot apply {} to  {}:{} and {}:{}",
                                    operator,
                                    lhs,
                                    lhs.type_name(),
                                    rhs,
                                    rhs.type_name()
                                )
                                .into(),
                                self.clone(),
                                depth,
                            )),
                        },
                        "-=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left -= rhs.eval(env)?;
                                env.define(base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::common(
                                format!(
                                    "cannot apply {} to  {}:{} and {}:{}",
                                    operator,
                                    lhs,
                                    lhs.type_name(),
                                    rhs,
                                    rhs.type_name()
                                )
                                .into(),
                                self.clone(),
                                depth,
                            )),
                        },
                        "*=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                left *= rhs.eval(env)?;
                                env.define(base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::common(
                                format!(
                                    "cannot apply {} to  {}:{} and {}:{}",
                                    operator,
                                    lhs,
                                    lhs.type_name(),
                                    rhs,
                                    rhs.type_name()
                                )
                                .into(),
                                self.clone(),
                                depth,
                            )),
                        },
                        "/=" => match lhs.as_ref() {
                            Expression::Symbol(base) => {
                                let mut left = env.get(base).unwrap_or(Expression::Integer(0));
                                let right = rhs.eval(env)?;
                                if !right.is_truthy() {
                                    return Err(RuntimeError::common(
                                        format!("can't divide {} by zero", base).into(),
                                        self.clone(),
                                        depth,
                                    ));
                                };
                                left /= right;
                                env.define(base, left.clone());
                                Ok(left)
                            }
                            _ => Err(RuntimeError::common(
                                format!(
                                    "cannot apply {} to  {}:{} and {}:{}",
                                    operator,
                                    lhs,
                                    lhs.type_name(),
                                    rhs,
                                    rhs.type_name()
                                )
                                .into(),
                                self.clone(),
                                depth,
                            )),
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
                            return match operator.as_str() {
                                "+" => {
                                    (l + r).map_err(|e| RuntimeError::new(e, self.clone(), depth))
                                }
                                "-" => {
                                    (l - r).map_err(|e| RuntimeError::new(e, self.clone(), depth))
                                }
                                "*" => {
                                    (l * r).map_err(|e| RuntimeError::new(e, self.clone(), depth))
                                }
                                "/" => {
                                    (l / r).map_err(|e| RuntimeError::new(e, self.clone(), depth))
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
                                        // 确保 exponent 是非负的
                                        if exponent < 0 {
                                            return Err(RuntimeError::common(
                                                format!(
                                                    "cannot raise {} to a negative power {}",
                                                    base, exponent
                                                )
                                                .into(),
                                                self.clone(),
                                                depth,
                                            ));
                                        }

                                        // 使用 checked_pow 进行幂运算
                                        match base.checked_pow(exponent as u32) {
                                            Some(n) => Ok(n.into()),
                                            None => Err(RuntimeError::common(
                                                format!(
                                                    "overflow when raising int {} to the power {}",
                                                    base, exponent
                                                )
                                                .into(),
                                                self.clone(),
                                                depth,
                                            )),
                                        }
                                    }
                                    (a, b) => Err(RuntimeError::common(
                                        format!(
                                            "cannot raise {}:{} to the power {}:{}",
                                            a,
                                            a.type_name(),
                                            b,
                                            b.type_name()
                                        )
                                        .into(),
                                        self.clone(),
                                        depth,
                                    )),
                                },

                                "==" => Ok(Expression::Boolean(l == r)),
                                "~=" => Ok(Expression::Boolean(l.to_string() == r.to_string())),
                                "!=" => Ok(Expression::Boolean(l != r)),
                                ">" => Ok(Expression::Boolean(l > r)),
                                "<" => Ok(Expression::Boolean(l < r)),
                                ">=" => Ok(Expression::Boolean(l >= r)),
                                "<=" => Ok(Expression::Boolean(l <= r)),
                                "~:" => Ok(Expression::Boolean(handle_contains(l, r)?)),
                                "!~:" => Ok(Expression::Boolean(!handle_contains(l, r)?)),
                                "~~" => {
                                    let regex = Regex::new(&r.to_string()).map_err(|e| {
                                        RuntimeError::common(
                                            e.to_string().into(),
                                            self.clone(),
                                            depth,
                                        )
                                    })?;
                                    Ok(Expression::Boolean(regex.is_match(&l.to_string())))
                                }
                                "!~~" => {
                                    let regex = Regex::new(&r.to_string()).map_err(|e| {
                                        RuntimeError::common(
                                            e.to_string().into(),
                                            self.clone(),
                                            depth,
                                        )
                                    })?;
                                    Ok(Expression::Boolean(!regex.is_match(&l.to_string())))
                                }

                                op if op.starts_with("_") => {
                                    if let Some(oper) = env.get(op) {
                                        let rs =
                                            Expression::Apply(Rc::new(oper), Rc::new(vec![l, r]));
                                        return rs.eval_mut(state, env, depth + 1);
                                    }

                                    Err(RuntimeError::common(
                                        format!("custom operation {op:?} not defined",).into(),
                                        self.clone(),
                                        depth,
                                    ))
                                }
                                // ----------
                                _ => Err(RuntimeError::new(
                                    RuntimeErrorKind::InvalidOperator(operator.clone()),
                                    self.clone(),
                                    depth,
                                )),
                            };
                        }
                    };
                }
                // RangeOP
                Self::RangeOp(operator, lhs, rhs, step) => {
                    let l = lhs.as_ref().eval_mut(state, env, depth + 1)?;
                    let r = rhs.as_ref().eval_mut(state, env, depth + 1)?;
                    let st = match step {
                        Some(s) => match s.as_ref().eval_mut(state, env, depth + 1)? {
                            Expression::Integer(i) => i as usize,
                            other => {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::TypeError {
                                        expected: "Integer".to_owned(),
                                        sym: other.to_string(),
                                        found: other.type_name(),
                                    },
                                    self.clone(),
                                    depth,
                                ));
                            }
                        },
                        _ => 1,
                    };
                    return match operator.as_str() {
                        "...<" => match (l, r) {
                            (Expression::Integer(fr), Expression::Integer(t)) => {
                                let v = (fr..t)
                                    .step_by(st)
                                    .map(Expression::from) // 将 i64 转换为 Expression
                                    .collect::<Vec<_>>();
                                Ok(Expression::from(v))
                            }
                            _ => Err(RuntimeError::new(
                                RuntimeErrorKind::CustomError("not valid range option".into()),
                                self.clone(),
                                depth,
                            )),
                        },
                        "..." => match (l, r) {
                            (Expression::Integer(fr), Expression::Integer(t)) => {
                                let v = (fr..=t)
                                    .step_by(st)
                                    .map(Expression::from) // 将 i64 转换为 Expression
                                    .collect::<Vec<_>>();
                                Ok(Expression::from(v))
                            }
                            _ => Err(RuntimeError::new(
                                RuntimeErrorKind::CustomError("not valid range option".into()),
                                self.clone(),
                                depth,
                            )),
                        },
                        "..<" => match (l, r) {
                            (Expression::Integer(fr), Expression::Integer(t)) => {
                                Ok(Expression::Range(fr..t, st))
                            }
                            _ => Err(RuntimeError::new(
                                RuntimeErrorKind::CustomError("not valid range option".into()),
                                self.clone(),
                                depth,
                            )),
                        },
                        ".." => match (l, r) {
                            (Expression::Integer(fr), Expression::Integer(t)) => {
                                Ok(Expression::Range(fr..t + 1, st))
                            }
                            _ => Err(RuntimeError::new(
                                RuntimeErrorKind::CustomError("not valid range option".into()),
                                self.clone(),
                                depth,
                            )),
                        },
                        _ => Err(RuntimeError::new(
                            RuntimeErrorKind::InvalidOperator(operator.clone()),
                            self.clone(),
                            depth,
                        )),
                    };
                }
                // 管道
                Self::Pipe(operator, lhs, rhs) => {
                    match operator.as_str() {
                        "|" | "|_" | "|>" | "|^" => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.ensure_fn_apply();
                            let left_output = match left_func.eval_mut(state, env, depth + 1) {
                                Ok(r) => r,
                                Err(e) => {
                                    return match e.kind {
                                        RuntimeErrorKind::Terminated => Ok(Expression::None),
                                        _ => Err(e),
                                    };
                                }
                            };
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }
                            match operator.as_str() {
                                "|^" => {
                                    state.set(State::PTY_MODE);
                                    state.pipe_in(left_output);

                                    let r = rhs.ensure_fn_apply().eval_mut(state, env, depth + 1);
                                    state.clear(State::PTY_MODE);
                                    return r;
                                }

                                "|_" => {
                                    return rhs
                                        .as_ref()
                                        .ensure_fn_apply()
                                        .replace_or_append_arg(left_output)
                                        .eval_mut(state, env, depth + 1);
                                }
                                "|>" => match left_output {
                                    Expression::List(ls) => {
                                        return ls
                                            .iter()
                                            .map(|item| {
                                                rhs.as_ref()
                                                    .ensure_fn_apply()
                                                    .replace_or_append_arg(item.clone())
                                                    .eval_mut(state, env, depth + 1)
                                            })
                                            .collect::<Result<Vec<_>, _>>()
                                            .and_then(|r| Ok(Expression::from(r)));
                                    }
                                    Expression::String(strls) => {
                                        return ifs_split(&strls, env)
                                            .into_iter()
                                            .map(|item| {
                                                rhs.as_ref()
                                                    .ensure_fn_apply()
                                                    .replace_or_append_arg(Expression::String(item))
                                                    .eval_mut(state, env, depth + 1)
                                            })
                                            .collect::<Result<Vec<_>, _>>()
                                            .and_then(|r| Ok(Expression::from(r)));
                                    }
                                    _ => {
                                        return rhs
                                            .as_ref()
                                            .ensure_fn_apply()
                                            .replace_or_append_arg(left_output)
                                            .eval_mut(state, env, depth + 1);
                                    }
                                },
                                "|" => {
                                    return match rhs.as_ref() {
                                        Expression::PipeMethod(method, args) => self
                                            .eval_module_method(
                                                left_output.get_module_name(),
                                                method,
                                                args,
                                                left_output,
                                                state,
                                                env,
                                                depth,
                                            ),
                                        _ => {
                                            state.pipe_in(left_output);
                                            match rhs.as_ref() {
                                                Expression::Symbol(_) => {
                                                    return rhs.execute(vec![]).eval_mut(
                                                        state,
                                                        env,
                                                        depth + 1,
                                                    );
                                                }
                                                r => {
                                                    job = r;
                                                    continue;
                                                }
                                            }
                                        }
                                    };
                                }
                                _ => unreachable!(),
                            }
                        }

                        ">>" => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.as_ref().ensure_fn_apply();
                            let left_output = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }

                            let s = rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string();
                            let path = std::env::current_dir()
                                .map_err(|e| {
                                    RuntimeError::from_io_error(
                                        e,
                                        "get cwd".into(),
                                        self.clone(),
                                        depth,
                                    )
                                })?
                                .join(expand_home(s.as_str()).to_string());
                            if !path.exists() {
                                std::fs::File::create(path.clone()).map_err(|e| {
                                    RuntimeError::from_io_error(
                                        e,
                                        "create file".into(),
                                        self.clone(),
                                        depth,
                                    )
                                })?;
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
                                        Err(e) => Err(RuntimeError::from_io_error(
                                            e,
                                            "write bytes".into(),
                                            self.clone(),
                                            depth,
                                        )),
                                    };
                                }
                                Err(e) => {
                                    return Err(RuntimeError::from_io_error(
                                        e,
                                        "append file".into(),
                                        self.clone(),
                                        depth,
                                    ));

                                    // return Err(match e.kind() {
                                    //     ErrorKind::PermissionDenied => {
                                    //         RuntimeError::PermissionDenied(rhs.as_ref().clone())
                                    //     }
                                    //     _ => RuntimeError::CustomError(format!(
                                    //         "could not open file {}: {:?}",
                                    //         path.display(),
                                    //         e
                                    //     )),
                                    // });
                                }
                            }
                        }
                        ">!" => {
                            let is_in_pipe = state.contains(State::IN_PIPE);
                            state.set(State::IN_PIPE);
                            let left_func = lhs.as_ref().ensure_fn_apply();
                            let l = left_func.eval_mut(state, env, depth + 1)?;
                            if !is_in_pipe {
                                state.clear(State::IN_PIPE);
                            }

                            // dbg!("-->> left=", &l);
                            let s = rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string();
                            let path = std::env::current_dir()
                                .map_err(|e| {
                                    RuntimeError::from_io_error(
                                        e,
                                        "get cwd".into(),
                                        self.clone(),
                                        depth,
                                    )
                                })?
                                .join(expand_home(s.as_str()).to_string());
                            // If the contents are bytes, write the bytes directly to the file.
                            let result = if let Expression::Bytes(bytes) = l.clone() {
                                std::fs::write(path, bytes)
                            } else {
                                // Otherwise, convert the contents to a pretty string and write that.
                                std::fs::write(path, l.to_string())
                            };

                            return match result {
                                Ok(()) => Ok(l),

                                Err(e) => Err(RuntimeError::from_io_error(
                                    e,
                                    "write bytes".into(),
                                    self.clone(),
                                    depth,
                                )),
                                // Err(RuntimeError::CustomError(format!(
                                //     "could not write to file {}: {:?}",
                                //     rhs, e
                                // ))),
                            };
                        }
                        "<<" => {
                            // 输入重定向处理
                            // handle_stdin_redirect(lhs, rhs, state, env, depth, true)
                            let path = rhs.eval_mut(state, env, depth + 1)?;
                            let contents = std::fs::read_to_string(path.to_string())
                                .map(Self::String)
                                .map_err(|e| {
                                    RuntimeError::from_io_error(
                                        e,
                                        "read file".into(),
                                        self.clone(),
                                        depth,
                                    )
                                })?;

                            state.pipe_in(contents);

                            let left_func = lhs.ensure_fn_apply();
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
                Self::Slice(lis, slice_params) => {
                    let listo = lis.eval(env)?;
                    let start_int =
                        Expression::eval_to_int_opt(&slice_params.start, state, env, depth + 1)?;
                    let end_int =
                        Expression::eval_to_int_opt(&slice_params.end, state, env, depth + 1)?;
                    let step_int =
                        Expression::eval_to_int_opt(&slice_params.step, state, env, depth + 1)?
                            .unwrap_or(1); // 默认步长1

                    // return Self::slice(listo, start_int, end_int, step_int);
                    let list = listo
                        .as_list()
                        .map_err(|ek| RuntimeError::new(ek, self.clone(), depth))?;
                    let len = list.len() as Int;

                    let clamp = |v: Int| if v < 0 { len + v } else { v }.clamp(0, len - 1);

                    let (mut start, mut end) = (
                        start_int.map(clamp).unwrap_or(0),
                        end_int.map(|v| clamp(v).min(len)).unwrap_or(len),
                    );

                    if step_int < 0 {
                        (start, end) = (end.clamp(0, len), start.clamp(0, len));
                    }

                    let mut result = Vec::new();
                    let mut i = start;
                    while (step_int > 0 && i < end) || (step_int < 0 && i > end) {
                        if let Some(item) = list.get(i as usize) {
                            result.push(item.clone());
                        }
                        i += step_int;
                    }
                    return Ok(Self::from(result));
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
                        (left, right) => Ok(Self::index_slm(left, right)
                            .map_err(|ek| RuntimeError::new(ek, self.clone(), depth))?),
                    };
                }

                // 执行应用
                Self::Apply(func, args) => {
                    break self.eval_apply(func.as_ref(), args, state, env, depth + 1);
                }
                Self::Command(cmd, args) => {
                    // dbg!("====", &cmd, &args);
                    break self.eval_command(cmd.as_ref(), args.as_ref(), state, env, depth + 1);
                }
                // break Self::eval_command(self, env, depth+1),
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
                    for (pat, expr) in branches.as_ref().iter() {
                        if matches_pattern(&val, pat) {
                            job = expr;
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        continue;
                    }
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::NoMatchingBranch(val.to_string()),
                        self.clone(),
                        depth,
                    ));
                }
                Expression::Chain(base, calls) => {
                    return self.eval_chain(base, calls, state, env, depth + 1);
                }
                Expression::DestructureAssign(pattern, value) => {
                    let evaluated_value = value.eval_mut(state, env, depth + 1)?;
                    return self.destructure_assign(pattern, evaluated_value, env, depth + 1);
                }
                // Expression::PipeMethod(, )
                // 其他表达式处理...
                _ => break job.eval_flows(state, env, depth + 1),
            };
            // depth += 1
        }
    }
}

impl Expression {
    /// 索引访问
    fn index_slm(l: Expression, r: Expression) -> Result<Expression, RuntimeErrorKind> {
        match l {
            // 处理列表索引
            Expression::List(list) => {
                if let Expression::Integer(index) = r {
                    list.as_ref().get(index as usize).cloned().ok_or_else(|| {
                        RuntimeErrorKind::IndexOutOfBounds {
                            index: index as Int,
                            len: list.as_ref().len(),
                        }
                    })
                } else {
                    Err(RuntimeErrorKind::TypeError {
                        expected: "integer".into(),
                        sym: r.to_string(),
                        found: r.type_name(),
                    })
                }
            }
            // range
            Expression::Range(list, step) => {
                if let Expression::Integer(index) = r {
                    list.step_by(step)
                        .nth(index as usize)
                        .and_then(|r| Some(Expression::Integer(r)))
                        .ok_or_else(|| {
                            RuntimeErrorKind::CustomError(
                                format!("index {}: out of bounds", index).into(),
                            )
                        })
                } else {
                    Err(RuntimeErrorKind::TypeError {
                        expected: "integer".into(),
                        sym: r.to_string(),
                        found: r.type_name(),
                    })
                }
            }

            // 处理字典键访问
            Expression::HMap(map) => {
                let key = r.to_string(); // 自动转换Symbol/字符串
                map.as_ref()
                    .get(&key)
                    .cloned()
                    .ok_or(RuntimeErrorKind::KeyNotFound(key))
            }
            Expression::Map(map) => {
                let key = r.to_string(); // 自动转换Symbol/字符串
                map.as_ref()
                    .get(&key)
                    .cloned()
                    .ok_or(RuntimeErrorKind::KeyNotFound(key))
            }

            // 处理字符串索引
            Expression::String(s) => {
                if let Expression::Integer(index) = r {
                    s.chars()
                        .nth(index as usize)
                        .map(|c| Expression::String(c.to_string()))
                        .ok_or(RuntimeErrorKind::IndexOutOfBounds {
                            index: index as Int,
                            len: s.len(),
                        })
                } else {
                    Err(RuntimeErrorKind::TypeError {
                        expected: "integer".into(),
                        sym: r.to_string(),
                        found: r.type_name(),
                    })
                }
            }

            _ => Err(RuntimeErrorKind::TypeError {
                expected: "indexable type (List/Map/String)".into(),
                sym: l.to_string(),
                found: l.type_name(),
            }),
        }
    }

    pub fn as_list(&self) -> Result<&Vec<Expression>, RuntimeErrorKind> {
        match self {
            Self::List(v) => Ok(v.as_ref()),
            _ => Err(RuntimeErrorKind::TypeError {
                expected: "list".into(),
                sym: self.to_string(),
                found: self.type_name(),
            }),
        }
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
                let evaluated = boxed_expr.as_ref().eval_mut(state, env, depth + 1)?;

                // 转换为整数
                match evaluated {
                    Self::Integer(i) => Ok(Some(i)),
                    // 处理隐式类型转换
                    Self::Float(f) if f.fract() == 0.0 => Ok(Some(f as Int)),
                    // 处理其他类型错误
                    _ => Err(RuntimeError::new(
                        RuntimeErrorKind::TypeError {
                            expected: "integer".into(),
                            sym: evaluated.to_string(),
                            found: evaluated.type_name(),
                        },
                        boxed_expr.as_ref().clone(),
                        depth,
                    )),
                }
            }
        }
    }
}

impl Expression {
    // 列表追加示例（写时复制）
    pub fn list_push(&self, item: Self) -> Result<Expression, RuntimeErrorKind> {
        match self {
            Self::List(items) => {
                let mut new_vec = Vec::with_capacity(items.len() + 1);
                new_vec.extend_from_slice(items);
                new_vec.push(item);
                Ok(Self::List(Rc::new(new_vec)))
            }
            s => Err(RuntimeErrorKind::TypeError {
                expected: "List".into(),
                sym: s.to_string(),
                found: s.type_name(),
            }),
        }
    }
    pub fn list_append(&self, other: Rc<Vec<Expression>>) -> Result<Expression, RuntimeErrorKind> {
        match self {
            Self::List(items) => {
                let mut new_vec = Vec::with_capacity(items.len() + other.len());
                new_vec.extend_from_slice(items);
                new_vec.extend_from_slice(&other);
                Ok(Self::List(Rc::new(new_vec)))
            }
            s => Err(RuntimeErrorKind::TypeError {
                expected: "List".into(),
                sym: s.to_string(),
                found: s.type_name(),
            }),
        }
    }

    // 映射插入示例
    pub fn map_insert(&self, key: String, value: Self) -> Result<Expression, RuntimeErrorKind> {
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
            s => Err(RuntimeErrorKind::TypeError {
                expected: "Map".into(),
                sym: s.to_string(),
                found: s.type_name(),
            }),
        }
    }

    pub fn map_append(
        &self,
        other: Rc<HashMap<String, Expression>>,
    ) -> Result<Expression, RuntimeErrorKind> {
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
            s => Err(RuntimeErrorKind::TypeError {
                expected: "Map".into(),
                sym: s.to_string(),
                found: s.type_name(),
            }),
        }
    }
    pub fn bmap_append(
        &self,
        other: Rc<BTreeMap<String, Expression>>,
    ) -> Result<Expression, RuntimeErrorKind> {
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
            s => Err(RuntimeErrorKind::TypeError {
                expected: "Map".into(),
                sym: s.to_string(),
                found: s.type_name(),
            }),
        }
    }
}

/// match的比对
fn matches_pattern(value: &Expression, pattern: &Vec<Expression>) -> bool {
    pattern.iter().any(|pat| match pat {
        Expression::Symbol(s) if s == "_" => true,
        Expression::Symbol(s) => s == &value.to_string(),
        Expression::String(s) => {
            Regex::new(s).is_ok_and(|r| r.is_match(value.to_string().as_str()))
        }

        o => o == value,
    })
}

fn handle_contains(l: Expression, r: Expression) -> Result<bool, RuntimeError> {
    Ok(match l {
        Expression::String(left) => left.contains(&r.to_string()),
        Expression::Range(left, st) => {
            if let Expression::Integer(i) = r {
                match st {
                    1 => left.contains(&i),
                    _ => left.step_by(st).any(|f| f == i),
                }
            } else {
                false
            }
        }
        Expression::List(left) => left.contains(&r),
        Expression::Map(left) => left.contains_key(r.to_symbol()?),
        Expression::HMap(left) => left.contains_key(r.to_symbol()?),
        _ => false,
    })
}
