use crate::{Environment, Expression, Int, LmError, SliceParams};
use regex_lite::Regex;
use std::{io::ErrorKind, io::Write, path::PathBuf};
// use num_traits::pow;
use crate::excutor::handle_pipes;

/// 二目运算
pub fn handle_binary(
    operator: &String,
    lhs: Box<Expression>,
    rhs: Box<Expression>,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, LmError> {
    return match operator.as_str() {
        "+=" => match *lhs {
            Expression::Symbol(base) => {
                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                left += rhs.eval(env)?;
                env.define(&base, left.clone());
                Ok(left)
            }
            _ => Err(LmError::CustomError(format!(
                "cannot apply {} to  {}:{} and {}:{}",
                operator,
                lhs,
                lhs.type_name(),
                rhs,
                rhs.type_name()
            ))),
        },
        "-=" => match *lhs {
            Expression::Symbol(base) => {
                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                left -= rhs.eval(env)?;
                env.define(&base, left.clone());
                Ok(left)
            }
            _ => Err(LmError::CustomError(format!(
                "cannot apply {} to  {}:{} and {}:{}",
                operator,
                lhs,
                lhs.type_name(),
                rhs,
                rhs.type_name()
            ))),
        },
        "*=" => match *lhs {
            Expression::Symbol(base) => {
                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                left *= rhs.eval(env)?;
                env.define(&base, left.clone());
                Ok(left)
            }
            _ => Err(LmError::CustomError(format!(
                "cannot apply {} to  {}:{} and {}:{}",
                operator,
                lhs,
                lhs.type_name(),
                rhs,
                rhs.type_name()
            ))),
        },
        "/=" => match *lhs {
            Expression::Symbol(base) => {
                let mut left = env.get(&base).unwrap_or(Expression::Integer(0));
                let right = rhs.eval(env)?;
                if !right.is_truthy() {
                    return Err(LmError::CustomError(format!(
                        "can't divide {} by zero",
                        base
                    )));
                };
                left /= right;
                env.define(&base, left.clone());
                Ok(left)
            }
            _ => Err(LmError::CustomError(format!(
                "cannot apply {} to  {}:{} and {}:{}",
                operator,
                lhs,
                lhs.type_name(),
                rhs,
                rhs.type_name()
            ))),
        },
        "&&" => Ok(Expression::Boolean(
            lhs.eval_mut(env, depth + 1)?.is_truthy() && rhs.eval_mut(env, depth + 1)?.is_truthy(),
        )),
        "||" => Ok(Expression::Boolean(
            lhs.eval_mut(env, depth + 1)?.is_truthy() || rhs.eval_mut(env, depth + 1)?.is_truthy(),
        )),
        "|" => {
            let bindings = env.get_bindings_map();
            let (pipe_out, expr_out) =
                handle_pipes(&lhs, &rhs, &bindings, false, None, env, depth)?;
            // dbg!(pipe_out, &expr_out);
            Ok(expr_out)
        }

        // {
        //     // 管道运算符特殊处理
        //     dbg!("--pipe--", &lhs, &rhs);
        //     // dbg!("--pipe--");
        //     let left_func = lhs.ensure_apply();
        //     let left_output = left_func.eval_mut(env, depth + 1)?;
        //     let mut new_env = env.fork();
        //     new_env.define("__stdin", left_output);

        //     let r_func = rhs.ensure_apply();
        //     let pipe_result = r_func.eval_mut(&mut new_env, depth + 1);
        //     // dbg!(&pipe_result);
        //     pipe_result
        // }
        "|>" => {
            // 执行左侧表达式
            let left_func = lhs.ensure_apply();
            let left_output = left_func.eval_mut(env, depth + 1)?;

            // 执行右侧表达式，获取函数或命令
            let rhs_eval = rhs.eval_mut(env, depth + 1)?;

            // 将左侧结果作为最后一个参数传递给右侧
            let args = vec![left_output];
            rhs_eval.append_args(args).eval_mut(env, depth + 1)
        }
        ">>" => {
            let left_func = lhs.ensure_apply();
            let l = left_func.eval_mut(env, depth + 1)?;

            let mut path = PathBuf::from(env.get_cwd());
            path = path.join(rhs.eval_mut(env, depth + 1)?.to_string());
            match std::fs::OpenOptions::new().append(true).open(&path) {
                Ok(mut file) => {
                    // use std::io::prelude::*;
                    let result = if let Expression::Bytes(bytes) = l.clone() {
                        // std::fs::write(path, bytes)
                        file.write_all(&bytes)
                    } else {
                        // Otherwise, convert the contents to a pretty string and write that.
                        // std::fs::write(path, contents.to_string())
                        file.write_all(l.clone().to_string().as_bytes())
                    };

                    match result {
                        Ok(()) => Ok(l),
                        Err(e) => {
                            return Err(LmError::CustomError(format!(
                                "could not append to file {}: {:?}",
                                rhs, e
                            )));
                        }
                    }
                }
                Err(e) => {
                    return Err(match e.kind() {
                        ErrorKind::PermissionDenied => LmError::PermissionDenied(*rhs),
                        _ => LmError::CustomError(format!(
                            "could not open file {}: {:?}",
                            path.display(),
                            e
                        )),
                    });
                }
            }
        }
        ">>!" => {
            // dbg!("-->>--", &lhs);
            let left_func = lhs.ensure_apply();
            let l = left_func.eval_mut(env, depth + 1)?;
            // dbg!("-->> left=", &l);
            let mut path = PathBuf::from(env.get_cwd());
            path = path.join(rhs.eval_mut(env, depth + 1)?.to_string());
            // If the contents are bytes, write the bytes directly to the file.
            let result = if let Expression::Bytes(bytes) = l.clone() {
                std::fs::write(path, bytes)
            } else {
                // Otherwise, convert the contents to a pretty string and write that.
                std::fs::write(path, l.to_string())
            };

            match result {
                Ok(()) => Ok(l),
                Err(e) => Err(LmError::CustomError(format!(
                    "could not write to file {}: {:?}",
                    rhs, e
                ))),
            }
        }
        "<<" => {
            // 输入重定向处理
            let path = rhs.eval_mut(env, depth + 1)?.to_string();
            let contents = std::fs::read_to_string(path)
                .map(Expression::String)
                .map_err(|e| LmError::CustomError(e.to_string()))?;
            let mut new_env = env.fork();
            new_env.define("__stdin", contents);
            let left_func = lhs.ensure_apply();
            let result = left_func.eval_mut(&mut new_env, depth + 1)?;
            return Ok(result);
        }
        _ => {
            let l = lhs.eval_mut(env, depth + 1)?;
            let r = rhs.eval_mut(env, depth + 1)?;
            return match operator.as_str() {
                "+" => Ok(l + r),
                "-" => Ok(l - r),
                "*" => Ok(l * r),
                "/" => {
                    if !r.is_truthy() {
                        return Err(LmError::CustomError(format!("can't divide {} by zero", l)));
                    };
                    Ok(l / r)
                } //no zero
                "%" => Ok(l % r),
                "**" => match (l, r) {
                    (Expression::Float(base), Expression::Float(exponent)) => {
                        Ok(base.powf(exponent).into())
                    }
                    (Expression::Float(base), Expression::Integer(exponent)) => {
                        Ok(base.powf(exponent as f64).into())
                    }
                    (Expression::Integer(base), Expression::Float(exponent)) => {
                        Ok((base as f64).powf(exponent).into())
                    }
                    (Expression::Integer(base), Expression::Integer(exponent)) => {
                        match base.checked_pow(exponent as u32) {
                            Some(n) => Ok(n.into()),
                            None => Err(LmError::CustomError(format!(
                                "overflow when raising int {} to the power {}",
                                base, exponent
                            ))),
                        }
                    }
                    (a, b) => Err(LmError::CustomError(format!(
                        "cannot raise {}:{} to the power {}:{}",
                        a,
                        a.type_name(),
                        b,
                        b.type_name()
                    ))),
                },

                "==" => Ok(Expression::Boolean(l == r)),
                "!=" => Ok(Expression::Boolean(l != r)),
                ">" => Ok(Expression::Boolean(l > r)),
                "<" => Ok(Expression::Boolean(l < r)),
                ">=" => Ok(Expression::Boolean(l >= r)),
                "<=" => Ok(Expression::Boolean(l <= r)),
                "~~" => Ok(Expression::Boolean(l.to_string().contains(&r.to_string()))),
                "~=" => {
                    let regex = Regex::new(&r.to_string())
                        .map_err(|e| LmError::CustomError(e.to_string()))?;

                    Ok(Expression::Boolean(regex.is_match(&l.to_string())))
                }
                "@" => index_slm(l, r),
                "." => match (l, r) {
                    (Expression::Map(m), n) => index_slm(Expression::Map(m), n),
                    (Expression::HMap(m), n) => index_slm(Expression::HMap(m), n),
                    (Expression::Symbol(m), Expression::Symbol(n)) => {
                        Ok(Expression::String(format!("{}.{}", m, n)))
                    }
                    // (Expression::String(m), Expression::String(n)) => Ok(Expression::String(m + &n)),
                    _ => Err(LmError::CustomError("not valid index option".into())),
                },
                ".." => match (l, r) {
                    (Expression::Integer(fr), Expression::Integer(t)) => {
                        let v = (fr..t)
                            .map(Expression::from) // 将 i64 转换为 Expression
                            .collect();
                        Ok(Expression::List(v))
                    }
                    _ => Err(LmError::CustomError("not valid range option".into())),
                },
                op if op.starts_with("_") => {
                    if let Some(oper) = env.get(op) {
                        let rs = Expression::Apply(Box::new(oper), vec![l, r]);
                        return rs.eval_mut(env, depth + 1);
                    }
                    Err(LmError::CustomError(format!(
                        "custom operation {op:?} not defined"
                    )))
                }
                // ----------
                _ => Err(LmError::InvalidOperator(operator.clone())),
            };
        }
    };
}

/// 索引
pub fn handle_index(
    lhs: Box<Expression>,
    rhs: Box<Expression>,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, LmError> {
    let l = lhs.eval_mut(env, depth + 1)?;
    let r = rhs.eval_mut(env, depth + 1)?;
    return index_slm(l, r);
}
/// 切片
pub fn handle_slice(
    list: &Box<Expression>,
    slice_params: SliceParams,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, LmError> {
    let listo = list.eval(env)?;
    let start_int = eval_to_int_opt(slice_params.start, env, depth)?;
    let end_int = eval_to_int_opt(slice_params.end, env, depth)?;
    let step_int = eval_to_int_opt(slice_params.step, env, depth)?.unwrap_or(1); // 默认步长1

    return slice(listo, start_int, end_int, step_int);
}

/// 索引访问
fn index_slm(l: Expression, r: Expression) -> Result<Expression, LmError> {
    match l {
        // 处理列表索引
        Expression::List(list) => {
            if let Expression::Integer(index) = r {
                list.get(index as usize)
                    .cloned()
                    .ok_or_else(|| LmError::IndexOutOfBounds {
                        index: index as usize,
                        len: list.len(),
                    })
            } else {
                Err(LmError::TypeError {
                    expected: "integer".into(),
                    found: r.type_name(),
                })
            }
        }

        // 处理字典键访问
        Expression::Map(map) => {
            let key = r.to_string(); // 自动转换Symbol/字符串
            map.get(&key)
                .cloned()
                .ok_or_else(|| LmError::KeyNotFound(key))
        }
        Expression::HMap(map) => {
            let key = r.to_string(); // 自动转换Symbol/字符串
            map.get(&key)
                .cloned()
                .ok_or_else(|| LmError::KeyNotFound(key))
        }

        // 处理字符串索引
        Expression::String(s) => {
            if let Expression::Integer(index) = r {
                s.chars()
                    .nth(index as usize)
                    .map(|c| Expression::String(c.to_string()))
                    .ok_or_else(|| LmError::IndexOutOfBounds {
                        index: index as usize,
                        len: s.len(),
                    })
            } else {
                Err(LmError::TypeError {
                    expected: "integer".into(),
                    found: r.type_name(),
                })
            }
        }

        _ => Err(LmError::TypeError {
            expected: "indexable type (list/dict/string)".into(),
            found: l.type_name(),
        }),
    }
}

/// 列表切片，处理负数索引和越界...

pub fn slice(
    list: Expression,
    start: Option<Int>,
    end: Option<Int>,
    step: Int,
) -> Result<Expression, LmError> {
    let list = list.as_list()?;
    let len = list.len() as Int;

    let clamp = |v: Int| if v < 0 { len + v } else { v }.clamp(0, len - 1);

    let (mut start, mut end) = (
        start.map(clamp).unwrap_or(0),
        end.map(|v| clamp(v).min(len)).unwrap_or(len),
    );

    if step < 0 {
        (start, end) = (end.clamp(0, len), start.clamp(0, len));
    }

    let mut result = Vec::new();
    let mut i = start;
    while (step > 0 && i < end) || (step < 0 && i > end) {
        if let Some(item) = list.get(i as usize) {
            result.push(item.clone());
        }
        i += step;
    }
    Ok(Expression::List(result))
}

/// 辅助方法：将表达式求值为整数选项
fn eval_to_int_opt(
    expr_opt: Option<Box<Expression>>,
    env: &mut Environment,
    depth: usize,
) -> Result<Option<Int>, LmError> {
    match expr_opt {
        // 无表达式时返回 None
        None => Ok(None),
        // 有表达式时进行求值
        Some(boxed_expr) => {
            // 递归求值表达式
            let evaluated = boxed_expr.eval_mut(env, depth)?;

            // 转换为整数
            match evaluated {
                Expression::Integer(i) => Ok(Some(i)),
                // 处理隐式类型转换
                Expression::Float(f) if f.fract() == 0.0 => Ok(Some(f as Int)),
                // 处理其他类型错误
                _ => Err(LmError::TypeError {
                    expected: "integer".into(),
                    found: evaluated.type_name(),
                }),
            }
        }
    }
}
