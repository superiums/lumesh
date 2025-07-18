use super::catcher::catch_error;
use super::eval::State;
use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    expression::{DestructurePattern, cmd_excutor::expand_home},
    runtime::{IFS_FOR, ifs_contains, load_module},
};
use glob::glob;
use std::{borrow::Cow, collections::HashMap, path::Path, rc::Rc};

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
                if state.contains(State::IN_DECO | State::IN_ASSIGN) {
                    Ok(func)
                } else {
                    Ok(Expression::None)
                }
            }

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

            Expression::Use(alias, module_path) => {
                let mut loaded_modules = HashMap::new();
                load_modules_to_map(&mut loaded_modules, alias, module_path, self, env, depth)?;

                for (module, functions) in loaded_modules.iter() {
                    env.define(module, functions.clone());
                }

                Ok(Expression::None)
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
            Expression::String(str) => {
                let s = expand_home(str.as_ref());
                if s.contains('*') {
                    // glob expansion logic
                    let iterator = glob_expand(&s).into_iter().map(Expression::String);
                    execute_iteration(var, iterator, body, state, env, depth)
                } else {
                    let iterator = ifs_split(&s, env).into_iter().map(Expression::String);
                    execute_iteration(var, iterator, body, state, env, depth)
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
}

fn glob_expand(s: &str) -> Vec<String> {
    let mut elist = vec![];
    for entry in glob(s).unwrap() {
        if let Ok(p) = entry {
            elist.push(p.to_string_lossy().to_string())
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

fn load_modules_to_map(
    result: &mut HashMap<String, Expression>,
    module_alias: &Option<String>,
    module_path: &str,
    // loaded_modules: &mut HashSet<String>,
    context: &Expression,
    env: &mut Environment,
    depth: usize,
) -> Result<(), RuntimeError> {
    let module_name = get_module_name_from_path(module_alias, module_path, context, depth + 1)?;
    if result.contains_key(module_name.as_ref()) {
        return Err(RuntimeError::common(
            "Circular module dependency".into(),
            context.clone(),
            depth,
        ));
    }

    // 读取模块文件
    // let file_path = PathBuf::from(format!("{}.lm", module_path));
    let module_info = load_module(module_path, env)?;

    // 当前导入模块的函数
    result.insert(module_name.into(), Expression::from(module_info.functions));

    // 递归处理依赖的 use 语句
    for (dep_alias, dep_path) in &module_info.use_statements {
        load_modules_to_map(
            result,
            dep_alias,
            dep_path,
            &Expression::Use(dep_alias.clone(), dep_path.clone()),
            env,
            depth + 1,
        )?;
    }

    Ok(())
}

fn get_module_name_from_path<'a>(
    alias: &'a Option<String>,
    module_path: &'a str,
    context: &Expression,
    depth: usize,
) -> Result<Cow<'a, str>, RuntimeError> {
    match alias {
        Some(n) => Ok(n.into()),
        _ => {
            let path = Path::new(module_path);

            // 获取文件名
            match path.file_name() {
                Some(name) => {
                    let fname = name.to_string_lossy();
                    Ok(match fname.split_once('.') {
                        Some((n, _)) => n.to_string().into(),
                        _ => fname.to_string().into(),
                    })
                }
                None => Err(RuntimeError::common(
                    "get filename failed".into(),
                    context.clone(),
                    depth,
                )),
            }
        }
    }
}
