use super::Builtin;
use super::catcher::catch_error;
use super::eval::State;

use super::{Expression, Pattern};
use crate::expression::pipe_excutor::handle_command;
use crate::{Environment, RuntimeError};
use glob::glob;
use std::io::ErrorKind;
use std::io::Write;
use std::rc::Rc;

// Expression求值2
impl Expression {
    /// 处理复杂表达式的递归求值
    #[inline]
    pub fn eval_complex(
        &self,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        match self {
            // 控制流表达式
            Self::If(cond, true_expr, false_expr) => {
                handle_if(cond, true_expr, false_expr, state, env, depth)
            }

            Self::Match(value, branches) => {
                // 模式匹配求值
                let val = value.as_ref().eval_mut(state, env, depth + 1)?;
                for (pat, expr) in branches {
                    if matches_pattern(&val, pat, env)? {
                        return expr.as_ref().eval_mut(state, env, depth + 1);
                    }
                }
                Err(RuntimeError::NoMatchingBranch(val.to_string()))
            }

            Self::For(var, list_expr, body) => handle_for(var, list_expr, body, state, env, depth),

            Self::While(cond, body) => {
                // 循环求值直到条件为假
                let mut last = Self::None;
                while cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy() {
                    last = body.as_ref().eval_mut(state, env, depth + 1)?;
                    if let Self::Break(value) = last {
                        return Ok(value.as_ref().clone()); // 或者返回其他值
                    }
                }
                Ok(last)
            }
            Self::Loop(body) => {
                // 循环求值直到条件为假
                loop {
                    let last = body.as_ref().eval_mut(state, env, depth + 1);
                    match last {
                        Ok(_) => {} //继续
                        Err(RuntimeError::EarlyBreak(v)) => {
                            return Ok(v);
                        } // 捕获函数体内的return
                        Err(e) => return Err(e),
                    }
                    // if let Self::Break(value) = last {
                    //     return Ok(value.as_ref().clone()); // 或者返回其他值
                    // }
                }
            }
            // TODO add Loop,break
            //
            // 函数相关表达式

            // Self::Function(name, params, body, def_env) => {
            //     // 函数定义时捕获环境
            //     return Ok(Self::Function(name, params, body, def_env));
            // }
            // // Apply a function or macro to an argument
            // Lambda定义优化（自动捕获环境）
            // Self::Lambda(params, body, _) => {
            //     // 自动捕获当前环境
            //     Ok(Self::Lambda(params, body, env.fork()))
            // }
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
                        let r_func = rhs.ensure_apply();
                        let pipe_result = r_func.eval_mut(state, env, depth + 1);
                        // dbg!(&pipe_result);
                        pipe_result
                        // } else {
                        // let (pipe_out, expr_out) = handle_pipes(
                        //     lhs,
                        //     rhs,
                        //     // &bindings,
                        //     false,
                        //     None,
                        //     None,
                        //     state,
                        //     env,
                        //     depth,
                        //     always_pipe,
                        // )?;
                        // // dgb!(&expr_out);
                        // match expr_out {
                        //     Some(e) => Ok(e),
                        //     _ => Ok(to_expr(pipe_out)),
                        // }
                        // }
                    }

                    // {
                    //     // 管道运算符特殊处理
                    //     dbg!("--pipe--", &lhs, &rhs);
                    //     // dbg!("--pipe--");
                    //     let left_func = lhs.ensure_apply();
                    //     let left_output = left_func.eval_mut(true,env, depth + 1)?;
                    //     let mut new_env = env.fork();
                    //     new_env.define("__stdin", left_output);

                    //     let r_func = rhs.ensure_apply();
                    //     let pipe_result = r_func.eval_mut(&mut new_env, depth + 1);
                    //     // dbg!(&pipe_result);
                    //     pipe_result
                    // }
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
                        rhs.as_ref()
                            .append_args(args)
                            .eval_mut(state, env, depth + 1)
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
                        path = path.join(rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string());
                        if !path.exists() {
                            std::fs::File::create(path.clone())?;
                        }
                        match std::fs::OpenOptions::new().append(true).open(&path) {
                            Ok(mut file) => {
                                // use std::io::prelude::*;
                                let result = if let Expression::Bytes(bytes) = left_output.clone() {
                                    // std::fs::write(path, bytes)
                                    file.write_all(&bytes)
                                } else {
                                    // Otherwise, convert the contents to a pretty string and write that.
                                    // std::fs::write(path, contents.to_string())
                                    file.write_all(left_output.to_string().as_bytes())
                                };

                                match result {
                                    Ok(()) => Ok(left_output),
                                    Err(e) => Err(RuntimeError::CustomError(format!(
                                        "could not append to file {}: {:?}",
                                        rhs, e
                                    ))),
                                }
                            }
                            Err(e) => Err(match e.kind() {
                                ErrorKind::PermissionDenied => {
                                    RuntimeError::PermissionDenied(rhs.as_ref().clone())
                                }
                                _ => RuntimeError::CustomError(format!(
                                    "could not open file {}: {:?}",
                                    path.display(),
                                    e
                                )),
                            }),
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
                        path = path.join(rhs.as_ref().eval_mut(state, env, depth + 1)?.to_string());
                        // If the contents are bytes, write the bytes directly to the file.
                        let result = if let Expression::Bytes(bytes) = l.clone() {
                            std::fs::write(path, bytes)
                        } else {
                            // Otherwise, convert the contents to a pretty string and write that.
                            std::fs::write(path, l.to_string())
                        };

                        match result {
                            Ok(()) => Ok(l),
                            Err(e) => Err(RuntimeError::CustomError(format!(
                                "could not write to file {}: {:?}",
                                rhs, e
                            ))),
                        }
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
    /// 执行
    pub fn eval_apply(
        &self,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Self, RuntimeError> {
        // 函数应用
        match self {
            Self::Apply(func, args) | Self::Command(func, args) => {
                // dbg!("2.--->Applying:", &self, &self.type_name(), &func, &args);

                // 递归求值函数和参数
                let func_eval = func.as_ref().eval_mut(state, env, depth + 1)?;
                // let args_eval = args
                //     .into_iter()
                //     .map(|a| a.eval_mut(true,env, depth + 1))
                //     .collect::<Result<Vec<_>, _>>()?;
                // let func_eval = *func.clone();

                // 分派到具体类型处理
                match func_eval {
                    // | Self::String(name)
                    Self::Symbol(name) | Self::String(name) => {
                        // Apply as Command
                        //dbg!("   3.--->applying symbol as Command:", &name);
                        handle_command(&name, args, state, env, depth)
                        // let bindings = env.get_bindings_map();

                        // let mut cmd_args = vec![];
                        // for arg in args {
                        //     for flattened_arg in
                        //         Self::flatten(vec![arg.eval_mut(env, depth + 1)?])
                        //     {
                        //         match flattened_arg {
                        //             Self::String(s) => cmd_args.push(s),
                        //             Self::Bytes(b) => {
                        //                 cmd_args.push(String::from_utf8_lossy(&b).to_string())
                        //             }
                        //             Self::None => continue,
                        //             _ => cmd_args.push(format!("{}", flattened_arg)),
                        //         }
                        //     }
                        // }

                        // let always_pipe = env.has("__ALWAYSPIPE");
                        // if always_pipe {
                        //     let output = Command::new(&name)
                        //         .current_dir(env.get_cwd())
                        //         .args(
                        //             cmd_args, // Self::flatten(args.clone()).iter()
                        //                      //     .filter(|&x| x != &Self::None)
                        //                      //     // .map(|x| Ok(format!("{}", x.eval_mut(env, depth + 1)?)))
                        //                      //     .collect::<Result<Vec<String>, Error>>()?,
                        //         )
                        //         .envs(bindings)
                        //         .output();

                        //     match output {
                        //         Ok(result) => {
                        //             // 检查命令是否成功执行
                        //             if result.status.success() {
                        //                 // 将标准输出转换为字符串并打印
                        //                 let stdout = String::from_utf8_lossy(&result.stdout);
                        //                 // println!("Command output:\n{}", stdout);
                        //                 return Ok(Expression::String(stdout.into_owned()));
                        //             } else {
                        //                 // 如果命令执行失败，打印错误信息
                        //                 let stderr = String::from_utf8_lossy(&result.stderr);
                        //                 // eprintln!("Command failed with error:\n{}", &stderr);
                        //                 return Err(RuntimeError::CustomError(format!(
                        //                     "{} command failed with error:\n{}",
                        //                     name, stderr,
                        //                 )));
                        //             }
                        //         }
                        //         Err(e) => {
                        //             return Err(match e.kind() {
                        //                 ErrorKind::NotFound => RuntimeError::ProgramNotFound(name),
                        //                 ErrorKind::PermissionDenied => {
                        //                     RuntimeError::PermissionDenied(self.clone())
                        //                 }
                        //                 _ => RuntimeError::CommandFailed(name, args.clone()),
                        //             });
                        //         }
                        //     }
                        // } else {
                        //     let mut child = Command::new(&name)
                        //         .current_dir(env.get_cwd())
                        //         .args(cmd_args)
                        //         .envs(bindings)
                        //         .stdin(Stdio::inherit()) // 继承标准输入
                        //         .stdout(Stdio::inherit()) // 继承标准输出
                        //         .stderr(Stdio::inherit()) // 继承标准错误
                        //         .spawn()
                        //         .map_err(|e| match e.kind() {
                        //             ErrorKind::NotFound => {
                        //                 RuntimeError::ProgramNotFound(name.to_string())
                        //             }
                        //             ErrorKind::PermissionDenied => {
                        //                 RuntimeError::PermissionDenied(self.clone())
                        //             }
                        //             _ => {
                        //                 RuntimeError::CommandFailed(name.to_string(), args.clone())
                        //             }
                        //         })?;
                        //     child.wait().map_err(|e| {
                        //         RuntimeError::CommandFailed2(name.to_string(), e.to_string())
                        //     })?;

                        //     return Ok(Expression::None);
                        // }
                    }

                    // Self::Builtin(builtin) => (builtin.body)(args_eval, env),
                    Self::Builtin(Builtin { body, .. }) => {
                        // dbg!("   3.--->applying Builtin:", &func, &args);

                        let exe = match state.pipe_out() {
                            Some(p) => {
                                let mut na = args.as_ref().clone();
                                na.push(p);
                                body(&na, env)
                            }
                            _ => body(args.as_ref(), env),
                        };

                        match exe {
                            Ok(result) => {
                                self.set_status_code(0, env);
                                // dbg!(&result);
                                Ok(result)
                            }
                            Err(e) => {
                                self.set_status_code(1, env);
                                Err(RuntimeError::CommandFailed2(
                                    func.to_string(),
                                    e.to_string(),
                                ))
                            }
                        }
                    }
                    // Lambda 应用 - 完全求值的函数应用
                    Self::Lambda(params, body) => {
                        let pipe_out = state.pipe_out(); //必须先取得pipeout，否则可能被参数取走
                        // dbg!("2.--- applying lambda---", &params);
                        let mut current_env = env.fork();

                        // 批量参数绑定前先求值所有参数
                        let mut evaluated_args = args
                            .iter()
                            .map(|arg| arg.eval_mut(state, env, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?;

                        match pipe_out {
                            Some(p) => {
                                evaluated_args.push(p);
                            }
                            _ => {}
                        };

                        match bind_arguments(&params, &evaluated_args, &mut current_env) {
                            // 完全应用：求值函数体
                            None => {
                                let result =
                                    body.as_ref().eval_mut(state, &mut current_env, depth + 1);
                                match result {
                                    Ok(v) => {
                                        self.set_status_code(0, env);
                                        Ok(v)
                                    }
                                    Err(RuntimeError::EarlyReturn(v)) => {
                                        self.set_status_code(0, env);
                                        Ok(v)
                                    } // 捕获函数体内的return
                                    Err(e) => {
                                        self.set_status_code(1, env);
                                        Err(e)
                                    }
                                }
                            }

                            // 部分应用：返回新的柯里化lambda
                            Some(remain) => Ok(Self::Lambda(remain, body)),
                        }
                    }

                    // Macro 应用 - 不自动求值参数的展开
                    // Self::Macro(params, body) => {
                    //     match bind_arguments(params, args.to_owned(), env) {
                    //         // 完全应用：求值函数体
                    //         None => body.eval_mut(true, env, depth + 1),

                    //         // 部分应用：返回新的柯里化lambda
                    //         Some(remain) => Ok(Self::Macro(remain, body)),
                    //     }
                    // }
                    Self::Function(name, params, pc, body) => {
                        // dbg!("2.--- applying function---", &name, &params);
                        // dbg!(&def_env);
                        // 参数数量校验
                        let pipe_out = state.pipe_out(); //必须先取得pipeout，否则可能被参数取走
                        let pipe_arg_len = match pipe_out {
                            Some(_) => 1,
                            _ => 0,
                        };

                        if pc.is_none() && args.len() + pipe_arg_len > params.len() {
                            return Err(RuntimeError::TooManyArguments {
                                name,
                                max: params.len(),
                                received: args.len(),
                            });
                        }

                        let mut actual_args = args
                            .as_ref()
                            .iter()
                            .map(|a| a.eval_mut(state, env, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?;

                        match pipe_out {
                            Some(p) => {
                                actual_args.push(p);
                            }
                            _ => {}
                        };

                        // 填充默认值逻辑（新增）
                        for (i, (_, default)) in params.iter().enumerate() {
                            if i >= actual_args.len() {
                                if let Some(def_expr) = default {
                                    // 仅允许基本类型直接使用
                                    actual_args.push(def_expr.clone());
                                } else {
                                    return Err(RuntimeError::ArgumentMismatch {
                                        name,
                                        expected: params.len(),
                                        received: actual_args.len(),
                                    });
                                }
                            }
                        }

                        // 创建新作用域并执行
                        let mut new_env = env.fork();
                        if let Some(collector) = pc {
                            new_env.define(
                                collector.as_str(),
                                Expression::from(actual_args[params.len()..].to_vec()),
                            );
                        }
                        for ((param, _), arg) in params.iter().zip(actual_args) {
                            new_env.define(param, arg);
                        }
                        // body env
                        // for symbol in body.get_used_symbols() {
                        //     if !def_env.is_defined(&symbol) {
                        //         if let Some(val) = env.get(&symbol) {
                        //             new_env.define(&symbol, val)
                        //         }
                        //     }
                        // }
                        // dbg!(&new_env);
                        match body.as_ref().eval_mut(state, &mut new_env, depth + 1) {
                            Ok(v) => {
                                self.set_status_code(0, env);
                                Ok(v)
                            }
                            Err(RuntimeError::EarlyReturn(v)) => {
                                self.set_status_code(0, env);

                                Ok(v)
                            } // 捕获函数体内的return
                            Err(e) => {
                                self.set_status_code(1, env);
                                Err(e)
                            }
                        }
                    }
                    _ => Err(RuntimeError::CannotApply(
                        func.as_ref().clone(),
                        args.as_ref().clone(),
                    )),
                }
            }
            _ => Err(RuntimeError::CustomError(self.to_string())), // unreachable!(),
        }
    }
}

/// 参数绑定辅助函数 - 将参数绑定到环境中
///
/// # 参数
/// - `params`: 形式参数列表
/// - `args`: 实际参数列表(已求值)
/// - `target_env`: 目标绑定环境(通常是新创建的lambda环境)
/// - `depth`: 当前求值深度(用于错误报告)
///
/// # 返回值
/// 返回元组: (剩余未绑定的形式参数)
pub fn bind_arguments(
    params: &Vec<String>,
    args: &Vec<Expression>,
    target_env: &mut Environment,
) -> Option<Vec<String>> {
    // 计算实际能绑定的参数数量
    let bound_count = params.len().min(args.len());
    // 绑定参数到目标环境
    for (param, arg) in params.iter().zip(args.iter().take(bound_count)) {
        target_env.define(param, arg.clone());
    }
    // 获取剩余参数
    if bound_count < params.len() {
        Some(params[bound_count..].to_vec())
    } else {
        None
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

#[inline]
fn handle_if(
    cond: &Rc<Expression>,
    true_expr: &Rc<Expression>,
    false_expr: &Rc<Expression>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    // 条件分支求值
    if cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy() {
        true_expr.as_ref().eval_mut(state, env, depth + 1)
    } else {
        false_expr.as_ref().eval_mut(state, env, depth + 1)
    }
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
        Expression::List(elist) => {
            let mut result = Vec::with_capacity(elist.len());
            for item in elist.iter() {
                env.define(var, item.clone());
                let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                result.push(last)
            }
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
                if elist.len() < 1 {
                    return Err(RuntimeError::WildcardNotMatched(s));
                }
                // loop
                let mut result = Vec::with_capacity(elist.len());
                for item in elist.iter() {
                    env.define(var, Expression::String(item.clone()));
                    let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                    result.push(last)
                }
                Ok(Expression::from(result))
            } else {
                let elist = s.chars();
                let mut result = Vec::with_capacity(s.len());
                for item in elist {
                    env.define(var, Expression::String(item.to_string()));
                    let last = body.as_ref().eval_mut(state, env, depth + 1)?;
                    result.push(last)
                }
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
