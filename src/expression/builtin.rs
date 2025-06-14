use super::Expression;
use crate::{Environment, LmError};
use std::fmt;

// 内置函数结构（显示优化）
#[derive(Clone)]
pub struct Builtin {
    pub name: String,
    pub body: fn(&Vec<Expression>, &mut Environment) -> Result<Expression, LmError>,
    pub help: String,
    pub hint: String,
}

impl fmt::Debug for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Builtin@{}", self.name)
    }
}
impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "builtin@{}", self.name)
    }
}
impl PartialEq for Builtin {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

// builtin
impl Expression {
    pub fn builtin(
        name: impl ToString,
        body: fn(&Vec<Expression>, &mut Environment) -> Result<Expression, LmError>,
        help: impl ToString,
        param_hint: impl ToString,
    ) -> Self {
        Self::Builtin(Builtin {
            name: name.to_string(),
            body,
            help: help.to_string(),
            hint: param_hint.to_string(),
        })
    }

    pub fn new(x: impl Into<Self>) -> Self {
        // dbg!("---- new exp");
        x.into()
    }
}
