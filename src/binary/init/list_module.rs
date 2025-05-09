use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;

use std::{collections::HashMap, rc::Rc};

pub fn get() -> Expression {
    (hash_map! {
        String::from("list") => Expression::builtin("list", list, "create a list from a variable number of arguments"),
        String::from("last") => Expression::builtin("last", last, "get the last of a list"),
        String::from("first") => Expression::builtin("first", first, "get the first of a list"),
        String::from("chunk") => Expression::builtin("chunk", chunk, "chunk a list into lists of n elements"),
        String::from("cons") => Expression::builtin("cons", cons, "prepend an element to a list"),
        String::from("append") => Expression::builtin("append", append, "append an element to a list"),
        String::from("rev") => Expression::builtin("rev", rev, "reverse a list"),
        String::from("range") => Expression::builtin("range", range, "create a list of integers from a to b"),
        String::from("foldl") => Expression::builtin("foldl", foldl, "fold a list from the left"),
        String::from("foldr") => Expression::builtin("foldr", foldr, "fold a list from the right"),
        String::from("zip") => Expression::builtin("zip", zip, "zip two lists together"),
        String::from("unzip") => Expression::builtin("unzip", unzip, "unzip a list of pairs into a pair of lists"),
        String::from("take") => Expression::builtin("take", take, "take the first n elements of a list"),
        String::from("drop") => Expression::builtin("drop", drop, "drop the first n elements of a list"),
        String::from("split_at") => Expression::builtin("split_at", split_at, "split a list at a given index"),
        String::from("nth") => Expression::builtin("nth", nth, "get the nth element of a list"),
        String::from("map") => Expression::builtin("map", map, "map a function over a list of values"),
        String::from("filter") => Expression::builtin("filter", filter, "filter a list of values with a condition function"),
        String::from("reduce") => Expression::builtin("reduce", reduce, "reduce a function over a list of values"),

        String::from("find") => Expression::builtin("find", |args, env| {
            super::check_exact_args_len("find", &args, 2)?;

            let list = match args[1].eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("find requires a list as last argument".to_string())),
            };

            let target = args[0].eval(env)?;

            Ok(match list.as_ref().iter().position(|x| *x == target) {
                Some(index) => Expression::Integer(index as Int),
                None => Expression::None,
            })
        }, "find the index of an element in a list, returns None if not found"),

        String::from("group_by") => Expression::builtin("group_by", |args, env| {
            super::check_exact_args_len("group_by", &args, 2)?;

            let list = match args[1].eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("group_by requires a list as last argument".to_string())),
            };

            let key_func = args[0].eval(env)?;

            let mut groups: HashMap<String, Vec<Expression>> = HashMap::new();

            for item in list.as_ref().iter() {
                let key = match Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
                    Expression::String(s) => s,
                    other => other.to_string(),
                };

                groups.entry(key).or_default().push(item.clone());
            }

            let result = groups.into_iter()
                .map(|(k, v)| Expression::List(Rc::new(vec![
                    Expression::String(k),
                    Expression::List(Rc::new(v))
                ])))
                .collect();

            Ok(Expression::List(Rc::new(result)))
        }, "group list elements by key function, returns list of [key, elements] pairs"),

        String::from("filter_map") => Expression::builtin("filter_map", |args, env| {
            super::check_exact_args_len("filter_map", &args, 2)?;

            let list = match args[1].eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("filter_map requires a list as last argument".to_string())),
            };

            let func = args[0].eval(env)?;
            let mut result = Vec::new();

            for item in list.as_ref().iter() {
                match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
                    Expression::None => continue,
                    val => result.push(val),
                }
            }

            Ok(Expression::List(Rc::new(result)))
        }, "filter and map list elements in one pass, skipping None values"),

        String::from("sort") => Expression::builtin("sort", |args, env| {
            super::check_args_len("sort", &args, 1..2)?;

            let list = match args.last().unwrap().eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("sort requires a list as last argument".to_string())),
            };

            let mut sorted = list.as_ref().clone();

            // 如果有key函数，则使用它来提取排序键
            if args.len() == 2 {
                let key_func = args[0].eval(env)?;
                sorted.sort_by(|a, b| {
                    let key_a = Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![a.clone()]))
                        .eval(env).unwrap_or(Expression::None);
                    let key_b = Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![b.clone()]))
                        .eval(env).unwrap_or(Expression::None);
                    key_a.partial_cmp(&key_b).unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            }

            Ok(Expression::List(Rc::new(sorted)))
        }, "sort a list, optionally with a key function"),

        String::from("unique") => Expression::builtin("unique", |args, env| {
            super::check_exact_args_len("unique", &args, 1)?;

            let list = match args[0].eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("unique requires a list as argument".to_string())),
            };

            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();

            for item in list.as_ref().iter() {
                if seen.insert(item.to_string()) {
                    result.push(item.clone());
                }
            }

            Ok(Expression::List(Rc::new(result)))
        }, "remove duplicates from a list while preserving order"),

        String::from("list_to_map") => Expression::builtin("list_to_map", |args, env| {
            super::check_args_len("list_to_map", &args, 1..2)?;

            let list = match args.last().unwrap().eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("list_to_map requires a list as last argument".to_string())),
            };

            let key_func = if args.len() == 2 {
                args[0].eval(env)?
            } else {
                Expression::builtin("_id", |args, _| Ok(args[0].clone()), "identity function")
            };

            let mut map = HashMap::new();

            for item in list.as_ref().iter() {
                let key = match Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
                    Expression::String(s) => s,
                    other => other.to_string(),
                };

                map.insert(key, item.clone());
            }

            Ok(Expression::Map(Rc::new(map)))
        }, "convert list to map using a key function (default: use items themselves as keys)"),

        String::from("transpose") => Expression::builtin("transpose", |args, env| {
            super::check_exact_args_len("transpose", &args, 1)?;

            let matrix = match args[0].eval(env)? {
                Expression::List(l) => l,
                _ => return Err(LmError::CustomError("transpose requires a list of lists as argument".to_string())),
            };

            if matrix.as_ref().is_empty() {
                return Ok(Expression::List(Rc::new(vec![])));
            }

            // 验证所有行长度相同
            let row_len = match matrix.as_ref().first() {
                Some(Expression::List(row)) => row.as_ref().len(),
                _ => return Err(LmError::CustomError("transpose requires a list of lists as argument".to_string())),
            };

            for row in matrix.as_ref().iter() {
                if let Expression::List(r) = row {
                    if r.as_ref().len() != row_len {
                        return Err(LmError::CustomError("all rows must have same length for transpose".to_string()));
                    }
                } else {
                    return Err(LmError::CustomError("transpose requires a list of lists as argument".to_string()));
                }
            }

            // 执行转置
            let mut transposed = Vec::new();
            for i in 0..row_len {
                let mut new_row = Vec::new();
                for row in matrix.as_ref().iter() {
                    if let Expression::List(r) = row {
                        new_row.push(r.as_ref()[i].clone());
                    }
                }
                transposed.push(Expression::List(Rc::new(new_row)));
            }

            Ok(Expression::List(Rc::new(transposed)))
        }, "transpose a matrix (list of lists) by switching rows and columns"),

        // String::from("par_map") => Expression::builtin("par_map", |args, env| {
        //     use rayon::prelude::*;

        //     super::check_exact_args_len("par_map", &args, 2)?;

        //     let list = match args[1].eval(env)? {
        //         Expression::List(l) => l,
        //         _ => return Err(LmError::CustomError("par_map requires a list as last argument".to_string())),
        //     };

        //     let func = args[0].eval(env)?;
        //     let env = env.clone(); // 需要克隆环境用于并行

        //     let result: Vec<Expression> = list.as_ref()
        //         .par_iter()
        //         .map(|item| {
        //             Expression::Apply(
        //                 Rc::new(func.clone()),
        //                 Rc::new(vec![item.clone()])
        //             ).eval(&mut env.clone()).unwrap_or_else(|_| Expression::None)
        //         })
        //         .collect();

        //     Ok(Expression::List(Rc::new(result)))
        // }, "parallel map operation on a list"),
    })
    .into()
}

fn list(args: Vec<Expression>, _env: &mut Environment) -> Result<Expression, LmError> {
    Ok(Expression::List(Rc::new(args)))
}
fn last(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 1 {
        return Err(LmError::CustomError(
            "last requires exactly one argument".to_string(),
        ));
    }
    let list = args[0].eval(env)?;
    if let Expression::List(list) = list {
        let list_ref = list.as_ref();
        Ok(list_ref
            .last()
            .cloned()
            .ok_or_else(|| LmError::CustomError("cannot get last of empty list".to_string()))?)
    } else {
        Err(LmError::CustomError(
            "last requires a list as its argument".to_string(),
        ))
    }
}

fn first(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 1 {
        return Err(LmError::CustomError(
            "first requires exactly one argument".to_string(),
        ));
    }
    let list = args[0].eval(env)?;
    if let Expression::List(list) = list {
        let list_ref = list.as_ref();
        Ok(list_ref
            .first()
            .cloned()
            .ok_or_else(|| LmError::CustomError("cannot get first of empty list".to_string()))?)
    } else {
        Err(LmError::CustomError(
            "first requires a list as its argument".to_string(),
        ))
    }
}

// 列表操作函数实现
fn chunk(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "chunk requires exactly two arguments".to_string(),
        ));
    }

    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "chunk requires an integer as its first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "chunk requires a list as its second argument".to_string(),
            ));
        }
    };

    let mut result = Vec::new();
    let mut chunk = Vec::new();

    for item in list.as_ref().iter() {
        chunk.push(item.clone());
        if chunk.len() == n as usize {
            result.push(Expression::List(Rc::new(chunk)));
            chunk = Vec::new();
        }
    }

    if !chunk.is_empty() {
        result.push(Expression::List(Rc::new(chunk)));
    }

    Ok(Expression::List(Rc::new(result)))
}

fn cons(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "cons requires exactly two arguments".to_string(),
        ));
    }

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "cons requires a list as its second argument".to_string(),
            ));
        }
    };

    let head = args[0].eval(env)?;
    let mut new_list = Vec::with_capacity(list.as_ref().len() + 1);
    new_list.push(head);
    new_list.extend(list.as_ref().iter().cloned());

    Ok(Expression::List(Rc::new(new_list)))
}

fn append(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "append requires exactly two arguments".to_string(),
        ));
    }

    let list = match args[0].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "append requires a list as its first argument".to_string(),
            ));
        }
    };

    let item = args[1].eval(env)?;
    let mut new_list: Vec<Expression> = list.as_ref().to_vec();
    new_list.push(item);

    Ok(Expression::List(Rc::new(new_list)))
}

pub fn rev(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 1 {
        return Err(LmError::CustomError(
            "rev requires exactly one argument".to_string(),
        ));
    }

    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut reversed: Vec<Expression> = list.as_ref().to_vec();
            reversed.reverse();
            Ok(Expression::List(Rc::new(reversed)))
        }
        Expression::String(s) => Ok(Expression::String(s.chars().rev().collect())),
        Expression::Symbol(s) => Ok(Expression::Symbol(s.chars().rev().collect())),
        Expression::Bytes(b) => Ok(Expression::Bytes(b.into_iter().rev().collect())),
        _ => Err(LmError::CustomError(
            "rev requires a list or string as its argument".to_string(),
        )),
    }
}

fn range(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "range requires exactly two arguments".to_string(),
        ));
    }

    match (args[0].eval(env)?, args[1].eval(env)?) {
        (Expression::Integer(a), Expression::Integer(b)) => Ok(Expression::List(Rc::new(
            (a..=b).map(Expression::from).collect(),
        ))),
        _ => Err(LmError::CustomError(
            "range requires two integers as its arguments".to_string(),
        )),
    }
}

fn foldl(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 3 {
        return Err(LmError::CustomError(
            "foldl requires exactly three arguments".to_string(),
        ));
    }

    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = match args[2].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "foldl requires a list as its third argument".to_string(),
            ));
        }
    };

    for item in list.as_ref().iter() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()])).eval(env)?;
    }

    Ok(acc)
}

fn foldr(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 3 {
        return Err(LmError::CustomError(
            "foldr requires exactly three arguments".to_string(),
        ));
    }

    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = match args[2].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "foldr requires a list as its third argument".to_string(),
            ));
        }
    };

    for item in list.as_ref().iter().rev() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone(), acc])).eval(env)?;
    }

    Ok(acc)
}

fn zip(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "zip requires exactly two arguments".to_string(),
        ));
    }

    match (args[0].eval(env)?, args[1].eval(env)?) {
        (Expression::List(list1), Expression::List(list2)) => {
            let mut result = Vec::new();

            for (item1, item2) in list1.as_ref().iter().zip(list2.as_ref().iter()) {
                result.push(Expression::List(Rc::new(vec![
                    item1.clone(),
                    item2.clone(),
                ])));
            }

            Ok(Expression::List(Rc::new(result)))
        }
        _ => Err(LmError::CustomError(
            "zip requires two lists as its arguments".to_string(),
        )),
    }
}

fn unzip(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 1 {
        return Err(LmError::CustomError(
            "unzip requires exactly one argument".to_string(),
        ));
    }

    let list = match args[0].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "unzip requires a list as its argument".to_string(),
            ));
        }
    };

    let mut list1 = Vec::new();
    let mut list2 = Vec::new();

    for item in list.as_ref().iter() {
        if let Expression::List(pair) = item {
            if pair.as_ref().len() != 2 {
                return Err(LmError::CustomError(
                    "unzip requires a list of pairs as its argument".to_string(),
                ));
            }
            list1.push(pair.as_ref()[0].clone());
            list2.push(pair.as_ref()[1].clone());
        } else {
            return Err(LmError::CustomError(
                "unzip requires a list of pairs as its argument".to_string(),
            ));
        }
    }

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(list1)),
        Expression::List(Rc::new(list2)),
    ])))
}

fn take(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "take requires exactly two arguments".to_string(),
        ));
    }

    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "take requires an integer as its first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "take requires a list as its second argument".to_string(),
            ));
        }
    };

    let taken: Vec<Expression> = list.as_ref().iter().take(n as usize).cloned().collect();

    Ok(Expression::List(Rc::new(taken)))
}

fn drop(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "drop requires exactly two arguments".to_string(),
        ));
    }

    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "drop requires an integer as its first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "drop requires a list as its second argument".to_string(),
            ));
        }
    };

    let dropped: Vec<Expression> = list.as_ref().iter().skip(n as usize).cloned().collect();

    Ok(Expression::List(Rc::new(dropped)))
}

fn split_at(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "split_at requires exactly two arguments".to_string(),
        ));
    }

    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "split_at requires an integer as its first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "split_at requires a list as its second argument".to_string(),
            ));
        }
    };

    let taken: Vec<Expression> = list.as_ref().iter().take(n as usize).cloned().collect();

    let dropped: Vec<Expression> = list.as_ref().iter().skip(n as usize).cloned().collect();

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(taken)),
        Expression::List(Rc::new(dropped)),
    ])))
}

fn nth(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "nth requires exactly two arguments".to_string(),
        ));
    }

    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "nth requires an integer as its first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(list) => list,
        _ => {
            return Err(LmError::CustomError(
                "nth requires a list as its second argument".to_string(),
            ));
        }
    };

    let idx = if n < 0 {
        list.as_ref().len().checked_sub((-n) as usize)
    } else {
        Some(n as usize)
    };

    match idx {
        Some(idx) => list
            .as_ref()
            .get(idx)
            .cloned()
            .ok_or_else(|| LmError::CustomError("index out of bounds".to_string())),
        None => Err(LmError::CustomError("index out of bounds".to_string())),
    }
}

fn map(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=2).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 2 {
                "too many arguments to function map"
            } else {
                "too few arguments to function map"
            }
            .to_string(),
        ));
    }

    if args.len() == 1 {
        Ok(Expression::Apply(
            Rc::new(crate::parse("(f,list) -> for item in list {f item}")?),
            Rc::new(args.clone()),
        )
        .eval(env)?)
    } else {
        let f = args[0].eval(env)?;
        let list = match args[1].eval(env)? {
            Expression::List(list) => list,
            _ => {
                return Err(LmError::CustomError(
                    "map requires a list as its second argument".to_string(),
                ));
            }
        };

        let mut result = Vec::with_capacity(list.as_ref().len());

        for item in list.as_ref().iter() {
            result.push(
                Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone()])).eval(env)?,
            );
        }

        Ok(Expression::List(Rc::new(result)))
    }
}

fn filter(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=2).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 2 {
                "too many arguments to function filter"
            } else {
                "too few arguments to function filter"
            }
            .to_string(),
        ));
    }

    if args.len() == 1 {
        Ok(Expression::Apply(
            Rc::new(crate::parse(
                r#"(f,list) -> {
                    let result = [];
                    for item in list {
                        if (f item) {
                            let result = result + [item];
                        }
                    }
                    result
                }"#,
            )?),
            Rc::new(args.clone()),
        )
        .eval(env)?)
    } else {
        let f = args[0].eval(env)?;
        let list = match args[1].eval(env)? {
            Expression::List(list) => list,
            _ => {
                return Err(LmError::CustomError(
                    "filter requires a list as its second argument".to_string(),
                ));
            }
        };

        let mut result = Vec::new();

        for item in list.as_ref().iter() {
            if Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone()]))
                .eval(env)?
                .is_truthy()
            {
                result.push(item.clone());
            }
        }

        Ok(Expression::List(Rc::new(result)))
    }
}

fn reduce(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=3).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 3 {
                "too many arguments to function reduce"
            } else {
                "too few arguments to function reduce"
            }
            .to_string(),
        ));
    }

    if args.len() < 3 {
        Ok(Expression::Apply(
            Rc::new(crate::parse(
                "(f,acc,list) -> {
                    for item in list {
                        let acc = f acc item
                    }
                    acc
                }",
            )?),
            Rc::new(args.clone()),
        )
        .eval(env)?)
    } else {
        let f = args[0].eval(env)?;
        let mut acc = args[1].eval(env)?;
        let list = match args[2].eval(env)? {
            Expression::List(list) => list,
            _ => {
                return Err(LmError::CustomError(
                    "reduce requires a list as its third argument".to_string(),
                ));
            }
        };

        for item in list.as_ref().iter() {
            acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()]))
                .eval(env)?;
        }

        Ok(acc)
    }
}
