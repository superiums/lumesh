// 运算符重载（内存优化）

use std::cmp::Ordering;

use super::Expression;

use std::ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Rem, Sub, SubAssign};

impl Add for Expression {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        match (self, other) {
            // num
            (Self::Integer(m), Self::Integer(n)) => match m.checked_add(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 + n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m + n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m + n),
            // string
            (Self::String(m), Self::String(n)) => Self::String(m + &n),
            // string + num
            (Self::String(m), Self::Integer(n)) => Self::String(m + &n.to_string()),
            (Self::String(m), Self::Float(n)) => Self::String(m + &n.to_string()),
            // num + string
            (Self::Integer(m), Self::String(n)) => Self::String(m.to_string() + &n),
            (Self::Float(m), Self::String(n)) => Self::String(m.to_string() + &n),
            // other
            (Self::Bytes(mut a), Self::Bytes(b)) => {
                a.extend(b);
                Self::Bytes(a)
            }
            (Self::List(mut a), Self::List(b)) => {
                a.extend(b);
                Self::List(a)
            }
            _ => Self::None,
        }
    }
}

impl Sub for Expression {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_sub(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 - n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m - n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m - n),
            (Self::Map(mut m), Self::String(n)) => match m.remove_entry(&n) {
                Some((_, val)) => val,
                None => Self::None,
            },
            (Self::List(mut m), Self::Integer(n)) if m.len() > n as usize => m.remove(n as usize),
            _ => Self::None,
        }
    }
}

impl Neg for Expression {
    type Output = Expression;
    fn neg(self) -> Self::Output {
        match self {
            Self::Integer(n) => Self::Integer(-n),
            Self::Boolean(b) => Self::Boolean(!b),
            Self::Float(n) => Self::Float(-n),
            _ => Self::None,
        }
    }
}
impl AddAssign for Expression {
    fn add_assign(&mut self, other: Self) {
        *self = match (&self, other) {
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m.to_owned() + n),
            (Self::Integer(m), Self::Float(n)) => Self::Float(m.to_owned() as f64 + n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m.to_owned() + n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m.to_owned() + n),
            _ => Self::None,
        }
    }
}
impl SubAssign for Expression {
    fn sub_assign(&mut self, other: Self) {
        *self = match (&self, other) {
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m.to_owned() - n),
            (Self::Integer(m), Self::Float(n)) => Self::Float(m.to_owned() as f64 - n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m.to_owned() - n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m.to_owned() - n),
            _ => Self::None,
        }
    }
}
impl MulAssign for Expression {
    fn mul_assign(&mut self, other: Self) {
        *self = match (&self, other) {
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m.to_owned() * n),
            (Self::Integer(m), Self::Float(n)) => Self::Float(m.to_owned() as f64 * n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m.to_owned() * n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m.to_owned() * n),
            _ => Self::None,
        }
    }
}
impl DivAssign for Expression {
    fn div_assign(&mut self, other: Self) {
        *self = match (&self, other) {
            (_, Self::Integer(0)) => Self::None,
            (_, Self::Float(0.0)) => Self::None,
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m.to_owned() / n),
            (Self::Integer(m), Self::Float(n)) => Self::Float(m.to_owned() as f64 / n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m.to_owned() / n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m.to_owned() / n),
            _ => Self::None,
        }
    }
}

impl Mul for Expression {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_mul(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 * n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m * n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m * n),
            (Self::String(m), Self::Integer(n)) | (Self::Integer(n), Self::String(m)) => {
                Self::String(m.repeat(n as usize))
            }
            (Self::List(m), Self::Integer(n)) | (Self::Integer(n), Self::List(m)) => {
                let mut result = vec![];
                for _ in 0..n {
                    result.extend(m.clone());
                }
                Self::List(result)
            }
            _ => Self::None,
        }
    }
}

impl Div for Expression {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        if !other.is_truthy() {
            return Self::None;
        }
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => match m.checked_div(n) {
                Some(i) => Self::Integer(i),
                None => Self::None,
            },
            (Self::Integer(m), Self::Float(n)) => Self::Float(m as f64 / n),
            (Self::Float(m), Self::Integer(n)) => Self::Float(m / n as f64),
            (Self::Float(m), Self::Float(n)) => Self::Float(m / n),
            _ => Self::None,
        }
    }
}

impl Rem for Expression {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        match (self, other) {
            (Self::Integer(m), Self::Integer(n)) => Self::Integer(m % n),
            _ => Self::None,
        }
    }
}

impl<T> Index<T> for Expression
where
    T: Into<Self>,
{
    type Output = Self;

    fn index(&self, idx: T) -> &Self {
        match (self, idx.into()) {
            (Self::Map(m), Self::Symbol(name)) | (Self::Map(m), Self::String(name)) => {
                match m.get(&name) {
                    Some(val) => val,
                    None => &Self::None,
                }
            }

            (Self::List(list), Self::Integer(n)) if list.len() > n as usize => &list[n as usize],
            _ => &Self::None,
        }
    }
}

/// PartialOrd实现
impl PartialOrd for Expression {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.partial_cmp(b),
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::Bytes(a), Self::Bytes(b)) => a.partial_cmp(b),
            (Self::List(a), Self::List(b)) => a.partial_cmp(b),
            (Self::Map(a), Self::Map(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}
