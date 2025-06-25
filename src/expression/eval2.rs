use super::catcher::catch_error;
use super::eval::State;
use crate::{Environment, Expression, RuntimeError};
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
                handle_for(var, list_expr, body, state, env, depth + 1)
            }

            Self::While(cond, body) => {
                // 循环求值直到条件为假
                let mut last = Ok(Expression::None);
                while cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy() {
                    last = body.as_ref().eval_mut(state, env, depth + 1);
                    match last {
                        Ok(_) => {} //继续
                        Err(RuntimeError::EarlyBreak(v)) => {
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
                        Err(RuntimeError::EarlyBreak(v)) => {
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
                                return Err(RuntimeError::InvalidDefaultValue(
                                    name.clone(),
                                    p.to_string(),
                                    expr.clone(),
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
                Err(RuntimeError::EarlyReturn(v))
            }
            Self::Break(expr) => {
                // 提前返回机制
                let v = expr.as_ref().eval_mut(state, env, depth + 1)?;
                // Ok(Self::Break(Rc::new(v)))
                Err(RuntimeError::EarlyBreak(v))
            }

            Self::Catch(body, typ, deeling) => {
                // dbg!(&typ, &deeling);
                let result = body.as_ref().eval_mut(state, env, depth + 1);
                match result {
                    Ok(result) => Ok(result),
                    Err(e) => catch_error(e, body, typ, deeling, state, env, depth + 1),
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
}

#[inline]
fn handle_for(
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
        Expression::Range(elist, step) => {
            let mut result = Vec::with_capacity((elist.end as usize - elist.start as usize) / step);
            for item in elist.step_by(step) {
                env.define(var, Expression::Integer(item));
                let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                result.push(last)
            }
            result.retain(|r| r != &Expression::None);
            Ok(Expression::from(result))
        }
        Expression::List(elist) => {
            let mut result = Vec::with_capacity(elist.len());
            for item in elist.iter() {
                env.define(var, item.clone());
                let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                result.push(last)
            }
            result.retain(|r| r != &Expression::None);
            Ok(Expression::from(result))
        }
        Expression::String(mut s) => {
            if s.starts_with("~") {
                if let Some(home_dir) = dirs::home_dir() {
                    s = s.replace("~", home_dir.to_string_lossy().as_ref());
                }
            }
            if s.contains('*') {
                let mut elist = vec![];
                for path in glob(&s).unwrap().filter_map(Result::ok) {
                    elist.push(path.to_string_lossy().to_string());
                }
                if elist.is_empty() {
                    return Err(RuntimeError::WildcardNotMatched(s));
                }
                // loop
                let mut result = Vec::with_capacity(elist.len());
                for item in elist.into_iter() {
                    env.define(var, Expression::String(item));
                    let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                    result.push(last)
                }
                result.retain(|r| r != &Expression::None);
                Ok(Expression::from(result))
            } else {
                let ifs = env.get("IFS");
                let slist = match ifs {
                    Some(Expression::String(fs)) => {
                        s.split_terminator(fs.as_str()).collect::<Vec<_>>()
                    }
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
                        elist
                    }
                };
                let mut result = slist
                    .into_iter()
                    .map(|i| {
                        env.define(var, Expression::String(i.to_string()));
                        body.as_ref().eval_mut(state, env, depth + 1)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                result.retain(|r| r != &Expression::None);
                Ok(Expression::from(result))
            }
        }
        _ => Err(RuntimeError::ForNonList(list_excuted)),
    }
    // 遍历每个元素执行循环体
    // let mut result = Vec::with_capacity(elist.len());
    // for item in elist.iter() {
    //     env.define(var, item.clone());
    //     let last = body.as_ref().eval_mut(state, env, depth + 1)?;
    //     result.push(last)
    // }
    // Ok(Expression::from(result))
    // let r: Result<Vec<Expression>, RuntimeError> = list
    //     .iter()
    //     .map(|item| {
    //         env.define(var, item.clone());
    //         body.as_ref().eval_mut(true, env, depth + 1)
    //     })
    //     .collect();
    // r.map(Expression::from)
}
