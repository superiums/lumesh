use super::Builtin;
use super::eval::State;
use crate::expression::cmd_excutor::handle_command;
use crate::expression::{ChainCall, alias};
use crate::{Environment, Expression, RuntimeError, RuntimeErrorKind, get_builtin};
use std::borrow::Cow;

/// 执行
impl Expression {
    // 函数应用
    #[inline]
    pub fn eval_apply(
        &self,
        func: &Expression,
        args: &Vec<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        if depth > 400 {
            //防止函数互相调用无限循环
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
            Expression::Symbol(cmd_sym) => {
                self.eval_symbo(cmd_sym, args, false, state, env, depth + 1)
            }

            Expression::Builtin(bti) => self.eval_builtin(&bti, args, state, env, depth + 1),
            // Lambda 应用 - 完全求值的函数应用
            Expression::Lambda(params, body) => {
                // let pipe_out = state.pipe_out(); //必须先取得pipeout，否则可能被参数取走
                // dbg!("2.--- applying lambda---", &params);
                let mut current_env = env.fork();

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
                    Some(remain) => Ok(Expression::Lambda(remain, body)),
                }
            }

            // Macro 应用 - 不自动求值参数的展开
            // Expression::Macro(params, body) => {
            //     match bind_arguments(params, args.to_owned(), env) {
            //         // 完全应用：求值函数体
            //         None => body.eval_mut(true, env, depth + 1),

            //         // 部分应用：返回新的柯里化lambda
            //         Some(remain) => Ok(Expression::Macro(remain, body)),
            //     }
            // }
            Expression::Function(name, params, pc, body, mut decos) => {
                // dbg!("2.--- applying function---", &name, &params, &decos);
                // dbg!(&def_env);
                // 参数数量校验
                // let pipe_out = state.pipe_out(); //必须先取得pipeout，否则可能被参数取走
                // let pipe_arg_len = match &pipe_out {
                //     Some(_) => 1,
                //     _ => 0,
                // };

                // if pc.is_none() && args.len() + pipe_arg_len > params.len() {
                if pc.is_none() && args.len() > params.len() {
                    return Err(RuntimeError::new(
                        crate::RuntimeErrorKind::TooManyArguments {
                            name,
                            max: params.len(),
                            received: args.len(),
                        },
                        self.clone(),
                        depth,
                    ));
                }

                let is_in_pipe = state.contains(State::IN_PIPE);
                state.set(State::IN_PIPE);

                // dbg!(&decos, "----");
                // println!();
                match decos.is_empty() {
                    false => {
                        // dbg!(&decov);
                        let mut last_fn = Expression::Function(name, params, pc, body, vec![]);
                        let mut env_deco = env.fork();
                        state.set(State::IN_DECO);
                        decos.reverse();
                        for deco in decos {
                            // dbg!(&deco);
                            let deco_f = env.get(deco.0.as_str());
                            if let Some(deco_fn) = deco_f {
                                // let deco_symbo_type = deco_fn.type_name();
                                // if "Function" == deco_symbo_type || "Labmda" == deco_symbo_type {
                                if matches!(
                                    deco_fn,
                                    Expression::Function(..) | Expression::Lambda(..)
                                ) {
                                    let deco_args = deco.1.unwrap_or(vec![]);
                                    // dbg!("deco is func", &deco_fn, &deco_args, &last_fn);
                                    let evaled_deco = self.eval_apply(
                                        &deco_fn,
                                        &deco_args,
                                        state,
                                        &mut env_deco,
                                        depth + 1,
                                    )?;
                                    // dbg!(&evaled_deco);
                                    // 只构建装饰器链，不执行
                                    last_fn = self.eval_apply(
                                        &evaled_deco,
                                        &vec![last_fn],
                                        state,
                                        &mut env_deco,
                                        depth + 1,
                                    )?;
                                } else {
                                    return Err(RuntimeError::common(
                                        "trying to apply non-function as decorator".into(),
                                        deco_fn,
                                        depth,
                                    ));
                                }
                            } else {
                                return Err(RuntimeError::common(
                                    "decorator not defined".into(),
                                    self.clone(),
                                    depth,
                                ));
                            }
                        }
                        last_fn =
                            self.eval_apply(&last_fn, args, state, &mut env_deco, depth + 1)?;
                        state.clear(State::IN_DECO);
                        // 最后执行装饰过的函数
                        Ok(last_fn)
                    }
                    _ => {
                        let mut actual_args = args
                            .iter()
                            .map(|a| a.eval_mut(state, env, depth + 1))
                            .collect::<Result<Vec<_>, _>>()?;
                        if !is_in_pipe {
                            state.clear(State::IN_PIPE);
                        }

                        // if let Some(p) = pipe_out {
                        //     actual_args.push(p);
                        // };

                        // 填充默认值逻辑（新增）
                        for (i, (_, default)) in params.iter().enumerate() {
                            if i >= actual_args.len() {
                                if let Some(def_expr) = default {
                                    // 仅允许基本类型直接使用
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

                        // 创建新作用域并执行
                        let new_env = match state.contains(State::IN_DECO) {
                            true => env,
                            _ => &mut env.fork(),
                        };
                        if let Some(collector) = pc {
                            if state.contains(State::IN_DECO) && new_env.has(collector.as_str()) {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::Redeclaration(collector),
                                    self.clone(),
                                    depth,
                                ));
                            }
                            new_env.define(
                                collector.as_str(),
                                Expression::from(actual_args[params.len()..].to_vec()),
                            );
                        }
                        for ((param, _), arg) in params.iter().zip(actual_args) {
                            // 装饰器内，不允许各个装饰器中有重复的参数名
                            if state.contains(State::IN_DECO) && new_env.has(param) {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::Redeclaration(param.to_string()),
                                    self.clone(),
                                    depth,
                                ));
                            }
                            new_env.define(param, arg);
                        }

                        match body.as_ref().eval_mut(state, new_env, depth + 1) {
                            Ok(v) => Ok(v),
                            // 捕获函数体内的return
                            Err(RuntimeError {
                                kind: RuntimeErrorKind::EarlyReturn(v),
                                context: _,
                                depth: _,
                            }) => Ok(v),
                            Err(e) => Err(e),
                        }
                    }
                }
            }
            // 模块调用
            Expression::ModuleCall(modules, function) => {
                state.extend_lookup_domains(&modules);
                let result = self.eval_apply(&function, args, state, env, depth + 1);
                state.truncate_lookup_domains(modules.len());
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
                return match env.get(name) {
                    Some(expr) => Ok(expr),
                    None => Ok(self.clone()),
                };
            }
            // may values as builtin
            Self::Index(lhs, rhs) => return self.handle_index(lhs, rhs, state, env, depth),
            // for lambda/function/moduleCall them self.
            _ => Ok(self.clone()),
        }
    }

    /// 执行
    #[inline]
    pub fn eval_command(
        &self,
        eval_cmd: Expression,
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
        match eval_cmd {
            Expression::Builtin(bti) => self.eval_builtin(&bti, args, state, env, depth),
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
            Expression::Symbol(cmd_sym) => {
                self.eval_symbo(cmd_sym, args, true, state, env, depth + 1)
            }
            // 延迟赋值命令 let x := ls
            Expression::Command(cmd_sym, cmd_args) => {
                let mut new_vec = Vec::with_capacity(cmd_args.len() + args.len());
                new_vec.extend_from_slice(&cmd_args);
                new_vec.extend_from_slice(args);
                handle_command(self, &cmd_sym.to_string(), &new_vec, state, env, depth + 1)
            }
            other => match args.is_empty() {
                true => Ok(other), // 单个symbol或变量，直接返回
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
        cmd_sym: String,
        args: &Vec<Expression>,
        is_cmd_mode: bool,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // dbg!("   3.--->applying Symbol:", &cmd_sym, &args);
        // NOTE alias is a symbol, when appreared on right of pipe, the _ receiver is not injected
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
                    Expression::Symbol(cmd_str) | Expression::String(cmd_str) if is_cmd_mode => {
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
            _ => {
                match get_builtin(cmd_sym.as_str()) {
                    // 顶级内置命令
                    Some(Expression::Builtin(bti)) => {
                        // dbg!("branch to builtin:", &cmd, &bti);
                        // bti.apply(args.to_vec()).eval_apply(state, env, depth+1)
                        self.eval_builtin(bti, args, state, env, depth + 1)
                    }
                    // Some(exp) => {} //never here
                    _ => {
                        if is_cmd_mode {
                            // 三方命令
                            handle_command(self, &cmd_sym, args, state, env, depth + 1)
                        } else {
                            Err(RuntimeError::new(
                                RuntimeErrorKind::TypeError {
                                    expected: "symbol for Function/Builtin".into(),
                                    sym: cmd_sym,
                                    found: "Symbol with no meaning".to_string(),
                                },
                                self.clone(),
                                depth,
                            ))
                        }
                    }
                }
            }
        }
    }
    #[inline]
    pub fn eval_builtin(
        &self,
        bti: &Builtin,
        args: &Vec<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // dbg!("   3.--->applying Builtin:", &bti.name, &args);
        // let pipe_out = state.pipe_out();

        // 执行时机应由内置函数自己选择，如 where(size>0)
        // 注意：bultin args通过相同env环境执行，但未传递state参数，无法继续得知管道状态
        // let rst = match pipe_out {
        //     Some(p) => {
        // let mut appened_args = args.clone();
        // appened_args.push(p);
        let appened_args = args
            .iter()
            .map(|x| match x {
                Expression::Blank => state.pipe_out().unwrap_or(Expression::Blank),
                o => o.clone(),
            })
            .collect::<Vec<_>>();
        let rst = (bti.body)(appened_args.as_ref(), env);
        //     }
        //     _ => (bti.body)(args, env),
        // };

        rst.map_err(|e| {
            RuntimeError::new(
                RuntimeErrorKind::BuiltinFailed(bti.name.clone(), e.to_string()),
                self.clone(),
                depth,
            )
        })
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

impl Expression {
    pub fn eval_chain(
        &self,
        base: &Expression,
        calls: &Vec<ChainCall>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // 首先求值基础表达式
        let mut current_base = match &base {
            // explain bultin
            Expression::Symbol(sym) if sym.starts_with(char::is_uppercase) => {
                match get_builtin(sym) {
                    Some(b) => b.clone(),
                    _ => base.eval_mut(state, env, depth + 1)?,
                }
            }
            _ => base.eval_mut(state, env, depth + 1)?,
        };

        // 依次执行每个链式调用
        for call in calls {
            let method = call.method.as_str();

            // 构造方法调用表达式
            let excuted = match &current_base {
                Expression::HMap(map) => {
                    match map.get(method) {
                        Some(func) => {
                            // 字典的键值是可执行对象
                            match &func {
                                Expression::Builtin(bti) => {
                                    self.eval_builtin(bti, &call.args, state, env, depth + 1)
                                }
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
                        None => {
                            // 尝试内置方法
                            self.eval_lib_method(
                                "Map".into(),
                                method,
                                &call.args,
                                current_base,
                                state,
                                env,
                                depth,
                            )
                        }
                    }
                }
                // 如果当前值是对象，尝试获取其方法
                Expression::Map(map) => {
                    match map.get(method) {
                        Some(func) => {
                            // 字典的键值是可执行对象
                            match func {
                                Expression::Lambda(..) | Expression::Function(..) => {
                                    self.eval_apply(func, &call.args, state, env, depth)
                                }
                                Expression::Builtin(bti) => {
                                    self.eval_builtin(bti, &call.args, state, env, depth + 1)
                                }
                                s => Err(RuntimeError::new(
                                    RuntimeErrorKind::NotAFunction(s.to_string()),
                                    self.clone(),
                                    depth,
                                )),
                            }
                        }
                        None => {
                            // 尝试内置方法
                            self.eval_lib_method(
                                "Map".into(),
                                method,
                                &call.args,
                                current_base,
                                state,
                                env,
                                depth,
                            )
                        }
                    }
                }
                // 对于其他类型，查找内置方法
                o => match o.get_belong_lib_name() {
                    Some(mo_name) => self.eval_lib_method(
                        mo_name,
                        method,
                        &call.args,
                        current_base,
                        state,
                        env,
                        depth,
                    ),
                    _ => Err(RuntimeError::new(
                        RuntimeErrorKind::NoLibDefined(
                            current_base.to_string(),
                            current_base.type_name().into(),
                            "eval_chain".into(),
                        ),
                        self.clone(),
                        depth,
                    )),
                },
            };
            current_base = excuted?;
        }

        Ok(current_base)
    }

    #[inline]
    pub fn eval_lib_method(
        &self,
        lib: Cow<'static, str>,
        call_method: &str,
        call_args: &[Expression],
        current_base: Expression,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match get_builtin(&lib) {
            // 顶级内置命令
            Some(Expression::HMap(hmap)) => {
                // let mut final_args = call_args
                //     .iter()
                //     .map(|a| a.eval_mut(state, env, depth + 1))
                //     .collect::<Result<Vec<_>, _>>()?;
                // final_args.push(current_base);

                let mut final_args = Vec::with_capacity(call_args.len() + 1);
                final_args.push(current_base);
                final_args.extend_from_slice(
                    call_args
                        .iter()
                        .map(|a| a.eval_mut(state, env, depth + 1))
                        .collect::<Result<Vec<_>, _>>()?
                        .as_ref(),
                );
                //dbg!(&hmap);
                let bti_expr = hmap.as_ref().get(call_method).ok_or(RuntimeError::new(
                    RuntimeErrorKind::MethodNotFound(
                        call_method.to_string().into(),
                        lib.to_owned(),
                    ),
                    self.clone(),
                    depth,
                ))?;
                match bti_expr {
                    Expression::Builtin(bti) => {
                        self.eval_builtin(bti, &final_args, state, env, depth + 1)
                    }
                    _ =>
                    // chained bti, inner has child.
                    {
                        Err(RuntimeError::new(
                            RuntimeErrorKind::CustomError(lib),
                            self.clone(),
                            depth,
                        ))
                    }
                }
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::NoLibDefined(
                    current_base.to_string(),
                    current_base.type_name().into(),
                    "eval_module_method".into(),
                ),
                self.clone(),
                depth,
            )),
        }
    }
}
