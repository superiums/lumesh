use std::rc::Rc;

use crate::eval::{State, is_strict};
use crate::libs::bin::top;
use crate::libs::helper::{check_args_len, check_exact_args_len, check_fn_arg, get_string_ref};
use crate::libs::lazy_module::LazyModule;
use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind, libs::BuiltinInfo, reg_info, reg_lazy,
};

use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // pprint,
        // from top
        len, insert, flatten, get,
        // 检查操作
        has,
        // 数据获取
        at, items, keys, values,
        // 查找
        find, filter,
        // 结构修改
        set,
        remove,
        // 创建操作
        from_items,
        // 集合运算
        union, intersect, difference, merge,
        // 转换操作
        map
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // pprint => "pretty print", "<map>"

        // 检查操作
        len => "get length of map", "<map>"
        insert => "insert item into map", "<map> <key> <value>"
        flatten => "flatten nested structure", "<map>"
        has => "check if a map has a key", "<map> <key>"

        // 数据获取
        get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
        at => "get value from map", "<map> <key>"
        items => "get the items of a map or list", "<map>"
        keys => "get the keys of a map", "<map>"
        values => "get the values of a map", "<map>"
        // 查找
        find => "find first key-value pair matching condition", "<map> <predicate_fn>"
        filter => "filter map by condition", "<map> <predicate_fn>"
        // 结构修改
        remove => "remove a key-value pair from a map", "<map> <key>"
        set => "set value for existing key in map", "<map> <key> <value>"
        // 创建操作
        from_items => "create a map from a list of key-value pairs", "<items>"

        // 集合运算
        union => "combine two maps", "<map1> <map2>"
        intersect => "get the intersection of two maps", "<map1> <map2>"
        difference => "get the difference of two maps", "<map1> <map2>"
        merge => "recursively merge two or more maps", "<map1> <map2> [<map3> ...]"

        // 转换操作
        map => "transform map keys and values with provided functions", "<map> <key_fn> <val_fn>"
    })
}

// ---from top---
fn insert(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::insert(args, env, ctx)
}
fn len(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::len(args, env, ctx)
}
fn get(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::get(args, env, ctx)
}
fn flatten(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::flatten(args, env, ctx)
}

// Helper Functions

fn map_err(expr: &Expression, ctx: &Expression) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::TypeError {
            expected: "Map".to_string(),
            found: expr.type_name(),
            sym: expr.to_string(),
        },
        ctx.clone(),
        0,
    )
}
// 检查操作函数
fn at(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("at", args, 2, ctx)?;

    let key = get_string_ref(&args[1], ctx)?.as_str();
    match &args[0] {
        Expression::Map(m) => m.get(key),
        Expression::HMap(m) => m.get(key),
        expr => return Err(map_err(expr, ctx)),
    }
    .cloned()
    .ok_or_else(|| {
        RuntimeError::common(
            format!("key '{}' not found in Map", key).into(),
            ctx.clone(),
            0,
        )
    })
}

fn has(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("has", args, 2, ctx)?;
    let key = get_string_ref(&args[1], ctx)?.as_str();
    let r = match &args[0] {
        Expression::Map(m) => m.contains_key(key),
        Expression::HMap(m) => m.contains_key(key),
        expr => return Err(map_err(expr, ctx)),
    };
    Ok(Expression::Boolean(r))
}
// 数据获取函数
fn items(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("items", args, 1, ctx)?;

    let r = match &args[0] {
        Expression::Map(m) => m
            .iter()
            .map(|(k, v)| Expression::from(vec![Expression::String(k.clone()), v.clone()]))
            .collect::<Vec<_>>(),
        Expression::HMap(m) => m
            .iter()
            .map(|(k, v)| Expression::from(vec![Expression::String(k.clone()), v.clone()]))
            .collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };
    Ok(Expression::from(r))
}

fn keys(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("keys", args, 1, ctx)?;
    let r = match &args[0] {
        Expression::Map(m) => m
            .keys()
            .map(|k| Expression::String(k.clone()))
            .collect::<Vec<_>>(),
        Expression::HMap(m) => m
            .keys()
            .map(|k| Expression::String(k.clone()))
            .collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };
    Ok(Expression::from(r))
}

fn values(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("values", args, 1, ctx)?;
    let r = match &args[0] {
        Expression::Map(m) => m.values().cloned().collect::<Vec<_>>(),
        Expression::HMap(m) => m.values().cloned().collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };
    Ok(Expression::from(r))
}
// 查找函数
fn find(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("find", args, 2, ctx)?;

    let predicate = &args[1];
    check_fn_arg(&predicate, 2, ctx)?;

    let items = match &args[0] {
        Expression::Map(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),
        Expression::HMap(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };

    let mut state = State::new(is_strict(env));
    for it in items {
        if predicate
            .eval_apply(predicate, &it, &mut state, env, 0)?
            .is_truthy()
        {
            return Ok(Expression::from(it));
        }
    }

    Ok(Expression::None)
}

fn filter(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filter", args, 2, ctx)?;

    let predicate = &args[1];
    check_fn_arg(&predicate, 2, ctx)?;

    let items = match &args[0] {
        Expression::Map(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),
        Expression::HMap(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };

    let mut new_map = BTreeMap::new();
    let mut state = State::new(is_strict(env));
    for it in items {
        if predicate
            .eval_apply(predicate, &it, &mut state, env, 0)?
            .is_truthy()
        {
            new_map.insert(it[0].to_string(), it[1].clone());
        }
    }

    Ok(Expression::from(new_map))
}
// 结构修改函数
fn remove(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove", args, 2, ctx)?;
    let map = &args[0];
    let key = &args[1];

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
        expr => return Err(map_err(expr, ctx)),
    })
}

fn set(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set", args, 3, ctx)?;
    let map = &args[0];
    let key_str: &str = get_string_ref(&args[1], ctx)?;
    let value = &args[2];

    match map {
        Expression::Map(map) => {
            if map.as_ref().contains_key(key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str.to_string(), value.clone());
                Ok(Expression::Map(Rc::new(new_map)))
            } else {
                return Err(RuntimeError::common(
                    format!("key '{key_str}' not found in map").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
        Expression::HMap(map) => {
            if map.as_ref().contains_key(key_str) {
                let mut new_map = map.as_ref().clone();
                new_map.insert(key_str.to_string(), value.clone());
                Ok(Expression::HMap(Rc::new(new_map)))
            } else {
                return Err(RuntimeError::common(
                    format!("key '{key_str}' not found in map").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
        expr => return Err(map_err(expr, ctx)),
    }
}
// 创建操作函数
fn from_items(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("from_items", args, 1, ctx)?;
    let expr = &args[0];

    match expr {
        Expression::List(list) => {
            let mut map = BTreeMap::new();
            for item in list.as_ref() {
                if let Expression::List(pair) = item {
                    if pair.as_ref().len() == 2 {
                        map.insert(pair.as_ref()[0].to_string(), pair.as_ref()[1].clone());
                    }
                }
            }
            Ok(Expression::from(map))
        }
        expr => return Err(map_err(expr, ctx)),
    }
}
// 集合运算函数
fn union(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("union", args, 2, ctx)?;
    let expr1 = &args[0];
    let expr2 = &args[1];

    match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = map1.as_ref().clone();
            new_map.extend(map2.as_ref().iter().map(|(k, v)| (k.clone(), v.clone())));
            Ok(Expression::Map(Rc::new(new_map)))
        }
        (Expression::Map(_), expr) => return Err(map_err(expr, ctx)),
        (expr, _) => return Err(map_err(expr, ctx)),
    }
}

fn intersect(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("intersect", args, 2, ctx)?;
    let expr1 = &args[0];
    let expr2 = &args[1];

    match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = BTreeMap::new();
            for (k, v) in map1.as_ref() {
                if map2.as_ref().contains_key(k) {
                    new_map.insert(k.clone(), v.clone());
                }
            }
            Ok(Expression::from(new_map))
        }
        (Expression::Map(_), expr) => return Err(map_err(expr, ctx)),
        (expr, _) => return Err(map_err(expr, ctx)),
    }
}

fn difference(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("difference", args, 2, ctx)?;
    let expr1 = &args[0];
    let expr2 = &args[1];

    match (expr1, expr2) {
        (Expression::Map(map1), Expression::Map(map2)) => {
            let mut new_map = BTreeMap::new();
            for (k, v) in map1.as_ref() {
                if !map2.as_ref().contains_key(k) {
                    new_map.insert(k.clone(), v.clone());
                }
            }
            Ok(Expression::from(new_map))
        }
        (Expression::Map(_), expr) => return Err(map_err(expr, ctx)),
        (expr, _) => return Err(map_err(expr, ctx)),
    }
}

fn merge(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("merge", args, 2.., ctx)?;

    let r = BTreeMap::new();
    let maps = args
        .iter()
        .map(|a| {
            if let Expression::Map(m) = a {
                m.as_ref()
            } else {
                &r
            }
        })
        .collect::<Vec<_>>();
    let base = maps[0];

    let mut r = BTreeMap::new();
    for next in maps.iter().skip(1) {
        r = deep_merge_maps(base, next)?;
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

    let key_func = &args[1];
    let val_func = &args[2];
    check_fn_arg(&key_func, 1, ctx)?;
    check_fn_arg(&val_func, 1, ctx)?;

    let items = match &args[0] {
        Expression::Map(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),
        Expression::HMap(m) => m
            .iter()
            .map(|(k, v)| vec![Expression::String(k.clone()), v.clone()])
            .collect::<Vec<_>>(),

        expr => return Err(map_err(expr, ctx)),
    };

    let mut new_map = BTreeMap::new();
    let mut state = State::new(is_strict(env));
    for it in items {
        let new_k = key_func.eval_apply(key_func, &vec![it[0].clone()], &mut state, env, 0)?;
        let new_v = val_func.eval_apply(key_func, &vec![it[1].clone()], &mut state, env, 0)?;

        new_map.insert(new_k.to_string(), new_v);
    }

    Ok(Expression::from(new_map))
}
