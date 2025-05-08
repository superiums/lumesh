// 运算符重载（内存优化）

use std::cmp::Ordering;

use crate::RuntimeError;

use super::Expression;

use std::ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Rem, Sub, SubAssign};

impl Add for Expression {
    type Output = Result<Self, RuntimeError>;

    fn add(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // num
            (Self::Integer(m), Self::Integer(n)) => match m.checked_add(n) {
                Some(i) => Ok(Self::Integer(i)),
                None => Err(RuntimeError::Overflow(format!("{} + {}", m, n))), // 溢出处理
            },
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 + n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m + n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m + n)),

            // string
            (Self::String(m), Self::String(n)) => Ok(Self::String(m + &n)),
            (Self::String(m), Self::Integer(n)) => Ok(Self::String(m + &n.to_string())),
            (Self::String(m), Self::Float(n)) => Ok(Self::String(m + &n.to_string())),
            // to-string
            (Self::Integer(m), Self::String(n)) => {
                // 尝试将字符串转换为整数
                match n.parse::<i64>() {
                    Ok(n) => Ok(Self::Integer(m + n)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "+".into(),
                        format!("Cannot convert string `{}` to integer", n),
                    )), // 转换失败
                }
            }
            (Self::Float(m), Self::String(n)) => {
                // 尝试将字符串转换为浮点数
                match n.parse::<f64>() {
                    Ok(n) => Ok(Self::Float(m + n)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "+".into(),
                        format!("Cannot convert string `{}` to integer", n),
                    )), // 转换失败
                }
            }

            // list
            (Self::List(mut a), Self::List(b)) => {
                a.extend(b);
                Ok(Self::List(a))
            }
            (Self::List(mut a), Self::Integer(n)) => {
                a.push(Self::Integer(n));
                Ok(Self::List(a))
            }
            (Self::List(mut a), Self::String(n)) => {
                a.push(Self::String(n));
                Ok(Self::List(a))
            }
            (Self::List(mut a), Self::Float(n)) => {
                a.push(Self::Float(n));
                Ok(Self::List(a))
            }
            // to-list
            (Self::Integer(m), Self::List(b)) => {
                // 将列表内部元素求和
                let sum: i64 = b
                    .iter()
                    .filter_map(|x| {
                        if let Self::Integer(n) = x {
                            Some(*n)
                        } else {
                            None // 只处理整数
                        }
                    })
                    .sum();
                Ok(Self::Integer(m + sum))
            }
            (Self::Float(m), Self::List(b)) => {
                // 将列表内部元素求和
                let sum: f64 = b
                    .iter()
                    .filter_map(|x| {
                        if let Self::Float(n) = x {
                            Some(*n)
                        } else if let Self::Integer(n) = x {
                            Some(*n as f64)
                        } else {
                            None // 只处理整数和浮点数
                        }
                    })
                    .sum();
                Ok(Self::Float(m + sum))
            }
            (Self::String(m), Self::List(b)) => {
                let concatenated: String = b
                    .iter()
                    .filter_map(|x| {
                        if let Self::String(n) = x {
                            Some(n.clone())
                        } else {
                            None // 只处理字符串
                        }
                    })
                    .collect();
                Ok(Self::String(m + &concatenated))
            }

            // bytes
            (Self::Bytes(mut a), Self::Bytes(b)) => {
                a.extend(b);
                Ok(Self::Bytes(a))
            }
            (Self::Bytes(mut a), Self::String(n)) => {
                a.extend(n.into_bytes());
                Ok(Self::Bytes(a))
            }
            // (Self::Bytes(mut a), Self::Integer(n)) => {
            //     a.extend(n.to_string().into_bytes());
            //     Ok(Self::Bytes(a))
            // }
            // (Self::Bytes(mut a), Self::Float(n)) => {
            //     a.extend(n.to_string().into_bytes());
            //     Ok(Self::Bytes(a))
            // }

            // map
            (Self::Map(mut a), Self::Map(b)) => {
                a.extend(b);
                Ok(Self::Map(a))
            }
            (Self::Map(mut a), Self::Symbol(n) | Self::String(n)) => {
                a.insert(n.clone(), Self::String(n));
                Ok(Self::Map(a))
            }
            // (Self::Map(mut a), Self::Integer(n)) => {
            //     a.insert(n.to_string(), Self::Integer(n));
            //     Ok(Self::Map(a))
            // }
            // (Self::Map(mut a), Self::Float(n)) => {
            //     a.insert(n.to_string(), Self::Float(n));
            //     Ok(Self::Map(a))
            // }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "+".into(),
                format!("Cannot add {} and {}", m.type_name(), n.type_name()),
            )),
        }
    }
}

impl Sub for Expression {
    type Output = Result<Self, RuntimeError>;

    fn sub(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // num
            (Self::Integer(m), Self::Integer(n)) => match m.checked_sub(n) {
                Some(i) => Ok(Self::Integer(i)),
                None => Err(RuntimeError::Overflow(format!("{} - {}", m, n))), // 溢出处理
            },
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 - n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m - n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m - n)),

            // string
            (Self::String(m), Self::String(n)) => {
                // 从字符串中移除另一个字符串
                if let Some(pos) = m.find(&n) {
                    let new_string = m[..pos].to_string() + &m[pos + n.len()..];
                    Ok(Self::String(new_string))
                } else {
                    Ok(Self::String(m))
                }
            }
            (Self::String(m), Self::Integer(n)) => {
                // 将整数转换为字符串并从前一个字符串中移除
                let n_str = n.to_string();
                if let Some(pos) = m.find(&n_str) {
                    let new_string = m[..pos].to_string() + &m[pos + n_str.len()..];
                    Ok(Self::String(new_string))
                } else {
                    Ok(Self::String(m))
                }
            }
            (Self::String(m), Self::Float(n)) => {
                // 将整数转换为字符串并从前一个字符串中移除
                let n_str = n.to_string();
                if let Some(pos) = m.find(&n_str) {
                    let new_string = m[..pos].to_string() + &m[pos + n_str.len()..];
                    Ok(Self::String(new_string))
                } else {
                    Ok(Self::String(m))
                }
            }
            // to-string
            (Self::Integer(m), Self::String(n)) => {
                // 尝试将字符串转换为整数
                match n.parse::<i64>() {
                    Ok(n) => Ok(Self::Integer(m - n)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "-".into(),
                        format!("Cannot convert string `{}` to integer", n),
                    )), // 转换失败
                }
            }
            (Self::Float(m), Self::String(n)) => {
                // 尝试将字符串转换为浮点数
                match n.parse::<f64>() {
                    Ok(n) => Ok(Self::Float(m - n)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "-".into(),
                        format!("Cannot convert string `{}` to integer", n),
                    )), // 转换失败
                }
            }

            // list
            (Self::List(mut a), Self::List(b)) => {
                // 从列表中移除另一个列表的元素
                for item in b {
                    a.retain(|x| x != &item);
                }
                Ok(Self::List(a))
            }
            (Self::List(mut a), Self::Integer(n)) => {
                // 从列表中移除指定的整数
                if let Some(pos) = a
                    .iter()
                    .position(|x| matches!(x, Self::Integer(val) if *val == n))
                {
                    a.remove(pos);
                    Ok(Self::List(a))
                } else {
                    Ok(Self::List(a))
                }
            }
            (Self::List(mut a), Self::String(n)) => {
                // 从列表中移除指定的字符串
                if let Some(pos) = a
                    .iter()
                    .position(|x| matches!(x, Self::String(val) if val == &n))
                {
                    a.remove(pos);
                    Ok(Self::List(a))
                } else {
                    Ok(Self::List(a))
                }
            }
            // to-list
            // ...non

            // map
            (Self::Map(mut a), Self::Map(b)) => {
                // 从映射中移除另一个映射的属性
                for key in b.keys() {
                    if a.get(key) == b.get(key) {
                        a.remove(key);
                    }
                }
                Ok(Self::Map(a))
            }
            (Self::Map(mut a), Self::Symbol(key) | Self::String(key)) => {
                // 从映射中移除指定的属性
                if a.remove(&key).is_some() {
                    Ok(Self::Map(a))
                } else {
                    Ok(Self::Map(a))
                }
            }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "-".into(),
                format!("Cannot subtract {} from {}", n.type_name(), m.type_name()),
            )),
        }
    }
}

impl Mul for Expression {
    type Output = Result<Self, RuntimeError>;

    fn mul(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // num
            (Self::Integer(m), Self::Integer(n)) => match m.checked_mul(n) {
                Some(result) => Ok(Self::Integer(result)),
                None => Err(RuntimeError::Overflow(format!(
                    "Integer overflow when multiplying {} and {}",
                    m, n
                ))),
            },
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 * n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m * n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m * n)),

            // string
            (Self::String(m), Self::Integer(n)) => {
                // 将字符串重复 n 次
                Ok(Self::String(m.repeat(n as usize)))
            }
            // to-string
            (Self::Integer(n), Self::String(m)) => {
                // 尝试将字符串转换为整数
                match m.parse::<i64>() {
                    Ok(num) => Ok(Self::Integer(n * num)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!("Cannot convert string `{}` to integer", m),
                    )),
                }
            }
            (Self::Float(n), Self::String(m)) => {
                // 尝试将字符串转换为浮点数
                match m.parse::<f64>() {
                    Ok(num) => Ok(Self::Float(n * num)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!("Cannot convert string `{}` to float", m),
                    )),
                }
            }

            // list
            (Self::List(a), Self::Integer(n)) => {
                // 将列表中的每个元素乘以 n
                let mut new_list = Vec::new();
                for element in a {
                    match element {
                        Self::Integer(val) => new_list.push(Self::Integer(val * n)),
                        Self::Float(val) => new_list.push(Self::Float(val * n as f64)),
                        _ => {
                            return Err(RuntimeError::CommandFailed2(
                                "*".into(),
                                format!("Cannot multiply non-numeric element {:?}", element),
                            ));
                        }
                    }
                }
                Ok(Self::List(new_list))
            }
            (Self::List(a), Self::Float(n)) => {
                // 将列表中的每个元素乘以 n
                let mut new_list = Vec::new();
                for element in a {
                    match element {
                        Self::Integer(val) => new_list.push(Self::Float(val as f64 * n)),
                        Self::Float(val) => new_list.push(Self::Float(val * n)),
                        _ => {
                            return Err(RuntimeError::CommandFailed2(
                                "*".into(),
                                format!("Cannot multiply non-numeric element {:?}", element),
                            ));
                        }
                    }
                }
                Ok(Self::List(new_list))
            }

            (Self::List(a), Self::List(b)) => {
                // 矩阵乘法
                // 假设 a 是 m x n 矩阵，b 是 n x p 矩阵
                let a_rows = a.len();
                let a_cols = if a_rows > 0 {
                    match &a[0] {
                        Self::List(inner) => inner.len(),
                        _ => 0,
                    }
                } else {
                    0
                };
                let b_cols = if b.len() > 0 {
                    match &b[0] {
                        Self::List(inner) => inner.len(),
                        _ => 0,
                    }
                } else {
                    0
                };

                if a_cols != b.len() {
                    return Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!(
                            "Matrix dimensions do not match for multiplication: {}x{} and {}x{}",
                            a_rows,
                            a_cols,
                            b.len(),
                            b_cols
                        ),
                    ));
                }

                let mut result = Vec::new();
                for i in 0..a_rows {
                    let mut row_result = Vec::new();
                    for j in 0..b_cols {
                        let mut sum = 0.0; // 使用浮点数进行计算
                        for k in 0..a_cols {
                            let a_value = match &a[i] {
                                Self::List(inner) => match inner.get(k) {
                                    Some(val) => match val {
                                        Self::Integer(v) => *v as f64,
                                        Self::Float(v) => *v,
                                        _ => 0.0,
                                    },
                                    None => 0.0,
                                },
                                _ => 0.0,
                            };
                            let b_value = match &b[k] {
                                Self::List(inner) => match inner.get(j) {
                                    Some(val) => match val {
                                        Self::Integer(v) => *v as f64,
                                        Self::Float(v) => *v,
                                        _ => 0.0,
                                    },
                                    None => 0.0,
                                },
                                _ => 0.0,
                            };
                            sum += a_value * b_value;
                        }
                        row_result.push(Self::Float(sum));
                    }
                    result.push(Self::List(row_result));
                }
                Ok(Self::List(result))
            }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "*".into(),
                format!("Cannot multiply {} and {}", n.type_name(), m.type_name()),
            )),
        }
    }
}

impl Div for Expression {
    type Output = Result<Self, RuntimeError>;

    fn div(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // 数值类型
            (l, Self::Integer(0) | Self::Float(0.0)) => Err(RuntimeError::CustomError(format!(
                "can't divide {} by zero",
                l
            ))),
            (l, Self::String(s)) if s == "0" => Err(RuntimeError::CustomError(format!(
                "can't divide {} by zero",
                l
            ))),
            (Self::Integer(m), Self::Integer(n)) => Ok(Self::Integer(m / n)),
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 / n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m / n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m / n)),

            // to-string
            (Self::Integer(n), Self::String(m)) => {
                // 尝试将字符串转换为整数
                match m.parse::<i64>() {
                    Ok(num) => Ok(Self::Integer(n / num)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!("Cannot convert string `{}` to integer", m),
                    )),
                }
            }
            (Self::Float(n), Self::String(m)) => {
                // 尝试将字符串转换为浮点数
                match m.parse::<f64>() {
                    Ok(num) => Ok(Self::Float(n / num)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!("Cannot convert string `{}` to float", m),
                    )),
                }
            }

            // 列表类型
            (Self::List(a), Self::Integer(n)) => {
                let new_list: Result<Vec<Self>, RuntimeError> = a
                    .into_iter()
                    .map(|element| match element {
                        Self::Integer(val) => Ok(Self::Float(val as f64 / n as f64)),
                        Self::Float(val) => Ok(Self::Float(val / n as f64)),
                        _ => Err(RuntimeError::CommandFailed2(
                            "/".into(),
                            format!("Cannot divide non-numeric element {:?}", element),
                        )),
                    })
                    .collect();
                new_list.map(Self::List) // 将 Result<Vec<Self>, RuntimeError> 转换为 Result<Self, RuntimeError>
            }
            (Self::List(a), Self::Float(n)) => {
                let new_list: Result<Vec<Self>, RuntimeError> = a
                    .into_iter()
                    .map(|element| match element {
                        Self::Integer(val) => Ok(Self::Float(val as f64 / n)),
                        Self::Float(val) => Ok(Self::Float(val / n)),
                        _ => Err(RuntimeError::CommandFailed2(
                            "/".into(),
                            format!("Cannot divide non-numeric element {:?}", element),
                        )),
                    })
                    .collect();
                new_list.map(Self::List) // 将 Result<Vec<Self>, RuntimeError> 转换为 Result<Self, RuntimeError>
            }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "/".into(),
                format!("Cannot divide {} by {}", m.type_name(), n.type_name()),
            )),
        }
    }
}

impl Neg for Expression {
    type Output = Expression;
    fn neg(self) -> Self::Output {
        match self {
            Self::Integer(n) => Self::Integer(-n),
            Self::Float(n) => Self::Float(-n),
            Self::Boolean(b) => Self::Boolean(!b),
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
            (Self::Map(a), Self::Map(b)) => a.keys().partial_cmp(b.keys()),
            _ => None,
        }
    }
}
