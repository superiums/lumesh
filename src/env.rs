use crate::{Expression, LmError};
use std::collections::BTreeMap;

const CWD_ENV_VAR: &str = "CWD";

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Environment {
    pub bindings: BTreeMap<String, Expression>,
    parent: Option<Box<Self>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn define_module<A: ToString, B: ToString>(
        &mut self,
        name: A,
        module: impl Into<BTreeMap<B, Expression>>,
    ) {
        let mut result = BTreeMap::new();
        for (key, value) in module.into() {
            result.insert(key.to_string(), value);
        }
        self.define(&name.to_string(), Expression::Map(result));
    }

    pub fn get_cwd(&self) -> String {
        match self.get(CWD_ENV_VAR) {
            Some(Expression::String(path)) => path,
            _ => String::from("/"),
        }
    }

    pub fn set_cwd(&mut self, cwd: impl ToString) {
        self.define(CWD_ENV_VAR, Expression::String(cwd.to_string()));
    }

    pub fn get(&self, name: &str) -> Option<Expression> {
        match self.bindings.get(name) {
            Some(expr) => Some(expr.clone()),
            None => match &self.parent {
                Some(parent) => parent.get(name),
                None => None,
            },
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
            || if let Some(ref parent) = self.parent {
                parent.is_defined(name)
            } else {
                false
            }
    }

    pub fn undefine(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    pub fn define(&mut self, name: &str, expr: Expression) {
        self.bindings.insert(name.to_string(), expr);
    }

    pub fn define_builtin(
        &mut self,
        name: impl ToString,
        builtin: fn(Vec<Expression>, &mut Environment) -> Result<Expression, LmError>,
        help: impl ToString,
    ) {
        self.define(
            &name.to_string(),
            Expression::builtin(name.to_string(), builtin, help.to_string()),
        )
    }

    pub fn set_parent(&mut self, parent: Self) {
        self.parent = Some(Box::new(parent));
    }
    pub fn get_parent(&self) -> Option<Box<Environment>> {
        self.parent.clone()
    }
    pub fn get_parent_mut(&mut self) -> Option<&mut Environment> {
        self.parent.as_mut().map(|p| p.as_mut())
    }
    pub fn fork(&self) -> Self {
        Self {
            bindings: BTreeMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    pub fn get_bindings_map(&self) -> BTreeMap<String, String> {
        self.bindings
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            // This is to prevent environment variables from getting too large.
            // This causes some strange bugs on Linux: mainly it becomes
            // impossible to execute any program because `the argument
            // list is too long`.
            .filter(|(_, s)| s.len() <= 1024)
            .collect::<BTreeMap<String, String>>()
    }
}
