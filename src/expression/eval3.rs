use std::borrow::Cow;

use super::eval::State;
use crate::expression::cmd_excutor::handle_command;
use crate::expression::{ChainCall, alias};
use crate::libs::{get_builtin_via_expr, is_lib};
use crate::{Environment, Expression, MAX_RUNTIME_RECURSION, RuntimeError, RuntimeErrorKind};

// 需要延迟解析的特殊命令列表
const LAZY_EVAL_COMMANDS: &[&str] = &["where", "repeat", "debug", "ddebug", "typeof"];

pub fn prepare_args<'a>(
    cmd: &str,
    args: &'a [Expression],
    check_lazy: bool,
    insert_arg: Option<Expression>,
    env: &mut Environment,
    state: &mut State,
    depth: usize,
) -> Result<Cow<'a, [Expression]>, RuntimeError> {
    if check_lazy {
        if LAZY_EVAL_COMMANDS.contains(&cmd) {
            if args.contains(&Expression::Blank) {
                return Ok(Cow::Owned(
                    args.iter()
                        .map(|x| match x {
                            Expression::Blank => {
                                x.eval_mut(state, env, depth).unwrap_or(Expression::Blank)
                            }
                            other => other.clone(),
                        })
                        .collect::<Vec<_>>(),
                ));
            }
            return Ok(Cow::Borrowed(args));
        }
    }
    let mut args_eval = if let Some(a) = insert_arg {
        let mut vec = Vec::with_capacity(args.len() + 1);
        vec.push(a);
        vec
    } else {
        Vec::with_capacity(args.len())
    };
    for arg in args.iter() {
        match arg.eval_mut(state, env, depth) {
            Ok(a) => args_eval.push(a),
            Err(e) => return Err(e),
        }
    }
    Ok(Cow::Owned(args_eval))
}

/// 执行
impl Expression {
    // 函数应用
    #[inline]
    pub fn eval_normal_function(
        &self,
        func: Expression,
        args: &[Expression],
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match func {
            Expression::Function(name, params, pc, body, _decos) => {
                // 1. 先检查参数数量上限
                if pc.is_none() && args.len() > params.len() {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::TooManyArguments {
                            name,
                            max: params.len(),
                            received: args.len(),
                        },
                        self.clone(),
                        depth,
                    ));
                }

                // 2. 求值实际参数
                let is_in_pipe = state.contains(State::IN_ASSIGN);
                state.set(State::IN_ASSIGN);
                let mut actual_args = args
                    .iter()
                    .map(|a| a.eval_mut(state, env, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?;
                if !is_in_pipe {
                    state.clear(State::IN_ASSIGN);
                }

                // 3. 填充默认值（修正后）
                for (i, (_, default)) in params.iter().enumerate() {
                    if i >= actual_args.len() {
                        if let Some(def_expr) = default {
                            actual_args.push(def_expr.clone());
                        } else {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::ArgumentMismatch {
                                    name,
                                    expected: params.len(),
                                    received: actual_args.len(),
                                },
                                self.clone(),
                                depth,
                            ));
                        }
                    }
                }

                // 4. 创建新作用域
                let new_env = match state.contains(State::IN_DECO) {
                    true => env,
                    _ => &mut env.fork(),
                };

                // 5. 正确处理 collector
                if let Some(collector) = pc {
                    // collector 获取剩余参数
                    new_env.define(
                        collector.as_str(),
                        Expression::from(actual_args[params.len()..].to_vec()),
                    );
                }

                // 6. 绑定正式参数（只绑定定义的参数）
                for ((param, _), arg) in params.iter().zip(actual_args.iter().take(params.len())) {
                    new_env.define(param, arg.clone());
                }

                // 执行函数体
                match body.as_ref().eval_mut(state, new_env, depth + 1) {
                    Ok(v) => Ok(v),
                    Err(RuntimeError {
                        kind: RuntimeErrorKind::EarlyReturn(v),
                        context: _,
                        depth: _,
                    }) => Ok(v),
                    Err(e) => Err(e),
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn eval_function_with_deco(
        &self,
        func: Expression,
        args: &[Expression],
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match &func {
            Expression::Function(name, _params, _pc, _body, decos) => {
                let mut decoders = Vec::with_capacity(decos.len());
                for deco in decos.iter() {
                    let deco_fo = env.get(deco.0.as_str());
                    if let Some(deco_fn) = deco_fo {
                        if matches!(deco_fn, Expression::Function(..) | Expression::Lambda(..)) {
                            let deco_args = deco.1.clone().unwrap_or(vec![]);
                            // dbg!("deco is func", &deco_fn, &deco_args, &last_fn);
                            let wrapper =
                                deco_fn.eval_apply(&deco_fn, &deco_args, state, env, depth + 1)?;
                            let item = match wrapper {
                                Expression::List(list) if list.len() == 2 => list,
                                _ => {
                                    return Err(RuntimeError::common(
                                        "decoder not return a [before,after] list".into(),
                                        wrapper,
                                        depth,
                                    ));
                                }
                            };
                            decoders.push(item);
                        } else {
                            return Err(RuntimeError::common(
                                "trying to apply non-function as decorator".into(),
                                deco_fn,
                                depth,
                            ));
                        }
                    } else {
                        return Err(RuntimeError::common(
                            format!("decorator `{}` not defined", deco.0).into(),
                            self.clone(),
                            depth,
                        ));
                    }
                }

                // 装饰器的总环境
                let mut env_deco = Environment::new();
                env_deco.define("NAME", Expression::String(name.to_string()));
                env_deco.define("ARGS", Expression::from(args.to_vec()));

                // 为每个装饰器创建独立环境
                // 每个装饰器的before，after共享一个单独的环境
                // 用IN_DECO状态指示以后不再fork
                let mut env_stack = Vec::new();
                for _ in 0..decoders.len() {
                    env_stack.push(env_deco.fork());
                }

                // 执行 before 函数
                state.set(State::IN_DECO);
                for (i, decoder) in decoders.iter().enumerate() {
                    let before = decoder.get(0).unwrap();
                    if !matches!(before, &Expression::None | &Expression::Blank) {
                        before.eval_apply(before, &vec![], state, &mut env_stack[i], depth)?;
                    }
                }
                state.clear(State::IN_DECO);

                // 执行原函数
                let result = self.eval_normal_function(func, args, state, env, depth)?;

                // 执行 after 函数（逆序，使用对应环境）
                state.set(State::IN_DECO);
                for (i, decoder) in decoders.iter().rev().enumerate() {
                    let after = decoder.get(1).unwrap();
                    if !matches!(after, &Expression::None | &Expression::Blank) {
                        let env_idx = decoders.len() - 1 - i;
                        env_stack[env_idx].define("RESULT", result.clone());
                        after.eval_apply(after, &vec![], state, &mut env_stack[env_idx], depth)?;
                    }
                }
                state.clear(State::IN_DECO);

                Ok(result)
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn eval_apply(
        &self,
        func: &Expression,
        args: &[Expression],
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        //防止函数互相调用无限循环
        if MAX_RUNTIME_RECURSION.with(|v| depth > *v.borrow()) {
            return Err(RuntimeError::new(
                RuntimeErrorKind::RecursionDepth(self.clone()),
                self.clone(),
                depth,
            ));
        }
        // println!();
        // dbg!("2.--->Applying:", depth, &func, &func.type_name(), &args);

        // 递归求值函数和参数
        // important for func to skip $ require in strict mode
        // important for func to be explained in domains.
        let func_eval = func.eval_symbo_with_domain(state, env, depth + 1)?;

        // dbg!(&func_eval, &func_eval.type_name());

        // 分派到具体类型处理

        match func_eval {
            // 顶级builtin，函数别名
            Expression::Symbol(_) => func_eval.eval_symbo(args, false, state, env, depth + 1),

            // Lambda 应用 - 完全求值的函数应用
            Expression::Lambda(params, body, captured_env) => {
                // let pipe_out = state.pipe_out(); //必须先取得pipeout，否则可能被参数取走
                // dbg!("2.--- applying lambda---", &params);
                let mut current_env = env.fork();
                // 先应用已捕获的环境
                if let Some(captured) = captured_env {
                    for (key, value) in captured.iter() {
                        current_env.define(key, value.clone());
                    }
                }
                // 批量参数绑定前先求值所有参数
                let is_in_pipe = state.contains(State::IN_PIPE);
                state.set(State::IN_PIPE);
                let evaluated_args = args
                    .iter()
                    .map(|arg| arg.eval_mut(state, env, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?;
                if !is_in_pipe {
                    state.clear(State::IN_PIPE);
                }

                // if let Some(p) = pipe_out {
                //     evaluated_args.push(p);
                // };

                match bind_arguments(&params, &evaluated_args, &mut current_env) {
                    // 完全应用：求值函数体
                    None => {
                        let result = body.as_ref().eval_mut(state, &mut current_env, depth + 1);
                        match result {
                            Ok(v) => {
                                // self.set_status_code(0, env);
                                Ok(v)
                            }
                            Err(RuntimeError {
                                kind: RuntimeErrorKind::EarlyReturn(v),
                                context: _,
                                depth: _,
                            }) => {
                                // self.set_status_code(0, env);
                                Ok(v)
                            } // 捕获函数体内的return
                            Err(e) => {
                                // self.set_status_code(1, env);
                                Err(e)
                            }
                        }
                    }

                    // 部分应用：返回新的柯里化lambda
                    Some(remain) => Ok(Expression::Lambda(
                        remain,
                        body,
                        Some(current_env.get_bindings_map()),
                    )),
                }
            }

            Expression::Function(.., ref decos) => {
                return match decos.is_empty() {
                    true => self.eval_normal_function(func_eval, args, state, env, depth),
                    false => self.eval_function_with_deco(func_eval, args, state, env, depth),
                };
            }
            // 命令形式的内置函数调用如： fs.read! a
            // 不用!,则进入eval_cmd中
            // Expression::Index
            // 模块调用
            Expression::ModuleCall(modules, function) => {
                state.set(State::IN_DOMAINS);
                state.extend_lookup_domains(&modules);
                let result = self.eval_apply(&function, args, state, env, depth + 1);
                state.truncate_lookup_domains(modules.len());
                state.clear(State::IN_DOMAINS);
                return result;
                // return self.eval_symbo_with_domain(module, function, args, state, env, depth + 1);
            }
            // Expression::None => Ok(Expression::None),
            o => {
                // dbg!(o.type_name());
                Err(RuntimeError::new(
                    RuntimeErrorKind::CannotApply(o.type_name(), func.clone()),
                    self.clone(),
                    depth,
                ))
            }
        }
    }

    pub fn eval_symbo_with_domain(
        &self,
        // name: &String,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match self {
            Expression::Symbol(name) => {
                if state.contains(State::IN_DOMAINS) {
                    // 获取当前查找域
                    let domains = state.get_lookup_domains();

                    // 在查找域中查找模块
                    if let Some(leading) = domains.first() {
                        let root = env.get(leading);
                        let mut parent = match root.as_ref() {
                            Some(Expression::HMap(m)) => m,
                            Some(x) => {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::SymbolNotModule(
                                        leading.to_string(),
                                        x.type_name(),
                                        "current module".into(),
                                        "".to_string(),
                                    ),
                                    self.clone(),
                                    depth,
                                ));
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::SymbolNotDefined(format!(
                                        "{} in current module",
                                        leading
                                    )),
                                    self.clone(),
                                    depth,
                                ));
                            }
                        };
                        for (index, domain) in domains.iter().skip(1).enumerate() {
                            match parent.get(domain) {
                                Some(Expression::HMap(m)) => {
                                    parent = m;
                                }
                                Some(x) => {
                                    return Err(RuntimeError::new(
                                        RuntimeErrorKind::SymbolNotModule(
                                            domain.to_string(),
                                            x.type_name(),
                                            domains[index].to_string().into(),
                                            domains.join("->"),
                                        ),
                                        self.clone(),
                                        depth,
                                    ));
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        RuntimeErrorKind::NoModuleDefined(
                                            domain.to_owned(),
                                            domains[index].to_string(),
                                            domains.join("->"),
                                        ),
                                        self.clone(),
                                        depth,
                                    ));
                                }
                            }
                        }
                        // after got parent
                        if let Some(func) = parent.get(name) {
                            // state.push_lookup_domain(module);
                            // let result = self.eval_apply(func, args, state, env, depth + 1);
                            // state.pop_lookup_domain();
                            // return result;
                            return Ok(func.clone());
                        } else {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::SymbolNotDefinedInModule(
                                    name.to_owned(),
                                    domains.last().unwrap().to_owned(),
                                    domains.join("->"),
                                ),
                                self.clone(),
                                depth,
                            ));
                        }
                    }
                }
                return match env.get(name) {
                    Some(expr) => Ok(expr),
                    None => Ok(self.clone()),
                };
            }
            // may values as builtin
            Self::Property(lhs, rhs) => {
                return self.handle_property(lhs, rhs, state, env, depth + 1);
            }
            Self::Index(lhs, rhs) => {
                return self.handle_index_or_slice(lhs, rhs, state, env, depth + 1);
            }
            // for lambda/function/moduleCall them self.
            _ => Ok(self.clone()),
        }
    }

    /// 执行
    #[inline]
    pub fn eval_command(
        &self,
        args: &Vec<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // dbg!(
        //     "2.--->Command:",
        //     &eval_cmd,
        //     &eval_cmd.type_name(),
        //     &args,
        //     &state
        // );
        match self {
            Expression::String(cmdx_str) => {
                // 空命令
                if cmdx_str.is_empty() || cmdx_str == ":" {
                    if args.is_empty() {
                        Ok(Expression::None)
                    } else {
                        let aa = args.split_at(1);
                        handle_command(
                            self,
                            &aa.0.to_vec().first().unwrap().to_string(),
                            &aa.1.to_vec(),
                            state,
                            env,
                            depth + 1,
                        )
                    }
                } else {
                    // let a=ls -l; a '/';
                    let cmdx_vec = cmdx_str.split_whitespace().collect::<Vec<_>>();
                    let mut new_vec = Vec::with_capacity(cmdx_vec.len() + args.len());
                    new_vec.extend_from_slice(
                        &cmdx_vec[1..]
                            .iter()
                            .map(|v| Expression::from(v.to_string()))
                            .collect::<Vec<_>>(),
                    );
                    new_vec.extend_from_slice(args);
                    handle_command(
                        self,
                        &cmdx_vec.first().unwrap().to_string(),
                        &new_vec,
                        state,
                        env,
                        depth + 1,
                    )
                }
            }
            // 符号
            Expression::Symbol(_) => self.eval_symbo(args, true, state, env, depth + 1),
            // 延迟赋值命令 let x := ls
            Expression::Command(cmd_sym, cmd_args) => {
                let mut new_vec = Vec::with_capacity(cmd_args.len() + args.len());
                new_vec.extend_from_slice(&cmd_args);
                new_vec.extend_from_slice(args);
                handle_command(self, &cmd_sym.to_string(), &new_vec, state, env, depth + 1)
            }
            other => match args.is_empty() {
                true => Ok(other.clone()), // 单个symbol或变量，直接返回
                false => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeError {
                        //非法命令
                        expected: "Symbol as command".to_string(),
                        sym: other.to_string(),
                        found: other.type_name(),
                    },
                    self.clone(),
                    depth,
                )),
            },
        }
    }

    #[inline]
    pub fn eval_symbo(
        &self,
        args: &[Expression],
        is_cmd_mode: bool,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match self {
            Self::Symbol(cmd_sym) =>
            // dbg!("   3.--->applying Symbol:", &cmd_sym, &args);
            // NOTE alias is a symbol, when appreared on right of pipe, the _ receiver is not injected
            {
                // top level builtin lib cmd, first of all, to invoid cmd to be covered by vars.
                // cmd already checked before this function.
                // this is only for func
                if !is_cmd_mode
                    && let Some(btr) = handle_builtin(
                        &Expression::Blank,
                        cmd_sym.as_ref(),
                        args,
                        self,
                        state,
                        env,
                        depth,
                    )?
                {
                    return Ok(btr);
                }

                match alias::get_alias(cmd_sym.as_str()) {
                    // 别名
                    Some(cmd_alias) => {
                        // dbg!(&cmd_alias.type_name());

                        // 合并参数
                        match cmd_alias {
                            // alias a=ls -l
                            Expression::Command(cmd_name, cmd_args) if is_cmd_mode => {
                                let mut new_vec = Vec::with_capacity(cmd_args.len() + args.len());
                                new_vec.extend_from_slice(&cmd_args);
                                new_vec.extend_from_slice(args);
                                handle_command(
                                    self,
                                    &cmd_name.as_ref().to_string(),
                                    &new_vec,
                                    state,
                                    env,
                                    depth + 1,
                                )
                            }
                            // alias a=ls
                            Expression::Symbol(cmd_str) | Expression::String(cmd_str)
                                if is_cmd_mode =>
                            {
                                handle_command(self, &cmd_str, args, state, env, depth + 1)
                            }

                            // -----need to inject _ receiver if on pipe right.
                            // alias a=myfunc()
                            Expression::Apply(..) if !is_cmd_mode => cmd_alias
                                .append_args(args.to_vec())
                                // .ensure_has_receiver()
                                .eval_mut(state, env, depth + 1),
                            // alias a=String.red   a=myfunc
                            Expression::Function(..) if !is_cmd_mode => cmd_alias
                                .ensure_fn_apply()
                                .append_args(args.to_vec())
                                // .ensure_has_receiver()
                                .eval_mut(state, env, depth + 1),
                            // alias a=String.red()
                            Expression::Chain(..) => cmd_alias
                                .append_args(args.to_vec())
                                // .ensure_has_receiver()
                                .eval_mut(state, env, depth + 1),
                            // alias a=String.red
                            // Expression::Index(..) => {
                            //     let cmdx = cmd_alias.eval_mut(state, env, depth + 1)?;
                            //     return match cmdx {
                            //         Expression::Builtin(bti) => {
                            //             self.eval_builtin(&bti, args, state, env, depth)
                            //         }
                            //         _ => Err(RuntimeError::new(
                            //             RuntimeErrorKind::TypeError {
                            //                 expected: "alias contains Builtin".into(),
                            //                 sym: cmdx.to_string(),
                            //                 found: cmdx.type_name(),
                            //             },
                            //             self.clone(),
                            //             depth,
                            //         )),
                            //     };
                            // }
                            _ => Err(RuntimeError::new(
                                RuntimeErrorKind::TypeError {
                                    expected: match is_cmd_mode {
                                        true => "alias for Command/Builtin".into(),
                                        false => "alias for Function/Builtin".into(),
                                    },
                                    sym: cmd_alias.to_string(),
                                    found: cmd_alias.type_name(),
                                },
                                self.clone(),
                                depth,
                            )),
                        }
                    }
                    // _ => {
                    //     match get_builtin_via_expr(&Expression::Blank, cmd_sym.as_str()) {
                    //         Some(bfn) => {
                    //             if args.contains(&Expression::Blank) {
                    //                 let received_args = args
                    //                     .iter()
                    //                     .map(|x| match x {
                    //                         Expression::Blank => {
                    //                             state.pipe_out().unwrap_or(Expression::Blank)
                    //                         }
                    //                         o => o.clone(),
                    //                     })
                    //                     .collect::<Vec<_>>();
                    //                 bfn(&received_args, env, self)
                    //             } else {
                    //                 bfn(&args, env, self)
                    //             }
                    //             // if let Some((base, args)) = args.split_first() {
                    //             // } else {
                    //             //     bfn(&Expression::Blank, &args, env, self, depth)
                    //             // }
                    //         }
                    // }
                    // match get_builtin(cmd_sym.as_str()) {
                    //     // 顶级内置命令
                    //     Some(Expression::Builtin(bti)) => {
                    //         // dbg!("branch to builtin:", &cmd, &bti);
                    //         // bti.apply(args.to_vec()).eval_apply(state, env, depth+1)
                    //         self.eval_builtin(bti, args, state, env, depth + 1)
                    //     }
                    // Some(exp) => {} //never here
                    _ => {
                        if is_cmd_mode {
                            // 三方命令
                            handle_command(self, cmd_sym, args, state, env, depth + 1)
                        } else {
                            Err(RuntimeError::new(
                                RuntimeErrorKind::TypeError {
                                    expected: "symbol for Function/Builtin".into(),
                                    sym: cmd_sym.to_string(),
                                    found: "Symbol with no meaning".to_string(),
                                },
                                self.clone(),
                                depth,
                            ))
                        }
                    } //     }
                      // }
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "symbol".into(),
                    sym: other.to_string(),
                    found: other.type_name(),
                },
                self.clone(),
                depth,
            )),
        }
    }
}
/// 参数绑定辅助函数 - 将参数绑定到环境中
///
/// # 参数
/// - `params`: 形式参数列表
/// - `args`: 实际参数列表(已求值)
/// - `target_env`: 目标绑定环境(通常是新创建的lambda环境)
///
/// # 返回值
/// 返回元组: (剩余未绑定的形式参数)
#[inline]
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

#[inline]
pub fn handle_builtin(
    base: &Expression,
    method: &str,
    args: &[Expression],
    ctx: &Expression,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Option<Expression>, RuntimeError> {
    if let Some(bfn) = get_builtin_via_expr(base, method) {
        // let bt_result = if args.contains(&Expression::Blank) {
        //     let received_args = args
        //         .iter()
        //         .map(|x| match x {
        //             Expression::Blank => state.pipe_out().unwrap_or(Expression::Blank),
        //             o => o.clone(),
        //         })
        //         .collect::<Vec<_>>();
        //     bfn(&received_args, env, ctx)
        // } else {
        //     bfn(&args, env, ctx)
        // };
        // return Some(bt_result);
        // TODO if eval first, btin should not eval again
        // and the specail conditon for where : size>0
        // not works well, should fix
        // the resean to excute it here, is for local vars in loop,
        // only current env knows the state.

        let p_args = match base {
            // lazy cmd is in top
            Expression::Blank => prepare_args(method, args, true, None, env, state, depth)?,
            // 判断是String.red 还是 ‘xx'.red
            Expression::Symbol(_) => prepare_args(method, args, false, None, env, state, depth)?,
            // 'xx' should be injected
            val => prepare_args(method, args, false, Some(val.clone()), env, state, depth)?,
        };
        let result = bfn(&p_args, env, ctx)?;

        return Ok(Some(result));
    }
    Ok(None)
}

impl Expression {
    #[inline]
    pub fn handle_builtin_n_normal_cmd(
        &self,
        cmd: &Expression,
        args: &Vec<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // inject builtin cmd executor here, to invoid to influent other index eval.

        if let Expression::Property(base, method) = cmd {
            if let Some(btr) =
                handle_builtin(base, &method.to_string(), args, self, state, env, depth)?
            {
                return Ok(btr);
            }
            // 自定义Map不应以命令方式调用,但文件名可能以a.b的方式存在
        }

        if let Expression::Symbol(cmd_sym) = cmd {
            if let Some(btr) = handle_builtin(
                &Expression::Blank,
                cmd_sym.as_ref(),
                args,
                self,
                state,
                env,
                depth,
            )? {
                return Ok(btr);
            }
        }

        // symbol和Property方式匹配失败后，允许其他含义，所以继续匹配
        return match cmd {
            Expression::Variable(_)
            | Expression::Symbol(_)
            | Expression::String(_)
            | Expression::Property(..) => {
                let eval_cmd = cmd.eval_mut(state, env, depth + 1)?;
                return eval_cmd.eval_command(args.as_ref(), state, env, depth + 1);
            }
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "Symbol".into(),
                    sym: cmd.type_name(),
                    found: cmd.to_string(),
                },
                self.clone(),
                depth,
            )),
        };
    }

    /// chain call
    #[inline]
    pub fn eval_chain(
        &self,
        base: &Expression,
        calls: &Vec<ChainCall>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // 首先求值基础表达式,但需要注意变量覆盖lib名
        let mut current_base = match base {
            Expression::Symbol(s) => match is_lib(s) {
                true => base.clone(),
                false => base.eval_mut(state, env, depth + 1)?,
            },
            _ => base.eval_mut(state, env, depth + 1)?,
        };

        // 依次执行每个链式调用
        for call in calls {
            let method = call.method.as_str();
            let lib_result =
                handle_builtin(&current_base, method, &call.args, self, state, env, depth);

            let executed = match lib_result? {
                Some(result) => Ok(result),

                // 非内置函数
                None => {
                    return match current_base {
                        // 是自定义的Map内的func
                        Expression::Map(map) => match map.get(method) {
                            Some(func) => {
                                // 字典的键值是可执行对象
                                match func {
                                    Expression::Lambda(..) | Expression::Function(..) => {
                                        self.eval_apply(func, &call.args, state, env, depth)
                                    }
                                    s => Err(RuntimeError::new(
                                        RuntimeErrorKind::NotAFunction(s.to_string()),
                                        self.clone(),
                                        depth,
                                    )),
                                }
                            }
                            None => Err(RuntimeError::new(
                                RuntimeErrorKind::KeyNotFound(method.into()),
                                self.clone(),
                                depth,
                            )),
                        },

                        _ => Err(RuntimeError::new(
                            RuntimeErrorKind::NoLibDefined(
                                method.to_string(),
                                current_base.type_name().into(),
                                "eval_chain".into(),
                                current_base.to_string(),
                            ),
                            self.clone(),
                            depth,
                        )),
                    };
                }
            };
            // 构造方法调用表达式
            // let excuted = match &current_base {
            //     Expression::HMap(map) => {
            //         match map.get(method) {
            //             Some(func) => {
            //                 // 字典的键值是可执行对象
            //                 match &func {
            //                     Expression::Builtin(bti) => {
            //                         self.eval_builtin(bti, &call.args, state, env, depth + 1)
            //                     }
            //                     Expression::Lambda(..) | Expression::Function(..) => {
            //                         self.eval_apply(func, &call.args, state, env, depth)
            //                     }
            //                     s => Err(RuntimeError::new(
            //                         RuntimeErrorKind::NotAFunction(s.to_string()),
            //                         self.clone(),
            //                         depth,
            //                     )),
            //                 }
            //             }
            //             None => {
            //                 // 尝试内置方法
            //                 self.eval_lib_method(
            //                     "Map".into(),
            //                     method,
            //                     &call.args,
            //                     current_base,
            //                     state,
            //                     env,
            //                     depth,
            //                 )
            //             }
            //         }
            //     }
            //     // 如果当前值是对象，尝试获取其方法
            //     Expression::Map(map) => {

            //     }
            //     // 对于其他类型，查找内置方法
            //     o => match o.get_belong_lib_name() {
            //         Some(mo_name) => self.eval_lib_method(
            //             mo_name,
            //             method,
            //             &call.args,
            //             current_base,
            //             state,
            //             env,
            //             depth,
            //         ),
            //         _ => Err(RuntimeError::new(
            //             RuntimeErrorKind::NoLibDefined(
            //                 current_base.to_string(),
            //                 current_base.type_name().into(),
            //                 "eval_chain".into(),
            //             ),
            //             self.clone(),
            //             depth,
            //         )),
            //     },
            // };
            current_base = executed?;
        }

        Ok(current_base)
    }

    // #[inline]
    // pub fn eval_lib_method(
    //     &self,
    //     lib: Cow<'static, str>,
    //     call_method: &str,
    //     call_args: &[Expression],
    //     current_base: Expression,
    //     state: &mut State,
    //     env: &mut Environment,
    //     depth: usize,
    // ) -> Result<Expression, RuntimeError> {
    //     match get_builtin(&lib) {
    //         // 顶级内置命令
    //         Some(Expression::HMap(hmap)) => {
    //             // let mut final_args = call_args
    //             //     .iter()
    //             //     .map(|a| a.eval_mut(state, env, depth + 1))
    //             //     .collect::<Result<Vec<_>, _>>()?;
    //             // final_args.push(current_base);

    //             let mut final_args = Vec::with_capacity(call_args.len() + 1);
    //             final_args.push(current_base);
    //             final_args.extend_from_slice(
    //                 call_args
    //                     .iter()
    //                     .map(|a| a.eval_mut(state, env, depth + 1))
    //                     .collect::<Result<Vec<_>, _>>()?
    //                     .as_ref(),
    //             );
    //             //dbg!(&hmap);
    //             let bti_expr = hmap.as_ref().get(call_method).ok_or(RuntimeError::new(
    //                 RuntimeErrorKind::MethodNotFound(
    //                     call_method.to_string().into(),
    //                     lib.to_owned(),
    //                 ),
    //                 self.clone(),
    //                 depth,
    //             ))?;
    //             match bti_expr {
    //                 Expression::Builtin(bti) => {
    //                     self.eval_builtin(bti, &final_args, state, env, depth + 1)
    //                 }
    //                 _ =>
    //                 // chained bti, inner has child.
    //                 {
    //                     Err(RuntimeError::new(
    //                         RuntimeErrorKind::CustomError(lib),
    //                         self.clone(),
    //                         depth,
    //                     ))
    //                 }
    //             }
    //         }

    //         _ => Err(RuntimeError::new(
    //             RuntimeErrorKind::NoLibDefined(
    //                 call_method.to_string(),
    //                 current_base.type_name().into(),
    //                 "eval_lib_method".into(),
    //                 current_base.to_string(),
    //             ),
    //             self.clone(),
    //             depth,
    //         )),
    //     }
    // }
}
