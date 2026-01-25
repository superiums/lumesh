use crate::Expression;
use rustc_hash::FxHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::rc::Rc;

// 定义默认哈希器
pub type DefaultHasher = BuildHasherDefault<FxHasher>;

#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    pub bindings: Rc<HashMap<String, Expression, DefaultHasher>>,
    pub parent: Option<Box<Self>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(HashMap::with_hasher(DefaultHasher::default())),
            parent: None,
        }
    }

    pub fn get(&self, name: &str) -> Option<Expression> {
        match self.bindings.get(name) {
            Some(expr) => Some(expr.clone()),
            None => self.parent.as_ref().and_then(|p| p.get(name)),
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.bindings.contains_key(name) || self.parent.as_ref().is_some_and(|p| p.is_defined(name))
    }

    pub fn undefine(&mut self, name: &str) {
        let bindings = Rc::make_mut(&mut self.bindings);
        bindings.remove(name);
    }

    pub fn define(&mut self, name: &str, expr: Expression) {
        let bindings = Rc::make_mut(&mut self.bindings);
        bindings.insert(name.to_string(), expr);
    }

    pub fn get_parent_mut(&mut self) -> Option<&mut Self> {
        self.parent.as_mut().map(|p| p.as_mut())
    }

    pub fn fork(&self) -> Self {
        Self {
            bindings: Rc::new(HashMap::with_hasher(DefaultHasher::default())),
            parent: Some(Box::new(self.clone())),
        }
    }

    pub fn get_bindings_string(&self) -> HashMap<&String, String> {
        self.bindings
            .iter()
            .filter(|(_, v)| match v {
                Expression::String(..)
                | Expression::Symbol(..)
                | Expression::Integer(..)
                // | Expression::Float(..)
                // | Expression::Boolean(..)
                => true,
                _ => false,
            })
            .map(|(k, v)| (k, v.to_string()))
            // 过滤过长的值以避免参数列表溢出
            .filter(|(_, s)| s.len() <= 1024)
            .collect()
    }

    // pub fn get_bindings_iter(&self) -> impl Iterator<Item = (&String, &Expression)> {
    //     self.bindings.iter()
    // }
    pub fn get_bindings_map(&self) -> HashMap<String, Expression> {
        self.bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_root(&self) -> &Self {
        match self.parent.as_ref() {
            Some(p) => p.get_root(),
            None => self,
        }
    }

    pub fn define_in_root(&mut self, name: &str, expr: Expression) {
        match self.parent.as_mut() {
            Some(p) => p.define_in_root(name, expr),
            None => self.define(name, expr),
        }
    }
    pub fn undefine_in_root(&mut self, name: &str) {
        match self.parent.as_mut() {
            Some(p) => p.undefine_in_root(name),
            None => self.undefine(name),
        }
    }
}
