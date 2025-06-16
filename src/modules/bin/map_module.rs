use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

use std::collections::BTreeMap;
use std::rc::Rc;

pub fn get() -> Expression {
    (hash_map! {
        // 检查操作
               String::from("has") => Expression::builtin("has", has, "check if a map has a key", "<key> <map>"),

               // 数据获取
               String::from("items") => Expression::builtin("items", items, "get the items of a map or list", "<map>"),
               String::from("keys") => Expression::builtin("keys", keys, "get the keys of a map", "<map>"),
               String::from("values") => Expression::builtin("values", values, "get the values of a map", "<map>"),

               // 结构修改
               String::from("insert") => Expression::builtin("insert", insert, "insert a key-value pair into a map", "<key> <value> <map>"),
               String::from("remove") => Expression::builtin("remove", remove, "remove a key-value pair from a map", "<key> <map>"),

               // 创建操作
               String::from("from_items") => Expression::builtin("from_items", from_items, "create a map from a list of key-value pairs", "<items>"),

               // 集合运算
               String::from("union") => Expression::builtin("union", union, "combine two maps", "<map1> <map2>"),
               String::from("intersect") => Expression::builtin("intersect", intersect, "get the intersection of two maps", "<map1> <map2>"),
               String::from("difference") => Expression::builtin("difference", difference, "get the difference of two maps", "<map1> <map2>"),
               String::from("deep_merge") => Expression::builtin("deep_merge", deep_merge, "recursively merge two or more maps", "<map1> <map2> [<map3> ...]"),

               // 转换操作
               String::from("map") => Expression::builtin("map", map_map, "transform map keys and values with provided functions", "<key_fn> <val_fn> <map>"),
    })
    .into()
}

// Helper function implementations

fn items(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("items", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => {
            let items = map
                .iter()
                .map(|(k, v)| Expression::from(vec![Expression::String(k.clone()), v.clone()]))
                .collect();
            Expression::List(Rc::new(items))
        }
        _ => Expression::None,
    })
}

fn keys(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("keys", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => {
            let keys = map
                .as_ref()
                .keys()
                .map(|k| Expression::String(k.clone()))
                .collect();
            Expression::List(Rc::new(keys))
        }
        _ => Expression::None,
    })
}

fn values(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("values", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => Expression::List(Rc::new(map.as_ref().values().cloned().collect())),
        _ => Expression::None,
    })
}

fn insert(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("insert", args, 3)?;
    let expr = args[2].eval(env)?;
    let key = args[0].eval(env)?;
    let value = args[1].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => {
            let mut new_map = map.as_ref().clone();
            new_map.insert(key.to_string(), value);
            Expression::Map(Rc::new(new_map))
        }
        _ => Expression::None,
    })
}

fn remove(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("remove", args, 2)?;
    let expr = args[1].eval(env)?;
    let key = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => {
            let mut new_map = map.as_ref().clone();
            new_map.remove(&key.to_string());
            Expression::Map(Rc::new(new_map))
        }
        _ => Expression::None,
    })
}

fn has(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("has", args, 2)?;
    let expr = args[1].eval(env)?;
    let key = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => Expression::Boolean(map.as_ref().contains_key(&key.to_string())),
        _ => Expression::None,
    })
}

fn from_items(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("from_items", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::List(list) => {
            let mut map = BTreeMap::new();
            for item in list.as_ref() {
                if let Expression::List(pair) = item {
                    if pair.as_ref().len() == 2 {
                        map.insert(pair.as_ref()[0].to_string(), pair.as_ref()[1].clone());
                    }
                }
            }
            Expression::from(map)
        }
        _ => Expression::None,
    })
}

fn union(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("union", args, 2)?;
    let expr1 = args[0].eval(env)?;
    let expr2 = args[1].eval(env)?;

    Ok(match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = map1.as_ref().clone();
            new_map.extend(map2.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
            Expression::Map(Rc::new(new_map))
        }
        _ => Expression::None,
    })
}

fn intersect(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("intersect", args, 2)?;
    let expr1 = args[0].eval(env)?;
    let expr2 = args[1].eval(env)?;

    Ok(match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = BTreeMap::new();
            for (k, v) in map1.as_ref() {
                if map2.as_ref().contains_key(k) {
                    new_map.insert(k.clone(), v.clone());
                }
            }
            Expression::from(new_map)
        }
        _ => Expression::None,
    })
}

fn difference(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("difference", args, 2)?;
    let expr1 = args[0].eval(env)?;
    let expr2 = args[1].eval(env)?;

    Ok(match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = BTreeMap::new();
            for (k, v) in map1.as_ref() {
                if !map2.as_ref().contains_key(k) {
                    new_map.insert(k.clone(), v.clone());
                }
            }
            Expression::from(new_map)
        }
        _ => Expression::None,
    })
}

fn map_map(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("map.map", args, 2..=3)?;
    let map = match args.last().unwrap().eval(env)? {
        Expression::Map(m) => m,
        _ => {
            return Err(LmError::CustomError(
                "map.map requires a map as last argument".to_string(),
            ));
        }
    };

    let key_func = args[0].eval(env)?;
    let val_func = if args.len() == 3 {
        args[1].eval(env)?
    } else {
        Expression::builtin(
            "_id",
            |args, _| Ok(args.last().unwrap().clone()),
            "identity function",
            "",
        )
    };

    let mut new_map = BTreeMap::new();

    for (k, v) in map.iter() {
        let new_key = match Expression::Apply(
            Rc::new(key_func.clone()),
            Rc::new(vec![Expression::String(k.clone())]),
        )
        .eval(env)?
        {
            Expression::String(s) => s,
            other => other.to_string(),
        };

        let new_val =
            Expression::Apply(Rc::new(val_func.clone()), Rc::new(vec![v.clone()])).eval(env)?;
        new_map.insert(new_key, new_val);
    }

    Ok(Expression::from(new_map))
}

fn deep_merge(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() < 2 {
        return Err(LmError::CustomError(
            "deep_merge requires at least two maps".to_string(),
        ));
    }

    let mut base = match args[0].eval(env)? {
        Expression::Map(m) => m.as_ref().clone(),
        _ => {
            return Err(LmError::CustomError(
                "deep_merge requires maps as arguments".to_string(),
            ));
        }
    };

    for arg in args.iter().skip(1) {
        let next = match arg.eval(env)? {
            Expression::Map(m) => m,
            _ => {
                return Err(LmError::CustomError(
                    "deep_merge requires maps as arguments".to_string(),
                ));
            }
        };
        base = deep_merge_maps(&base, next.as_ref())?;
    }

    Ok(Expression::from(base))
}

fn deep_merge_maps(
    a: &BTreeMap<String, Expression>,
    b: &BTreeMap<String, Expression>,
) -> Result<BTreeMap<String, Expression>, LmError> {
    let mut result = a.clone();

    for (k, v) in b.iter() {
        if let Some(existing) = result.get(k) {
            if let (Expression::Map(ma), Expression::Map(mb)) = (existing, v) {
                result.insert(
                    k.clone(),
                    Expression::Map(Rc::new(deep_merge_maps(ma.as_ref(), mb.as_ref())?)),
                );
                continue;
            }
        }
        result.insert(k.clone(), v.clone());
    }

    Ok(result)
}
