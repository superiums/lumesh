use super::catcher::catch_error;
use super::eval::State;
use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    expression::DestructurePattern,
    modman::use_module,
    runtime::{IFS_FOR, ifs_contains},
    utils::expand_home,
};
use glob::glob;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

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
            Self::For(var, index_name, list_expr, body) => self.handle_for(
                var.clone(),
                index_name.clone(),
                list_expr,
                body.as_ref(),
                state,
                env,
                depth + 1,
            ),

            Self::While(cond, body) => {
                // 循环求值直到条件为假
                let mut last = Ok(Expression::None);
                let mut condition_result =
                    cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy();

                while condition_result {
                    last = body.as_ref().eval_mut(state, env, depth + 1);
                    match last {
                        Ok(_) => {
                            condition_result =
                                cond.as_ref().eval_mut(state, env, depth + 1)?.is_truthy();
                        } //todo maybe only eval when condition change
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

            Self::Lambda(params, body, _) => {
                let free_vars = body.get_free_variables();

                // 只捕获自由变量
                let mut captured_env = HashMap::new();
                for var in &free_vars {
                    if let Some(value) = env.get(var) {
                        captured_env.insert(var.to_string(), value);
                    }
                }
                return Ok(Self::Lambda(
                    params.clone(),
                    body.clone(),
                    Some(captured_env),
                ));
            }
            // 处理函数定义
            Self::Function(name, params, pc, body, decos) => {
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
                let func = Self::Function(
                    name.clone(),
                    params.clone(),
                    pc.clone(),
                    body.clone(),
                    decos.clone(),
                );
                if state.contains(State::STRICT) && env.has(name) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::Redeclaration(name.to_string()),
                        self.clone(),
                        depth,
                    ));
                }
                env.define(name, func.clone());
                // deco eval need it
                // if state.contains(State::IN_DECO | State::IN_ASSIGN) {
                Ok(func)
                // } else {
                //     Ok(Expression::None)
                // }
            }

            Self::Sequence(exprs) => {
                for expr in exprs {
                    expr.eval_mut(state, env, depth + 1)?;
                }
                Ok(Expression::None)
            }

            // 块表达式
            Self::Block(exprs) => {
                // 顺序求值语句块
                if exprs.is_empty() {
                    return Ok(Expression::None);
                }

                let mut last = Expression::None;
                let is_last_local = state.contains(State::IN_LOCAL);
                let last_local_vars = if is_last_local {
                    Some(state.get_local_vars())
                } else {
                    None
                };
                state.set(State::IN_LOCAL);
                for expr in exprs.as_ref() {
                    last = expr.eval_mut(state, env, depth + 1)?;
                }
                if !state.contains(State::IN_FOR_LOOP) && !is_last_local {
                    // not clear local in for loop. especialy the index.
                    state.clear_local_var();
                }
                if is_last_local {
                    state.set_local_vars(last_local_vars.unwrap());
                } else {
                    state.clear(State::IN_LOCAL);
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
            Expression::Use(alias, module_path) => {
                // let module_info = load_module(module_path, env)?;

                // // 使用别名或模块名作为键，存储为Map
                // let module_name = alias.as_ref().unwrap_or(module_path);
                // let module_map = Expression::HMap(Rc::new(module_info.functions));
                let (module_name, module_map) = use_module(alias, module_path, env)?;
                // dbg!(&module_map);
                env.define(&module_name, module_map);
                Ok(Expression::None)
            }
            // Expression::Use(alias, module_path) => {
            //     let mut loaded_modules = HashMap::new();
            //     load_modules_to_map(&mut loaded_modules, alias, module_path, self, env, depth)?;

            //     for (module, functions) in loaded_modules.iter() {
            //         env.define(module, functions.clone());
            //     }

            //     Ok(Expression::None)
            // }

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
        var: String,
        index_name: Option<String>,
        list_expr: &Rc<Expression>,
        body: &Expression,
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
                let count = iterator.clone().count().div_ceil(step.max(1));
                execute_iteration(var, index_name, iterator, count, body, state, env, depth)
            }
            Expression::List(items) => {
                let owned_items: Vec<Expression> = items.iter().cloned().collect();
                let iterator = owned_items.into_iter();
                // let iterator = items.iter().cloned();
                execute_iteration(
                    var,
                    index_name,
                    iterator,
                    items.iter().count(),
                    body,
                    state,
                    env,
                    depth,
                )
            }
            Expression::String(str) => {
                let s = expand_home(str.as_ref());
                if s.contains('*') {
                    // glob expansion logic
                    let iterator = glob_expand(&s).into_iter().map(Expression::String);
                    execute_iteration(var, index_name, iterator, s.len(), body, state, env, depth)
                } else {
                    let iterator = ifs_split(&s, env).into_iter().map(Expression::String);
                    execute_iteration(var, index_name, iterator, s.len(), body, state, env, depth)
                }
            }
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::ForNonList(list_excuted),
                self.clone(),
                depth,
            )),
        }
    }

    pub fn destructure_assign(
        &self,
        patterns: &Vec<DestructurePattern>,
        value: Expression,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        match value {
            // 数组解构
            Expression::List(values) => {
                for (i, pattern) in patterns.iter().enumerate() {
                    match pattern {
                        DestructurePattern::Identifier(name) => {
                            if let Some(val) = values.get(i) {
                                env.define(name.as_str(), val.clone());
                            } else {
                                env.define(name.as_str(), Expression::None);
                            }
                        }
                        DestructurePattern::Rest(name) => {
                            let rest_values: Vec<Expression> =
                                values.iter().skip(i).cloned().collect();
                            env.define(name.as_str(), Expression::List(Rc::new(rest_values)));
                            break;
                        } // ... 其他模式
                        _ => {
                            return Err(RuntimeError::common(
                                "never use map_destructure on List".into(),
                                self.clone(),
                                depth,
                            ));
                        }
                    }
                }
                Ok(Expression::None)
            }

            // 对象解构
            Expression::Map(map) => {
                for pattern in patterns {
                    match pattern {
                        DestructurePattern::Identifier(name) => {
                            let value = map.get(name).cloned().unwrap_or(Expression::None);
                            env.define(name.as_str(), value);
                        }
                        DestructurePattern::Renamed((key, name)) => {
                            let value = map.get(key).cloned().unwrap_or(Expression::None);
                            env.define(name.as_str(), value);
                        }
                        _ => {
                            return Err(RuntimeError::common(
                                "never use list_destructure on Map".into(),
                                self.clone(),
                                depth,
                            ));
                        }
                    }
                }
                Ok(Expression::None)
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "destructurable value".into(),
                    sym: value.to_string(),
                    found: value.type_name(),
                },
                self.clone(),
                depth,
            )),
        }
    }

    pub fn get_free_variables(&self) -> HashSet<String> {
        match self {
            // 变量和符号 - 符号是自由变量，变量需要环境解析
            Self::Symbol(name) | Self::Variable(name) => HashSet::from([name.clone()]),

            // Lambda - 从body收集自由变量，然后移除参数
            Self::Lambda(params, body, _) => {
                let mut free_vars = body.get_free_variables();
                for param in params {
                    free_vars.remove(param);
                }
                free_vars
            }

            // 二元操作符 - 收集左右操作数
            Self::BinaryOp(_, lhs, rhs) => {
                let mut vars = lhs.get_free_variables();
                vars.extend(rhs.get_free_variables());
                vars
            }

            // 一元操作符 - 收集操作数
            Self::UnaryOp(_, expr, _) => expr.get_free_variables(),

            // 范围操作符 - 收集起始、结束和步长
            Self::RangeOp(_, start, end, step) => {
                let mut vars = start.get_free_variables();
                vars.extend(end.get_free_variables());
                if let Some(step_expr) = step {
                    vars.extend(step_expr.get_free_variables());
                }
                vars
            }

            // 管道操作符 - 收集左右表达式
            Self::Pipe(_, left, right) => {
                let mut vars = left.get_free_variables();
                vars.extend(right.get_free_variables());
                vars
            }

            // 索引和属性访问 - 收集对象和索引/属性
            Self::Index(obj, index) => {
                let mut vars = obj.get_free_variables();
                vars.extend(index.get_free_variables());
                vars
            }
            Self::Property(obj, prop) => {
                let mut vars = obj.get_free_variables();
                vars.extend(prop.get_free_variables());
                vars
            }

            // 集合类型 - 收集所有元素
            Self::List(items) => items.iter().fold(HashSet::new(), |mut vars, item| {
                vars.extend(item.get_free_variables());
                vars
            }),
            Self::Map(items) => {
                items.iter().fold(HashSet::new(), |mut vars, (_, value)| {
                    // 键是字符串字面量，不是变量
                    vars.extend(value.get_free_variables());
                    vars
                })
            }
            Self::HMap(items) => {
                items.iter().fold(HashSet::new(), |mut vars, (_, value)| {
                    // 键是字符串字面量，不是变量
                    vars.extend(value.get_free_variables());
                    vars
                })
            }

            // 控制流结构
            Self::If(cond, true_expr, false_expr) => {
                let mut vars = cond.get_free_variables();
                vars.extend(true_expr.get_free_variables());
                vars.extend(false_expr.get_free_variables());
                vars
            }

            Self::While(cond, body) => {
                let mut vars = cond.get_free_variables();
                vars.extend(body.get_free_variables());
                vars
            }

            Self::Loop(body) => body.get_free_variables(),

            Self::For(.., body) => body.get_free_variables(),

            Self::Match(value, branches) => {
                let mut vars = value.get_free_variables();
                for (_, expr) in branches.iter() {
                    vars.extend(expr.get_free_variables());
                }
                vars
            }

            // 函数和应用
            Self::Apply(func, args) => {
                let mut vars = func.get_free_variables();
                for arg in args.iter() {
                    vars.extend(arg.get_free_variables());
                }
                vars
            }

            Self::Command(cmd, args) | Self::CommandRaw(cmd, args) => {
                let mut vars = cmd.get_free_variables();
                for arg in args.iter() {
                    vars.extend(arg.get_free_variables());
                }
                vars
            }

            Self::Function(_, params, _, body, decorators) => {
                let mut vars = body.get_free_variables();
                // 移除参数
                for (param, _) in params {
                    vars.remove(param);
                }
                // 收集装饰器中的自由变量
                for (_, decorator_args) in decorators {
                    if let Some(args) = decorator_args {
                        for arg in args {
                            vars.extend(arg.get_free_variables());
                        }
                    }
                }
                vars
            }

            // 链式调用
            Self::Chain(base, calls) => {
                let mut vars = base.get_free_variables();
                for call in calls {
                    vars.extend(call.args.iter().fold(HashSet::new(), |mut acc, arg| {
                        acc.extend(arg.get_free_variables());
                        acc
                    }));
                }
                vars
            }

            Self::PipeMethod(_, args) => args.iter().fold(HashSet::new(), |mut vars, arg| {
                vars.extend(arg.get_free_variables());
                vars
            }),

            // 其他表达式
            Self::Declare(_, expr) => expr.get_free_variables(),
            Self::Assign(_, expr) => expr.get_free_variables(),
            Self::DestructureAssign(_, expr) => expr.get_free_variables(),
            Self::Return(expr) => expr.get_free_variables(),
            Self::Break(expr) => expr.get_free_variables(),
            Self::Block(exprs) => exprs.iter().fold(HashSet::new(), |mut vars, expr| {
                vars.extend(expr.get_free_variables());
                vars
            }),

            Self::Catch(expr, _, handler) => {
                let mut vars = expr.get_free_variables();
                if let Some(handler_expr) = handler {
                    vars.extend(handler_expr.get_free_variables());
                }
                vars
            }
            _ => HashSet::new(), // Del是删除语句，没有自由变量
        }
    }
}

fn glob_expand(s: &str) -> Vec<String> {
    let mut elist = vec![];
    if let Some(g) = glob(s).ok() {
        for entry in g {
            if let Ok(p) = entry {
                elist.push(p.to_string_lossy().to_string())
            }
        }
    }
    elist
}
pub fn ifs_split(s: &str, env: &mut Environment) -> Vec<String> {
    let ifs = match ifs_contains(IFS_FOR, env) {
        true => env.get("IFS"),
        _ => None,
    };
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

pub fn execute_iteration<I>(
    var_name: String,
    index_name: Option<String>,
    iterator: I,
    count: usize,
    body: &Expression,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError>
where
    I: Iterator<Item = Expression> + 'static,
{
    // 设置循环状态
    let last_iter = state.take_iter();
    let is_last_in_loop = state.contains(State::IN_FOR_LOOP);
    state.set(State::IN_FOR_LOOP);
    state.set_iter(var_name, index_name, Box::new(iterator));

    let r = if state.contains(State::IN_ASSIGN) {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            if let Err(_) = state.pop_iter() {
                break;
            };
            match body.eval_mut(state, env, depth) {
                Ok(result) => results.push(result),
                Err(RuntimeError {
                    kind: RuntimeErrorKind::EarlyBreak(v),
                    ..
                }) => {
                    results.push(v);
                    break;
                }
                Err(RuntimeError {
                    kind: RuntimeErrorKind::IteratorExhausted(_),
                    ..
                }) => break, // 循环正常结束
                Err(e) => {
                    state.clear_iter();
                    if is_last_in_loop {
                        if let Some((var_name, index_name, iterator)) = last_iter {
                            state.set_iter(var_name, index_name, iterator);
                        }
                    } else {
                        state.clear(State::IN_FOR_LOOP);
                        state.clear_local_var();
                    }
                    return Err(e);
                }
            }
        }
        Ok(Expression::from(results))
    } else {
        for _ in 0..count {
            if let Err(_) = state.pop_iter() {
                break;
            }
            match body.eval_mut(state, env, depth) {
                Ok(_) => {}
                Err(RuntimeError {
                    kind: RuntimeErrorKind::EarlyBreak(_v),
                    ..
                }) => break,
                Err(RuntimeError {
                    kind: RuntimeErrorKind::IteratorExhausted(_),
                    ..
                }) => break,
                Err(e) => {
                    state.clear_iter();
                    if is_last_in_loop {
                        if let Some((var_name, index_name, iterator)) = last_iter {
                            state.set_iter(var_name, index_name, iterator);
                        }
                    } else {
                        state.clear(State::IN_FOR_LOOP);
                        state.clear_local_var();
                    }
                    return Err(e);
                }
            }
        }
        Ok(Expression::None)
    };

    // 清理循环状态
    state.clear_iter();
    if is_last_in_loop {
        if let Some((var_name, index_name, iterator)) = last_iter {
            state.set_iter(var_name, index_name, iterator);
        }
    } else {
        state.clear(State::IN_FOR_LOOP);
        state.clear_local_var(); //clear here instead of in every block. for efficency and index secure.
        state.take_iter();
    }
    r
}
