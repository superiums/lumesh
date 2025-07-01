use super::catcher::catch_error;
use super::eval::State;
use crate::{Environment, Expression, RuntimeError, RuntimeErrorKind};
use glob::glob;
use std::rc::Rc;

// Expression求值2
impl Expression {
    /// 处理复杂表达式的递归求值
    #[inline]
    pub fn eval_flows(
        &self,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        match self {
            Self::For(var, list_expr, body) => {
                self.handle_for(var, list_expr, body, state, env, depth + 1)
            }

            Self::While(cond, body) => {
                // 循环求值直到条件为假
                let mut last = Ok(Expression::None);
                while cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy() {
                    last = body.as_ref().eval_mut(state, env, depth + 1);
                    match last {
                        Ok(_) => {} //继续
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::EarlyBreak(v),
                            context: _,
                            depth: _,
                        }) => {
                            return Ok(v);
                        } // 捕获函数体内的return
                        Err(e) => return Err(e),
                    }
                }
                last
            }
            Self::Loop(body) => {
                loop {
                    let last = body.as_ref().eval_mut(state, env, depth + 1);
                    // dbg!(&last);
                    match last {
                        Ok(_) => {} //继续
                        Err(RuntimeError {
                            kind: RuntimeErrorKind::EarlyBreak(v),
                            context: _,
                            depth: _,
                        }) => {
                            return Ok(v);
                        } // 捕获函数体内的return
                        Err(e) => return Err(e),
                    }
                }
            }

            // 处理函数定义
            Self::Function(name, params, pc, body) => {
                // dbg!(&def_env);
                // 验证默认值类型（新增）
                for (p, default) in params {
                    if let Some(expr) = default {
                        match expr {
                            Expression::String(_)
                            | Expression::Integer(_)
                            | Expression::Float(_)
                            | Expression::Boolean(_) => {}
                            _ => {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::InvalidDefaultValue(
                                        name.clone(),
                                        p.to_string(),
                                        expr.clone(),
                                    ),
                                    self.clone(),
                                    depth,
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
                let func = Self::Function(name.clone(), params.clone(), pc.clone(), body.clone());
                env.define(name, func.clone());
                Ok(func)
            }
            // Self::Macro(param, body) => {
            //     // 宏定义保持未求值状态
            //     Ok(Self::Macro(param, body))
            // }

            // 块表达式
            Self::Do(exprs) => {
                // dbg!("2.--->DoBlock:", &exprs);
                // 创建子环境继承父作用域
                // let mut child_env = env.clone();
                // 顺序求值语句块
                let mut last = Self::None;
                for expr in exprs.as_ref() {
                    last = expr.eval_mut(state, env, depth + 1)?;
                }
                Ok(last)
            }

            Self::Return(expr) => {
                // 提前返回机制
                let v = expr.as_ref().eval_mut(state, env, depth + 1)?;
                // Ok(Self::Return(Rc::new(v)))
                Err(RuntimeError::new(
                    RuntimeErrorKind::EarlyReturn(v),
                    Expression::None,
                    depth,
                ))
            }
            Self::Break(expr) => {
                // 提前返回机制
                let v = expr.as_ref().eval_mut(state, env, depth + 1)?;
                // Ok(Self::Break(Rc::new(v)))
                Err(RuntimeError::new(
                    RuntimeErrorKind::EarlyBreak(v),
                    Expression::None,
                    depth,
                ))
            }

            Self::Catch(body, typ, deeling) => {
                // dbg!(&typ, &deeling);
                let result = body.as_ref().eval_mut(state, env, depth + 1);
                match result {
                    Ok(result) => Ok(result),
                    Err(e) => catch_error(e, typ, deeling, state, env, depth + 1),
                }
            }
            // 默认情况
            _ => {
                //dbg!("2.--->Default:", &self);
                Ok(self.clone())
            } // 基本类型已在 eval_mut 处理
        }
    }

    // }

    #[inline]
    fn handle_for(
        &self,
        var: &String,
        list_expr: &Rc<Expression>,
        body: &Rc<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // 求值列表表达式
        let list_excuted = list_expr.as_ref().eval_mut(state, env, depth + 1)?;
        // .as_list()?;
        match list_excuted {
            Expression::Range(range, step) => {
                let iterator = range.step_by(step).map(Expression::Integer);
                execute_iteration(var, iterator, body, state, env, depth)
            }
            Expression::List(items) => {
                let iterator = items.iter().cloned();
                execute_iteration(var, iterator, body, state, env, depth)
            }
            Expression::String(s) => {
                let iterator = if s.contains('*') {
                    // glob expansion logic
                    glob_expand(&s).into_iter().map(Expression::String)
                } else {
                    // IFS splitting logic
                    ifs_split(&s, env).into_iter().map(Expression::String)
                };
                execute_iteration(var, iterator, body, state, env, depth)
            }
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::ForNonList(list_excuted),
                self.clone(),
                depth,
            )),
        }
    }
}

fn glob_expand(s: &str) -> Vec<String> {
    let mut elist = vec![];
    for entry in glob(&s).unwrap() {
        match entry {
            Ok(p) => elist.push(p.to_string_lossy().to_string()),
            _ => {}
        }
    }
    elist
}
fn ifs_split(s: &str, env: &mut Environment) -> Vec<String> {
    let ifs = env.get("IFS");
    match ifs {
        Some(Expression::String(fs)) => s
            .split_terminator(fs.as_str())
            .map(|v| v.to_string())
            .collect::<Vec<_>>(),
        _ => {
            let mut elist = s.lines().collect::<Vec<_>>();
            if elist.len() < 2 {
                elist = s.split_ascii_whitespace().collect::<Vec<_>>();
                if elist.len() < 2 {
                    elist = s.split_terminator(";").collect::<Vec<_>>();
                    if elist.len() < 2 {
                        elist = s.split_terminator(",").collect::<Vec<_>>();
                    }
                }
            }
            elist.iter().map(|v| v.to_string()).collect::<Vec<_>>()
        }
    }
}
fn execute_iteration<I>(
    var: &String,
    iterator: I,
    body: &Rc<Expression>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError>
where
    I: Iterator<Item = Expression>,
{
    if state.contains(State::IN_ASSIGN) {
        let mut results = Vec::new();

        for item in iterator {
            env.define(var, item);
            match body.as_ref().eval_mut(state, env, depth) {
                Ok(result) => {
                    // if !matches!(result, Expression::None) {
                    results.push(result);
                    // }
                }
                Err(RuntimeError {
                    kind: RuntimeErrorKind::EarlyBreak(v),
                    ..
                }) => return Ok(v),
                Err(e) => return Err(e),
            }
        }
        Ok(Expression::from(results))
    } else {
        for item in iterator {
            env.define(var, item);
            match body.as_ref().eval_mut(state, env, depth) {
                Ok(_) => {}
                Err(RuntimeError {
                    kind: RuntimeErrorKind::EarlyBreak(v),
                    ..
                }) => return Ok(v),
                Err(e) => return Err(e),
            }
        }
        Ok(Expression::None)
    }
}
