use smallstr::SmallString;

use super::{Environment, Expression, Int};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

impl From<Int> for Expression {
    fn from(x: Int) -> Self {
        Self::Integer(x)
    }
}

impl From<f64> for Expression {
    fn from(x: f64) -> Self {
        Self::Float(x)
    }
}

impl From<&str> for Expression {
    fn from(x: &str) -> Self {
        Self::String(x.to_string())
    }
}

impl From<String> for Expression {
    fn from(x: String) -> Self {
        Self::String(x)
    }
}

impl From<Vec<u8>> for Expression {
    fn from(x: Vec<u8>) -> Self {
        Self::Bytes(x)
    }
}

impl From<bool> for Expression {
    fn from(x: bool) -> Self {
        Self::Boolean(x)
    }
}

impl<T> From<HashMap<SmallString<[u8; 16]>, T>> for Expression
where
    T: Into<Self>,
{
    fn from(map: HashMap<SmallString<[u8; 16]>, T>) -> Self {
        Self::HMap(Rc::new(
            map.into_iter()
                .map(|(name, item)| (name, item.into()))
                .collect::<HashMap<SmallString<[u8; 16]>, Self>>(),
        ))
    }
}

impl<T> From<BTreeMap<SmallString<[u8; 16]>, T>> for Expression
where
    T: Into<Self>,
{
    fn from(map: BTreeMap<SmallString<[u8; 16]>, T>) -> Self {
        Self::Map(Rc::new(
            map.into_iter()
                .map(|(name, item)| (name, item.into()))
                .collect::<BTreeMap<SmallString<[u8; 16]>, Self>>(),
        ))
    }
}

impl<T> From<Vec<T>> for Expression
where
    T: Into<Self>,
{
    fn from(list: Vec<T>) -> Self {
        Self::List(Rc::new(
            list.into_iter()
                .map(|item| item.into())
                .collect::<Vec<Self>>(),
        ))
    }
}

impl From<Environment> for Expression {
    fn from(env: Environment) -> Self {
        Self::from(env.get_bindings_map())
    }
}
