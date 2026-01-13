use super::Builtin;
use super::eval::State;
use crate::expression::cmd_excutor::handle_command;
use crate::expression::{ChainCall, alias};
use crate::{Environment, Expression, RuntimeError, RuntimeErrorKind, get_builtin};
use std::borrow::Cow;

/// 执行
impl Expression {
    pub fn eval_symbo_with_domain(
        &self,
        module: String,
        function: String,
        args: &Vec<Expression>,
        state: &mut State,
        env: &mut Environment,
        depth: usize,
    ) -> Result<Expression, RuntimeError> {
        // 获取当前查找域
        let domains = state.get_lookup_domains();

        if domains.is_empty() {
            // 在当前环境中查找

            let root = env.get(&module);
            let parent = match root.as_ref() {
                Some(Expression::HMap(m)) => m,
                Some(x) => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::SymbolNotModule(
                            module,
                            x.type_name(),
                            "current module".into(),
                        ),
                        self.clone(),
                        depth,
                    ));
                }
                _ => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::SymbolNotDefined(format!("{} in current module", module)),
                        self.clone(),
                        depth,
                    ));
                }
            };
            if let Some(func) = parent.get(&function) {
                state.push_lookup_domain(module);
                let result = self.eval_apply(func, args, state, env, depth + 1);
                state.pop_lookup_domain();
                return result;
            }

            Err(RuntimeError::new(
                RuntimeErrorKind::SymbolNotDefined(format!("{}::{}", module, function)),
                self.clone(),
                depth,
            ))
        } else {
            // 在查找域中查找模块
            let leading = domains.first().unwrap();
            let root = env.get(leading);
            let mut parent = match root.as_ref() {
                Some(Expression::HMap(m)) => m,
                Some(x) => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::SymbolNotModule(
                            leading.to_string(),
                            x.type_name(),
                            "current module".into(),
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
                            ),
                            self.clone(),
                            depth,
                        ));
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::SymbolNotDefined(format!(
                                "{} in module {}",
                                leading, domains[index]
                            )),
                            self.clone(),
                            depth,
                        ));
                    }
                }
            }
            // after got parent
            if let Some(func) = parent.get(&function) {
                state.push_lookup_domain(module);
                let result = self.eval_apply(func, args, state, env, depth + 1);
                state.pop_lookup_domain();
                return result;
            } else {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::SymbolNotDefined(format!(
                        "{} in module {}",
                        function,
                        domains.last().unwrap()
                    )),
                    self.clone(),
                    depth,
                ));
            }
        }
    }
}
