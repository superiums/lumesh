// 运算符重载（内存优化）

use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use crate::RuntimeError;

use super::Expression;

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, Sub, SubAssign};

impl Add for Expression {
    type Output = Result<Self, RuntimeError>;

    fn add(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // 数值运算
            (Self::Integer(m), Self::Integer(n)) => m
                .checked_add(n)
                .map(Self::Integer)
                .ok_or_else(|| RuntimeError::Overflow(format!("{} + {}", m, n))),
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 + n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m + n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m + n)),

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
            // to-list
            (Self::Integer(m), Self::List(b)) => {
                // 将列表内部元素求和
                let sum: i64 = b
                    .as_ref()
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
                    .as_ref()
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

            // 字符串拼接
            (Self::String(m), Self::String(n)) => Ok(Self::String(m + &n)),
            (Self::String(m), Self::Integer(n)) => Ok(Self::String(m + &n.to_string())),
            (Self::String(m), Self::Float(n)) => Ok(Self::String(m + &n.to_string())),

            (Self::String(m), Self::List(b)) => {
                let concatenated: String = b
                    .as_ref()
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

            // range
            (Self::Range(a), Self::Integer(b)) if b >= 0 => {
                Ok(Expression::Range(a.start..a.end + b))
            }
            (Self::Range(a), Self::Integer(b)) => Ok(Expression::Range((a.start + b)..a.end)),

            // 列表合并
            (Self::List(a), Self::List(b)) => {
                // let mut new_list = a.as_ref().clone(); // Clone the list
                // new_list.extend(b.as_ref().iter().cloned()); // Extend with the second list
                // Ok(Self::List(Rc::new(new_list)))
                Self::List(a).list_append(b)
            }
            (Self::List(a), other) => Self::List(a).list_push(other),

            // 映射合并

            // Map merging
            (Self::HMap(a), Self::HMap(b)) => {
                Self::HMap(a).map_append(b)
                // let mut new_map = a.as_ref().clone();
                // new_map.extend(b.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));

                // Ok(Self::Map(Rc::new(new_map)))
            }
            (Self::HMap(a), other) => {
                Self::HMap(a).map_insert(other.to_string(), other)
                // let mut new_map = a.as_ref().clone();
                // new_map.insert(other.to_string(), other); // Insert the other element
                // Ok(Self::Map(Rc::new(new_map)))
            }
            (Self::Map(a), Self::Map(b)) => Self::Map(a).bmap_append(b),
            (Self::Map(a), other) => Self::Map(a).map_insert(other.to_string(), other),

            // (Self::Map(mut a), Self::Integer(n)) => {
            //     a.insert(n.to_string(), Self::Integer(n));
            //     Ok(Self::Map(a))
            // }
            // (Self::Map(mut a), Self::Float(n)) => {
            //     a.insert(n.to_string(), Self::Float(n));
            //     Ok(Self::Map(a))
            // }

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
            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "+".into(),
                format!(
                    "Cannot add {}:{} and {}:{}",
                    m,
                    m.type_name(),
                    n,
                    n.type_name()
                ),
            )),
        }
    }
}

impl Sub for Expression {
    type Output = Result<Self, RuntimeError>;

    fn sub(self, other: Self) -> Result<Self, RuntimeError> {
        match (self, other) {
            // 数值运算
            (Self::Integer(m), Self::Integer(n)) => m
                .checked_sub(n)
                .map(Self::Integer)
                .ok_or_else(|| RuntimeError::Overflow(format!("{} - {}", m, n))),
            (Self::Integer(m), Self::Float(n)) => Ok(Self::Float(m as f64 - n)),
            (Self::Float(m), Self::Integer(n)) => Ok(Self::Float(m - n as f64)),
            (Self::Float(m), Self::Float(n)) => Ok(Self::Float(m - n)),
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
                // 字符串首尾截取
                // let n = n as usize;
                if n >= 0 {
                    if m.len() >= n as usize {
                        let l = m.len() - n as usize;
                        Ok(Self::String(m[..l].to_string()))
                    } else {
                        Ok(Self::String("".to_owned()))
                    }
                } else {
                    let l = -n as usize;
                    if l <= m.len() {
                        Ok(Self::String(m[l..].to_string()))
                    } else {
                        Ok(Self::String("".to_owned()))
                    }
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

            (Self::Range(a), Self::Integer(b)) if b >= 0 => {
                Ok(Expression::Range(a.start..a.end - b))
            }
            (Self::Range(a), Self::Integer(b)) => Ok(Expression::Range(a.start - b..a.end)),

            (Self::List(a), Self::List(b)) => {
                if Rc::ptr_eq(&a, &b) {
                    Ok(Self::List(Rc::new(Vec::new()))) // Clear the list if they are the same
                } else {
                    let mut a_items = a.as_ref().to_vec(); // Clone items directly into a new Vec
                    let b_items = b.as_ref().to_vec(); // Use a HashSet for faster lookups
                    a_items.retain(|x| !b_items.contains(x)); // Remove items in b from a
                    Ok(Self::List(Rc::new(a_items)))
                }
            }

            (Self::List(a), value) => {
                let pos = a.as_ref().iter().position(|x| *x == value);

                if let Some(pos) = pos {
                    // Create a new Vec without the element at the found position
                    let mut a_items: Vec<_> = a.as_ref().to_vec();
                    a_items.remove(pos);
                    Ok(Self::List(Rc::new(a_items)))
                } else {
                    Ok(Self::List(a))
                }
            }

            // Map operations
            (Self::HMap(a), Self::HMap(b)) => {
                // 如果两个 Rc 指向同一个 HashMap，返回一个新的空 HashMap
                if Rc::ptr_eq(&a, &b) {
                    return Ok(Self::HMap(Rc::new(HashMap::new())));
                }
                // 创建一个新的 HashMap，直接从 a 中移除 b 中的键
                let mut a_map = a.as_ref().clone(); // 只在这里克隆一次
                for key in b.as_ref().keys() {
                    a_map.remove(key); // 从 a_map 中移除 b_map 的键
                }
                Ok(Self::from(a_map))
            }

            (Self::HMap(a), Self::Symbol(key) | Self::String(key)) => {
                let mut new_map = a.as_ref().clone();
                new_map.remove(&key);
                Ok(Self::from(new_map))
            }
            // BMap
            (Self::Map(a), Self::Map(b)) => {
                // 如果两个 Rc 指向同一个 HashMap，返回一个新的空 HashMap
                if Rc::ptr_eq(&a, &b) {
                    return Ok(Self::Map(Rc::new(BTreeMap::new())));
                }
                // 创建一个新的 HashMap，直接从 a 中移除 b 中的键
                let mut a_map = a.as_ref().clone(); // 只在这里克隆一次
                for key in b.as_ref().keys() {
                    a_map.remove(key); // 从 a_map 中移除 b_map 的键
                }
                Ok(Self::from(a_map))
            }

            (Self::Map(a), Self::Symbol(key) | Self::String(key)) => {
                let mut new_map = a.as_ref().clone();
                new_map.remove(&key);
                Ok(Self::from(new_map))
            }

            // 其他情况
            (n, m) => Err(RuntimeError::CommandFailed2(
                "-".into(),
                format!(
                    "Cannot subtract {}:{} from {}:{}",
                    m,
                    m.type_name(),
                    n,
                    n.type_name()
                ),
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

            // string
            (Self::String(m), Self::Integer(n)) => {
                // 将字符串重复 n 次
                Ok(Self::String(m.repeat(n as usize)))
            }

            // list
            (Self::List(a), Self::List(b)) => {
                // 矩阵乘法
                // 假设 a 是 m x n 矩阵，b 是 n x p 矩阵
                let a_rows = a.as_ref().len();
                let a_cols = if a_rows > 0 {
                    match &a.as_ref()[0] {
                        Self::List(inner) => inner.as_ref().len(),
                        _ => 0,
                    }
                } else {
                    0
                };
                let b_cols = if !b.as_ref().is_empty() {
                    match &b.as_ref()[0] {
                        Self::List(inner) => inner.as_ref().len(),
                        _ => 0,
                    }
                } else {
                    0
                };

                if a_cols != b.as_ref().len() {
                    return Err(RuntimeError::CommandFailed2(
                        "*".into(),
                        format!(
                            "Matrix dimensions do not match for multiplication: {}x{} and {}x{}",
                            a_rows,
                            a_cols,
                            b.as_ref().len(),
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
                            let a_value = match &a.as_ref()[i] {
                                Self::List(inner) => match inner.as_ref().get(k) {
                                    Some(val) => match val {
                                        Self::Integer(v) => *v as f64,
                                        Self::Float(v) => *v,
                                        _ => 0.0,
                                    },
                                    None => 0.0,
                                },
                                _ => 0.0,
                            };
                            let b_value = match &b.as_ref()[k] {
                                Self::List(inner) => match inner.as_ref().get(j) {
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
                    result.push(Self::from(row_result));
                }
                Ok(Self::from(result))
            }

            (Self::List(a), value) => {
                let mut new_list = Vec::new();
                let n = match value {
                    Self::Integer(n) => n as f64, // 将整数转换为浮点数
                    Self::Float(n) => n,
                    _ => {
                        return Err(RuntimeError::CommandFailed2(
                            "*".into(),
                            format!("Cannot multiply by non-numeric value {:?}", value),
                        ));
                    }
                };

                for element in a.as_ref().iter() {
                    match element {
                        Self::Integer(val) => new_list.push(Self::Float(*val as f64 * n)),
                        Self::Float(val) => new_list.push(Self::Float(val * n)),
                        _ => {
                            return Err(RuntimeError::CommandFailed2(
                                "*".into(),
                                format!("Cannot multiply non-numeric element {:?}", element),
                            ));
                        }
                    }
                }
                Ok(Self::from(new_list))
            }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "*".into(),
                format!(
                    "Cannot multiply {}:{} and {}:{}",
                    m,
                    m.type_name(),
                    n,
                    n.type_name()
                ),
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
                        "/".into(),
                        format!("Cannot convert string `{}` to integer", m),
                    )),
                }
            }
            (Self::Float(n), Self::String(m)) => {
                // 尝试将字符串转换为浮点数
                match m.parse::<f64>() {
                    Ok(num) => Ok(Self::Float(n / num)),
                    Err(_) => Err(RuntimeError::CommandFailed2(
                        "/".into(),
                        format!("Cannot convert string `{}` to float", m),
                    )),
                }
            }

            // 列表类型
            (Self::List(a), value) => {
                let divisor = match value {
                    Self::Integer(n) => n as f64,
                    Self::Float(n) => n,
                    _ => {
                        return Err(RuntimeError::CommandFailed2(
                            "/".into(),
                            format!("Cannot divide by non-numeric value {:?}", value),
                        ));
                    }
                };

                let new_list: Result<Vec<Self>, RuntimeError> = a
                    .as_ref()
                    .iter()
                    .map(|element| match element {
                        Self::Integer(val) => Ok(Self::Float(*val as f64 / divisor)),
                        Self::Float(val) => Ok(Self::Float(val / divisor)),
                        _ => Err(RuntimeError::CommandFailed2(
                            "/".into(),
                            format!("Cannot divide non-numeric element {:?}", element),
                        )),
                    })
                    .collect();

                new_list.map(Self::from) // 将 Result<Vec<Self>, RuntimeError> 转换为 Result<Self, RuntimeError>
            }

            // 其他情况
            (m, n) => Err(RuntimeError::CommandFailed2(
                "/".into(),
                format!(
                    "Cannot divide {}:{} by {}:{}",
                    m,
                    m.type_name(),
                    n,
                    n.type_name()
                ),
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

// impl<T> Index<T> for Expression
// where
//     T: Into<Expression>,
// {
//     type Output = Expression;

//     fn index(&self, idx: T) -> &Self::Output {
//         match (self, idx.into()) {
//             // 处理 Map 索引
//             (Expression::Map(map), Expression::Symbol(name)) => {
//                 map.as_ref().get(&name).unwrap_or(&Expression::None)
//             }
//             (Expression::Map(map), Expression::String(name)) => {
//                 map.as_ref().get(&name).unwrap_or(&Expression::None)
//             }

//             // 处理 List 索引
//             (Expression::List(list), Expression::Integer(n))
//                 if n >= 0 && (n as usize) < list.as_ref().len() =>
//             {
//                 &list.as_ref()[n as usize]
//             }

//             // 其他情况返回 None
//             _ => &Expression::None,
//         }
//     }
// }

// impl Ord for Expression {
//     fn cmp(&self, other: &Self) -> Ordering {
//         match (self, other) {
//             _ => Ordering::Equal,
//         }
//     }
// }
