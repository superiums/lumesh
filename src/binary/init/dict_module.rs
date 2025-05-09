use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) fn flatten(expr: Expression) -> Vec<Expression> {
    match expr {
        Expression::List(list) => list
            .as_ref()
            .iter()
            .flat_map(|item| flatten(item.clone()))
            .collect(),
        Expression::Map(map) => map
            .as_ref()
            .values()
            .flat_map(|item| flatten(item.clone()))
            .collect(),
        expr => vec![expr],
    }
}

pub fn get() -> Expression {
    (hash_map! {
        String::from("flatten") => Expression::builtin("flatten", |args, env| {
            super::check_exact_args_len("flatten", &args, 1)?;
            Ok(Expression::List(Rc::new(flatten(args[0].eval(env)?))))
        }, "flatten a list"),

        String::from("items") => Expression::builtin("items", |args, env| {
            super::check_exact_args_len("items", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => Expression::List(Rc::new(
                    map.as_ref()
                        .iter()
                        .map(|(k, v)| Expression::List(Rc::new(vec![
                            Expression::String(k.clone()),
                            v.clone()
                        ])))
                        .collect()
                )),
                Expression::List(list) => Expression::List(Rc::new(
                    list.as_ref()
                        .iter()
                        .enumerate()
                        .map(|(i, v)| Expression::List(Rc::new(vec![
                            Expression::Integer(i as Int),
                            v.clone()
                        ])))
                        .collect()
                )),
                _ => Expression::None
            })
        }, "get the items of a map or list"),

        String::from("keys") => Expression::builtin("keys", |args, env| {
            super::check_exact_args_len("keys", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => Expression::List(Rc::new(
                    map.as_ref()
                        .keys()
                        .map(|k| Expression::String(k.clone()))
                        .collect()
                )),
                _ => Expression::None
            })
        }, "get the keys of a map"),

        String::from("values") => Expression::builtin("values", |args, env| {
            super::check_exact_args_len("values", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => Expression::List(Rc::new(
                    map.as_ref()
                        .values()
                        .cloned()
                        .collect()
                )),
                _ => Expression::None
            })
        }, "get the values of a map"),

        String::from("insert") => Expression::builtin("insert", |args, env| {
            super::check_exact_args_len("insert", &args, 3)?;
            let expr = args[0].eval(env)?;
            let key = args[1].eval(env)?;
            let value = args[2].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => {
                    let mut new_map = HashMap::with_capacity(map.as_ref().len() + 1);
                    new_map.extend(map.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
                    new_map.insert(key.to_string(), value);
                    Expression::Map(Rc::new(new_map))
                },
                _ => Expression::None
            })
        }, "insert a key-value pair into a map"),

        String::from("remove") => Expression::builtin("remove", |args, env| {
            super::check_exact_args_len("remove", &args, 2)?;
            let expr = args[0].eval(env)?;
            let key = args[1].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => {
                    let mut new_map = HashMap::with_capacity(map.as_ref().len());
                    new_map.extend(map.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
                    new_map.remove(&key.to_string());
                    Expression::Map(Rc::new(new_map))
                },
                _ => Expression::None
            })
        }, "remove a key-value pair from a map"),

        String::from("has") => Expression::builtin("has", |args, env| {
            super::check_exact_args_len("has", &args, 2)?;
            let expr = args[0].eval(env)?;
            let key = args[1].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => Expression::Boolean(
                    map.as_ref().contains_key(&key.to_string())
                ),
                _ => Expression::None
            })
        }, "check if a map has a key"),

        String::from("len") => Expression::builtin("len", |args, env| {
            super::check_exact_args_len("len", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::Map(map) => Expression::Integer(map.as_ref().len() as Int),
                Expression::List(list) => Expression::Integer(list.as_ref().len() as Int),
                Expression::String(s) => Expression::Integer(s.len() as Int),
                Expression::Bytes(b) => Expression::Integer(b.len() as Int),
                _ => Expression::None
            })
        }, "get the length of a map, list, string, or bytes"),

        String::from("from_items") => Expression::builtin("from_items", |args, env| {
            super::check_exact_args_len("from_items", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match expr {
                Expression::List(list) => {
                    let mut map = HashMap::new();
                    for item in list.as_ref() {
                        if let Expression::List(pair) = item {
                            if pair.as_ref().len() == 2 {
                                map.insert(pair.as_ref()[0].to_string(), pair.as_ref()[1].clone());
                            }
                        }
                    }
                    Expression::Map(Rc::new(map))
                },
                _ => Expression::None
            })
        }, "create a map from a list of key-value pairs"),

        String::from("union") => Expression::builtin("union", |args, env| {
            super::check_exact_args_len("union", &args, 2)?;
            let expr1 = args[0].eval(env)?;
            let expr2 = args[1].eval(env)?;
            Ok(match (expr1, expr2) {
                (Expression::Map(map1), Expression::Map(map2)) => {
                    let mut new_map = HashMap::with_capacity(map1.as_ref().len() + map2.as_ref().len());
                    new_map.extend(map1.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
                    new_map.extend(map2.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
                    Expression::Map(Rc::new(new_map))
                },
                _ => Expression::None
            })
        }, "combine two maps"),

        String::from("intersect") => Expression::builtin("intersect", |args, env| {
            super::check_exact_args_len("intersect", &args, 2)?;
            let expr1 = args[0].eval(env)?;
            let expr2 = args[1].eval(env)?;
            Ok(match (expr1, expr2) {
                (Expression::Map(map1), Expression::Map(map2)) => {
                    let mut new_map = HashMap::new();
                    for (k, v) in map1.as_ref() {
                        if map2.as_ref().contains_key(k) {
                            new_map.insert(k.clone(), v.clone());
                        }
                    }
                    Expression::Map(Rc::new(new_map))
                },
                _ => Expression::None
            })
        }, "get the intersection of two maps"),

        String::from("difference") => Expression::builtin("difference", |args, env| {
            super::check_exact_args_len("difference", &args, 2)?;
            let expr1 = args[0].eval(env)?;
            let expr2 = args[1].eval(env)?;
            Ok(match (expr1, expr2) {
                (Expression::Map(map1), Expression::Map(map2)) => {
                    let mut new_map = HashMap::new();
                    for (k, v) in map1.as_ref() {
                        if !map2.as_ref().contains_key(k) {
                            new_map.insert(k.clone(), v.clone());
                        }
                    }
                    Expression::Map(Rc::new(new_map))
                },
                _ => Expression::None
            })
        }, "get the difference of two maps"),

        String::from("map_map") => Expression::builtin("map_map", |args, env| {
            super::check_args_len("map_map", &args, 2..3)?;

            let map = match args.last().unwrap().eval(env)? {
                Expression::Map(m) => m,
                _ => return Err(LmError::CustomError("map_map requires a map as last argument".to_string())),
            };

            let key_func = args[0].eval(env)?;
            let val_func = if args.len() == 3 {
                args[1].eval(env)?
            } else {
                Expression::builtin("_id", |args, _| Ok(args.last().unwrap().clone()), "identity function")
            };

            let mut new_map = HashMap::new();

            for (k, v) in map.as_ref().iter() {
                let new_key = match Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![Expression::String(k.clone())])).eval(env)? {
                    Expression::String(s) => s,
                    other => other.to_string(),
                };

                let new_val = Expression::Apply(Rc::new(val_func.clone()), Rc::new(vec![v.clone()])).eval(env)?;

                new_map.insert(new_key, new_val);
            }

            Ok(Expression::Map(Rc::new(new_map)))
        }, "transform map keys and values with provided functions"),

        String::from("deep_merge") => Expression::builtin("deep_merge", |args, env| {
            // 至少需要两个map来合并
            if args.len() < 2 {
                return Err(LmError::CustomError("deep_merge requires at least two maps".to_string()));
            }

            let mut base = match args[0].eval(env)? {
                Expression::Map(m) => m.as_ref().clone(),
                _ => return Err(LmError::CustomError("deep_merge requires maps as arguments".to_string())),
            };

            for arg in args.iter().skip(1) {
                let next = match arg.eval(env)? {
                    Expression::Map(m) => m,
                    _ => return Err(LmError::CustomError("deep_merge requires maps as arguments".to_string())),
                };
                base = deep_merge_maps(&base, next.as_ref(), env)?;
            }

            Ok(Expression::Map(Rc::new(base)))
        }, "recursively merge two or more maps"),

        String::from("get_path") => Expression::builtin("get_path", |args, env| {
            super::check_exact_args_len("get_path", &args, 2)?;

            let map = match args[1].eval(env)? {
                Expression::Map(m) => m,
                _ => return Err(LmError::CustomError("get_path requires a map as last argument".to_string())),
            };

            let path = match args[0].eval(env)? {
                Expression::String(s) => s,
                _ => return Err(LmError::CustomError("get_path requires a path string as second argument".to_string())),
            };

            let path_segments: Vec<&str> = path.split('.').collect();
            if path_segments.is_empty() {
                return Ok(Expression::Map(map));
            }

            get_value_by_path(map.as_ref(), &path_segments)
        }, "get value from nested map using dot notation path (e.g., 'a.b.c')"),

    })
    .into()
}

fn deep_merge_maps(
    a: &HashMap<String, Expression>,
    b: &HashMap<String, Expression>,
    env: &mut Environment,
) -> Result<HashMap<String, Expression>, LmError> {
    let mut result = HashMap::new();

    // 先插入a的所有元素
    for (k, v) in a.iter() {
        result.insert(k.clone(), v.clone());
    }

    // 合并b的元素
    for (k, v) in b.iter() {
        if let Some(existing) = result.get(k) {
            // 如果两个值都是map，则递归合并
            if let (Expression::Map(ma), Expression::Map(mb)) = (existing, v) {
                result.insert(
                    k.clone(),
                    Expression::Map(Rc::new(deep_merge_maps(ma.as_ref(), mb.as_ref(), env)?)),
                );
            } else {
                // 否则使用b的值覆盖
                result.insert(k.clone(), v.clone());
            }
        } else {
            result.insert(k.clone(), v.clone());
        }
    }

    Ok(result)
}

fn get_value_by_path(
    map: &HashMap<String, Expression>,
    path: &[&str],
    // env: &mut Environment,
) -> Result<Expression, LmError> {
    let mut current = Expression::Map(Rc::new(map.clone()));

    for segment in path {
        match current {
            Expression::Map(m) => {
                current = m
                    .as_ref()
                    .get(*segment)
                    .ok_or_else(|| {
                        LmError::CustomError(format!("path segment '{}' not found", segment))
                    })?
                    .clone();
            }
            _ => {
                return Err(LmError::CustomError(
                    "path segment access on non-map type".to_string(),
                ));
            }
        }
    }

    Ok(current)
}
