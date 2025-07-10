use crate::{Environment, Expression, LmError};
use common_macros::hash_map;

use std::collections::BTreeMap;
use std::rc::Rc;

pub fn get() -> Expression {
    (hash_map! {
        String::from("pprint") => Expression::builtin("pprint", super::pretty_print, "pretty print", "<map>"),

        // 检查操作
        String::from("len") => Expression::builtin("len", super::len, "get length of map", "<map>"),
        String::from("insert") => Expression::builtin("insert", super::insert, "insert item into map", "<key> <value> <map>"),
        String::from("flatten") => Expression::builtin("flatten", super::flatten_wrapper, "flatten nested structure", "<map>"),

        String::from("has") => Expression::builtin("has", has, "check if a map has a key", "<key> <map>"),

        // 数据获取
        String::from("get") => Expression::builtin("get", super::get, "get value from nested map/list/range using dot notation path", "<path> <map|list|range>"),
        String::from("at") => Expression::builtin("at", at, "get value from map", "<key> <map>"),
        String::from("items") => Expression::builtin("items", items, "get the items of a map or list", "<map>"),
        String::from("keys") => Expression::builtin("keys", keys, "get the keys of a map", "<map>"),
        String::from("values") => Expression::builtin("values", values, "get the values of a map", "<map>"),
        // 查找
        String::from("find") => Expression::builtin("find", find, "find first key-value pair matching condition", "<predicate_fn> <map>"),
        String::from("filter") => Expression::builtin("filter", filter, "filter map by condition", "<predicate_fn> <map>"),
        // 结构修改
        String::from("remove") => Expression::builtin("remove", remove, "remove a key-value pair from a map", "<key> <map>"),
        String::from("set") => Expression::builtin("set", set_map, "set value for existing key in map", "<key> <value> <map>"),
        // 创建操作
        String::from("from_items") => Expression::builtin("from_items", from_items, "create a map from a list of key-value pairs", "<items>"),

        // 集合运算
        String::from("union") => Expression::builtin("union", union, "combine two maps", "<map1> <map2>"),
        String::from("intersect") => Expression::builtin("intersect", intersect, "get the intersection of two maps", "<map1> <map2>"),
        String::from("difference") => Expression::builtin("difference", difference, "get the difference of two maps", "<map1> <map2>"),
        String::from("merge") => Expression::builtin("merge", deep_merge, "recursively merge two or more maps", "<map1> <map2> [<map3> ...]"),

        // 转换操作
        String::from("map") => Expression::builtin("map", map_map, "transform map keys and values with provided functions", "<key_fn> <val_fn> <map>"),
    })
    .into()
}

// Helper function implementations
fn at(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("at", args, 2)?;
    let k = super::get_string_arg(args[0].eval(env)?)?;
    let map = get_map_arg(args[1].eval(env)?)?;

    map.as_ref()
        .get(&k)
        .cloned()
        .ok_or_else(|| LmError::CustomError(format!("key '{}' not found in Map `{}`", k, args[1])))
}

fn items(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
        Expression::HMap(map) => {
            let items = map
                .iter()
                .map(|(k, v)| Expression::from(vec![Expression::String(k.clone()), v.clone()]))
                .collect();
            Expression::List(Rc::new(items))
        }
        _ => Expression::None,
    })
}

fn keys(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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
        Expression::HMap(map) => {
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

fn values(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("values", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => Expression::List(Rc::new(map.as_ref().values().cloned().collect())),
        Expression::HMap(map) => {
            Expression::List(Rc::new(map.as_ref().values().cloned().collect()))
        }
        _ => Expression::None,
    })
}

fn set_map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("set", args, 3)?;
    let expr = args[2].eval(env)?;
    let value = args[1].eval(env)?;
    let key_str = super::get_string_arg(args[0].eval(env)?)?;

    Ok(match expr {
        Expression::Map(map) => {
            if map.as_ref().contains_key(&key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str, value);
                Expression::Map(Rc::new(new_map))
            } else {
                return Err(LmError::CustomError(format!(
                    "key '{key_str}' not found in map"
                )));
            }
        }
        Expression::HMap(map) => {
            if map.as_ref().contains_key(&key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str, value);
                Expression::HMap(Rc::new(new_map))
            } else {
                return Err(LmError::CustomError(format!(
                    "key '{key_str}' not found in map"
                )));
            }
        }
        _ => return Err(LmError::CustomError("expected map".to_string())),
    })
}

fn remove(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("remove", args, 2)?;
    let expr = args[1].eval(env)?;
    let key = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => {
            let mut new_map = map.as_ref().clone();
            new_map.remove(&key.to_string());
            Expression::Map(Rc::new(new_map))
        }
        Expression::HMap(map) => {
            let mut new_map = map.as_ref().clone();
            new_map.remove(&key.to_string());
            Expression::HMap(Rc::new(new_map))
        }
        _ => Expression::None,
    })
}

fn has(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("has", args, 2)?;
    let expr = args[1].eval(env)?;
    let key = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => Expression::Boolean(map.as_ref().contains_key(&key.to_string())),
        Expression::HMap(map) => Expression::Boolean(map.as_ref().contains_key(&key.to_string())),
        _ => Expression::None,
    })
}

fn from_items(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn union(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn intersect(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn difference(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
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

fn map_map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("map.map", args, 2..=3)?;

    let map = get_map_arg(args.last().unwrap().eval(env)?)?;

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

fn deep_merge(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("merge", args, 2..)?;

    let base = get_map_arg(args[0].eval(env)?)?;
    let mut r = BTreeMap::new();
    for arg in args.iter().skip(1) {
        let next = get_map_arg(arg.eval(env)?)?;
        r = deep_merge_maps(base.as_ref(), next.as_ref())?;
    }

    Ok(Expression::from(r))
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

fn find(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("map.find", args, 2)?;
    let map = get_map_arg(args[1].eval(env)?)?;

    let predicate = args[0].eval(env)?;

    for (k, v) in map.iter() {
        let result = Expression::Apply(
            Rc::new(predicate.clone()),
            Rc::new(vec![Expression::String(k.clone()), v.clone()]),
        )
        .eval(env)?;

        if let Expression::Boolean(true) = result {
            return Ok(Expression::from(vec![
                Expression::String(k.clone()),
                v.clone(),
            ]));
        }
    }

    Ok(Expression::None)
}

fn filter(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("map.filter", args, 2)?;
    let map = get_map_arg(args[1].eval(env)?)?;

    let predicate = args[0].eval(env)?;
    let mut new_map = BTreeMap::new();

    for (k, v) in map.iter() {
        let result = Expression::Apply(
            Rc::new(predicate.clone()),
            Rc::new(vec![Expression::String(k.clone()), v.clone()]),
        )
        .eval(env)?;

        if let Expression::Boolean(true) = result {
            new_map.insert(k.clone(), v.clone());
        }
    }

    Ok(Expression::from(new_map))
}

fn get_map_arg(expr: Expression) -> Result<Rc<BTreeMap<String, Expression>>, LmError> {
    match expr {
        Expression::Map(s) => Ok(s),
        e => Err(LmError::TypeError {
            expected: "Map".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
