use crate::Expression;
use core::option::Option::None;
use std::collections::BTreeMap;

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

    // pub fn define_builtin(
    //     &mut self,
    //     name: impl ToString,
    //     builtin: fn(Vec<Expression>, &mut Environment) -> Result<Expression, LmError>,
    //     help: impl ToString,
    // ) {
    //     self.define(
    //         &name.to_string(),
    //         Expression::builtin(name.to_string(), builtin, help.to_string()),
    //     )
    // }

    // pub fn set_parent(&mut self, parent: Self) {
    //     self.parent = Some(Box::new(parent));
    // }
    // pub fn get_parent(&self) -> Option<Box<Environment>> {
    //     self.parent.clone()
    // }
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
