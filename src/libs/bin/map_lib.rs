use std::{collections::HashMap, rc::Rc};

use crate::libs::helper::{check_args_len, check_exact_args_len, get_string_arg};
use crate::libs::lazy_module::LazyModule;
use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind, libs::BuiltinInfo, reg_info, reg_lazy,
};

use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // pprint,
        // 检查操作
        // len, insert, flatten,
        has,
        // 数据获取
        // get,
        at, items, keys, values,
        // 查找
        find, filter,
        // 结构修改
        remove,
        // set,
        // 创建操作
        from_items,
        // 集合运算
        union, intersect, difference, merge,
        // 转换操作
        map
    })
}
pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({
        pprint => "pretty print", "<map>"

        // 检查操作
        len => "get length of map", "<map>"
        insert => "insert item into map", "<key> <value> <map>"
        flatten => "flatten nested structure", "<map>"
        has => "check if a map has a key", "<key> <map>"

        // 数据获取
        get => "get value from nested map/list/range using dot notation path", "<path> <map|list|range>"
        at => "get value from map", "<key> <map>"
        items => "get the items of a map or list", "<map>"
        keys => "get the keys of a map", "<map>"
        values => "get the values of a map", "<map>"
        // 查找
        find => "find first key-value pair matching condition", "<predicate_fn> <map>"
        filter => "filter map by condition", "<predicate_fn> <map>"
        // 结构修改
        remove => "remove a key-value pair from a map", "<key> <map>"
        set => "set value for existing key in map", "<key> <value> <map>"
        // 创建操作
        from_items => "create a map from a list of key-value pairs", "<items>"

        // 集合运算
        union => "combine two maps", "<map1> <map2>"
        intersect => "get the intersection of two maps", "<map1> <map2>"
        difference => "get the difference of two maps", "<map1> <map2>"
        merge => "recursively merge two or more maps", "<map1> <map2> [<map3> ...]"

        // 转换操作
        map => "transform map keys and values with provided functions", "<key_fn> <val_fn> <map>"
    })
}

// Helper Functions
fn get_map_arg(
    expr: Expression,
    ctx: &Expression,
) -> Result<Rc<BTreeMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::Map(s) => Ok(s),
        Expression::HMap(rc_hashmap) => {
            let btree_map: BTreeMap<_, _> = rc_hashmap
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(Rc::new(btree_map))
        }
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Map".to_string(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}
// 检查操作函数
fn at(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("at", args, 2, ctx)?;
    let map = get_map_arg(args[0].eval(env)?, ctx)?;
    let k = get_string_arg(args[1].eval(env)?, ctx)?;

    map.as_ref().get(&k).cloned().ok_or_else(|| {
        RuntimeError::common(
            format!("key '{}' not found in Map", k).into(),
            ctx.clone(),
            0,
        )
    })
}

fn has(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("has", args, 2, ctx)?;
    let map = args[0].eval(env)?;
    let key = args[1].eval(env)?;

    Ok(match map {
        Expression::Map(map) => Expression::Boolean(map.contains_key(&key.to_string())),
        Expression::HMap(map) => Expression::Boolean(map.contains_key(&key.to_string())),
        _ => Expression::None,
    })
}
// 数据获取函数
fn items(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("items", args, 1, ctx)?;
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

fn keys(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("keys", args, 1, ctx)?;
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

fn values(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("values", args, 1, ctx)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::Map(map) => Expression::List(Rc::new(map.as_ref().values().cloned().collect())),
        Expression::HMap(map) => {
            Expression::List(Rc::new(map.as_ref().values().cloned().collect()))
        }
        _ => Expression::None,
    })
}
// 查找函数
fn find(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("map.find", args, 2, ctx)?;
    let map = get_map_arg(args[0].eval(env)?, ctx)?;
    let predicate = args[1].eval(env)?;

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

fn filter(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("map.filter", args, 2, ctx)?;
    let map = get_map_arg(args[0].eval(env)?, ctx)?;
    let predicate = args[1].eval(env)?;
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
// 结构修改函数
fn remove(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove", args, 2, ctx)?;
    let map = args[0].eval(env)?;
    let key = args[1].eval(env)?;

    Ok(match map {
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

fn set(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set", args, 3, ctx)?;
    let map = args[0].eval(env)?;
    let key_str = get_string_arg(args[1].eval(env)?, ctx)?;
    let value = args[2].eval(env)?;

    Ok(match map {
        Expression::Map(map) => {
            if map.as_ref().contains_key(&key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str, value);
                Expression::Map(Rc::new(new_map))
            } else {
                return Err(RuntimeError::common(
                    format!("key '{key_str}' not found in map").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
        Expression::HMap(map) => {
            if map.as_ref().contains_key(&key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str, value);
                Expression::HMap(Rc::new(new_map))
            } else {
                return Err(RuntimeError::common(
                    format!("key '{key_str}' not found in map").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
        _ => {
            return Err(RuntimeError::common(
                "expected map".to_string().into(),
                ctx.clone(),
                0,
            ));
        }
    })
}
// 创建操作函数
fn from_items(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("from_items", args, 1, ctx)?;
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
// 集合运算函数
fn union(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("union", args, 2, ctx)?;
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

fn intersect(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("intersect", args, 2, ctx)?;
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

fn difference(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("difference", args, 2, ctx)?;
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

fn merge(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("merge", args, 2.., ctx)?;

    let base = get_map_arg(args[0].eval(env)?, ctx)?;
    let mut r = BTreeMap::new();
    for arg in args.iter().skip(1) {
        let next = get_map_arg(arg.eval(env)?, ctx)?;
        r = deep_merge_maps(base.as_ref(), next.as_ref())?;
    }

    Ok(Expression::from(r))
}

fn deep_merge_maps(
    a: &BTreeMap<String, Expression>,
    b: &BTreeMap<String, Expression>,
) -> Result<BTreeMap<String, Expression>, RuntimeError> {
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
// 转换操作函数
fn map(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("map", args, 3, ctx)?;

    let map = get_map_arg(args[0].eval(env)?, ctx)?;

    let key_func = args[1].eval(env)?;
    let val_func = args[2].eval(env)?;

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
