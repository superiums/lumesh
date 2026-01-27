// use super::math_module::{average, max, min, sum};
use std::rc::Rc;

use crate::eval::{State, is_strict};
use crate::expression::eval2::execute_iteration;
use crate::libs::bin::{math_lib, top};
use crate::libs::helper::{
    check_args_len, check_exact_args_len, check_fn_arg, get_integer_ref, get_string_args,
    get_string_ref,
};
use crate::libs::lazy_module::LazyModule;
use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind, libs::BuiltinInfo, reg_info,
    reg_lazy,
};

use std::cmp::Ordering;
use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        //打印
        // pprint,
        //数学统计
        max,min,sum,average,
        //读取操作
        get,len,insert,rev,flatten,
        first,last,at,take,drop,is_empty,
        //查找操作
        contains,find,find_last,
        //修改操作
        append,prepend,unique,split_at,sort,group,remove_at,remove,set,
        //创建操作
        concat,from,
        //遍历操作
        map,items,filter,filter_map,any,all,
        //转换操作
        join,to_map,
        //结构操作
        transpose,chunk,foldl,foldr,zip,unzip,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // 打印
        // pprint => "pretty print", "<list>"

        // 数学统计
        max => "get max value in an array or multi args", "<num1> <num2> ... | <array>"
        min => "get min value in an array or multi args", "<num1> <num2> ... | <array>"
        sum => "sum a list of numbers", "<num1> <num2> ... | <array>"
        average => "get the average of a list of numbers", "<num1> <num2> ... | <array>"

        // 读取操作
        get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
        len => "get length of list", "<list>"
        insert => "insert item into list", "<list> <index> <value>"
        rev => "reverse sequence", "<list>"
        flatten => "flatten nested structure", "<collection>"
        is_empty => "is this list empty?", "<list>"

        first => "get the first element of a list", "<list>"
        last => "get the last element of a list", "<list>"
        at => "get the nth element of a list", "<list> <index>"
        take => "take the first n elements of a list", "<list> <count>"
        drop => "drop the first n elements of a list", "<list> <count>"
        // 查找操作
        contains => "check if list contains an item", "<list> <item>"
        find => "find first index of matching element", "<list> <item|fn> [start_index]"
        find_last => "find last index of item", "<list> <item|fn> [start_index]"

        // 修改操作
        append => "append an element to a list", "<list> <element>"
        prepend => "prepend an element to a list", "<list> <element>"
        unique => "remove duplicates from a list while preserving order", "<list>"
        split_at => "split a list at a given index", "<list> <index>"
        // splice => "change contents by removing/adding elements", "<start> <deleteCount> [items...] <list>"
        sort => "sort a string/list, optionally with a key function or key_list", "<string|list> [key_fn|key_list|keys...]"
        group => "group list elements by key function", "<list> <key_fn|key>"
        remove_at => "remove n elements starting from index", "<list> <index> [count]"
        remove => "remove first matching element", "<list> <item> [all?]"
        set => "set element at existing index", "<list> <index> <value>"
        // 创建操作
        concat => "concatenate multiple lists into one", "<list1|item1> <list2|item2> ..."
        from => "create a list from a range", "<range|item...>"

        // 遍历操作
        map => "apply function for each element", "<list> <fn>"
        items => "iterate over index-value pairs", "<list>"
        filter => "filter elements by condition", "<list> <fn>"
        filter_map => "filter and map in one pass", "<list> <fn>"
        any => "test if any element passes condition", "<list> <fn>"
        all => "test if all elements pass condition", "<list> <fn>"

        // 转换操作
        join => "join string list with separator", "<list> <separator>"
        to_map => "convert list to map using key function", "<list> [key_fn] [val_fn]"

        // 结构操作
        transpose => "transpose matrix (list of lists)", "<matrix>"
        chunk => "split list into chunks of size n", "<list> <size>"
        foldl => "fold list from left with function", "<list> <fn> <init>"
        foldr => "fold list from right with function", "<list> <fn> <init>"
        zip => "zip two lists into list of pairs", "<list1> <list2>"
        unzip => "unzip list of pairs into two lists", "<list_of_pairs>"
    })
}

// ---from math---
fn max(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    math_lib::max(args, env, ctx)
}
fn min(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    math_lib::min(args, env, ctx)
}
fn sum(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    math_lib::sum(args, env, ctx)
}
fn average(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    math_lib::average(args, env, ctx)
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
fn rev(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::rev(args, env, ctx)
}
fn flatten(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    top::flatten(args, env, ctx)
}

// ---self---
fn is_empty(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_empty", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    Ok(Expression::Boolean(list.is_empty()))
}
fn first(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("first", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    list.as_ref().first().cloned().ok_or_else(|| {
        RuntimeError::common("cannot get first of empty list".into(), ctx.clone(), 0)
    })
}

fn last(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("last", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    list.as_ref()
        .last()
        .cloned()
        .ok_or_else(|| RuntimeError::common("cannot get last of empty list".into(), ctx.clone(), 0))
}
fn clamp(n: Int, len: usize) -> usize {
    if n < 0 {
        (len + n as usize).max(0)
    } else {
        (n as usize).min(len)
    }
}
fn at(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("at", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

    let index = clamp(n, list.len());

    list.get(index).cloned().ok_or(RuntimeError::new(
        RuntimeErrorKind::IndexOutOfBounds {
            index: n,
            len: list.as_ref().len(),
        },
        ctx.clone(),
        0,
    ))
}

fn take(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("take", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

    let count = clamp(n, list.len());

    Ok(Expression::List(Rc::new(
        list.as_ref().iter().take(count).cloned().collect(),
    )))
}

fn drop(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("drop", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

    let count = clamp(n, list.len());

    Ok(Expression::List(Rc::new(
        list.as_ref().iter().skip(count).cloned().collect(),
    )))
}
// 查找操作函数
fn contains(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("contains", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    Ok(Expression::Boolean(list.as_ref().contains(&args[1])))
}

fn find(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("find", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let target = &args[1];
    let start = if args.len() == 3 {
        get_integer_ref(&args[2], ctx)? as usize
    } else {
        0
    };

    match &target {
        Expression::Function(..) | Expression::Lambda(..) => {
            for (i, item) in list.as_ref().iter().enumerate().skip(start) {
                match Expression::Apply(Rc::new(target.clone()), Rc::new(vec![item.clone()]))
                    .eval(env)?
                {
                    Expression::Boolean(true) => return Ok(Expression::Integer(i as Int)),
                    _ => continue,
                }
            }
            Ok(Expression::None)
        }
        _ => Ok(
            match list.as_ref().iter().skip(start).position(|x| x == target) {
                Some(index) => Expression::Integer(index as Int),
                None => Expression::None,
            },
        ),
    }
}

fn find_last(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("find_last", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let target = &args[1];
    let start = if args.len() == 3 {
        get_integer_ref(&args[2], ctx)? as usize
    } else {
        0
    };

    match &target {
        Expression::Function(..) | Expression::Lambda(..) => {
            for (i, item) in list.as_ref().iter().enumerate().rev().skip(start) {
                match Expression::Apply(Rc::new(target.clone()), Rc::new(vec![item.clone()]))
                    .eval(env)?
                {
                    Expression::Boolean(true) => return Ok(Expression::Integer(i as Int)),
                    _ => continue,
                }
            }
            Ok(Expression::None)
        }
        _ => Ok(
            match list
                .as_ref()
                .iter()
                .rev()
                .skip(start)
                .position(|x| x == target)
            {
                Some(index) => Expression::Integer((list.as_ref().len() - 1 - index) as Int),
                None => Expression::None,
            },
        ),
    }
}
// 修改操作函数
fn append(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("append", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let item = args[1].clone();

    let mut new_list = list.as_ref().to_vec();
    new_list.push(item);
    Ok(Expression::List(Rc::new(new_list)))
}

fn prepend(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("prepend", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let head = args[1].clone();

    let mut new_list = Vec::with_capacity(list.as_ref().len() + 1);
    new_list.push(head);
    new_list.extend(list.as_ref().iter().cloned());
    Ok(Expression::List(Rc::new(new_list)))
}

fn unique(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("unique", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for item in list.as_ref().iter() {
        if seen.insert(item.to_string()) {
            result.push(item.clone());
        }
    }
    Ok(Expression::List(Rc::new(result)))
}

fn split_at(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("split_at", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

    let index = if n < 0 {
        (list.as_ref().len() as Int + n).max(0) as usize
    } else {
        (n as usize).min(list.as_ref().len())
    };

    let (first, second) = list.as_ref().split_at(index);
    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(first.to_vec())),
        Expression::List(Rc::new(second.to_vec())),
    ])))
}

fn sort(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("sort", args, 1.., ctx)?;

    let list = &args[0];
    let (func, headers) = match args.len() {
        2 => {
            let key_func = &args[1];
            match key_func {
                Expression::Lambda(..) | Expression::Function(..) => {
                    (Some(Rc::new(key_func)), None)
                }
                Expression::Symbol(s) | Expression::String(s) => (None, Some(vec![s.to_string()])),
                Expression::List(s) => (
                    None,
                    Some(s.iter().map(|e| e.to_string()).collect::<Vec<_>>()),
                ),
                _ => (None, None),
            }
        }
        3.. => {
            let cols = get_string_args(&args[1..], env, ctx)?;
            (None, Some(cols))
        }
        _ => (None, None),
    };

    let mut sorted: Vec<_> = match list {
        Expression::List(l) => l.as_ref().clone(),
        Expression::String(s) => {
            let mut elist = s
                .lines()
                .map(|s| Expression::String(s.to_owned()))
                .collect::<Vec<_>>();
            if elist.len() < 2 {
                elist = s
                    .split_ascii_whitespace()
                    .map(|s| Expression::String(s.to_owned()))
                    .collect::<Vec<_>>();
                if elist.len() < 2 {
                    elist = s
                        .split_terminator(";")
                        .map(|s| Expression::String(s.to_owned()))
                        .collect::<Vec<_>>();
                    if elist.len() < 2 {
                        elist = s
                            .split_terminator(",")
                            .map(|s| Expression::String(s.to_owned()))
                            .collect::<Vec<_>>();
                    }
                }
            }
            elist
        }
        s => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "List as last argument".to_string(),
                    sym: s.to_string(),
                    found: s.type_name(),
                },
                ctx.clone(),
                0,
            ));
        }
    };

    if let Some(sort_func) = func {
        sorted.sort_by(|a, b| {
            let sort_result = Expression::Apply(
                Rc::new((*sort_func).clone()),
                Rc::new(vec![a.clone(), b.clone()]),
            )
            .eval(env);
            match sort_result {
                Ok(Expression::Integer(i)) => match i {
                    1.. => Ordering::Greater,
                    0 => Ordering::Equal,
                    ..0 => Ordering::Less,
                },
                Ok(Expression::Boolean(b)) => match b {
                    true => Ordering::Greater,
                    false => Ordering::Less,
                },
                _ => Ordering::Equal,
            }
        });
    } else if let Some(heads) = headers {
        sorted.sort_by(|a, b| match (a, b) {
            (Expression::Map(map_a), Expression::Map(map_b)) => {
                let key_a = heads
                    .iter()
                    .map(|col| map_a.get(col).unwrap_or(&Expression::None))
                    .collect::<Vec<_>>();
                let key_b = heads
                    .iter()
                    .map(|col| map_b.get(col).unwrap_or(&Expression::None))
                    .collect::<Vec<_>>();

                key_a
                    .iter()
                    .zip(key_b.iter())
                    .find_map(|(a_val, b_val)| match a_val.partial_cmp(b_val) {
                        Some(Ordering::Equal) => None,
                        other => other,
                    })
                    .unwrap_or(Ordering::Equal)
            }
            (Expression::HMap(map_a), Expression::HMap(map_b)) => {
                let key_a = heads
                    .iter()
                    .map(|col| map_a.get(col).unwrap_or(&Expression::None))
                    .collect::<Vec<_>>();
                let key_b = heads
                    .iter()
                    .map(|col| map_b.get(col).unwrap_or(&Expression::None))
                    .collect::<Vec<_>>();

                key_a
                    .iter()
                    .zip(key_b.iter())
                    .find_map(|(a_val, b_val)| match a_val.partial_cmp(b_val) {
                        Some(Ordering::Equal) => None,
                        other => other,
                    })
                    .unwrap_or(Ordering::Equal)
            }
            _ => Ordering::Equal,
        });
    } else {
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    Ok(Expression::List(Rc::new(sorted)))
}

fn group(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("group", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let key_func = &args[1];

    let mut groups: BTreeMap<String, Vec<Expression>> = BTreeMap::new();

    match key_func {
        Expression::Lambda(..) | Expression::Function(..) => {
            let key_f = Rc::new(key_func);
            for item in list.as_ref().iter() {
                let key =
                    match Expression::Apply(Rc::new((*key_f).clone()), Rc::new(vec![item.clone()]))
                        .eval(env)?
                    {
                        Expression::String(s) => s,
                        other => other.to_string(),
                    };
                groups.entry(key).or_default().push(item.clone());
            }
        }
        Expression::Symbol(k) | Expression::String(k) => {
            for item in list.as_ref().iter() {
                let kk: &str = &k;
                let keyitem = match item {
                    Expression::Map(m) => m.get(kk),
                    Expression::HMap(m) => m.get(kk),
                    _ => {
                        return Err(RuntimeError::common(
                            "group by key can only apply to a map".to_string().into(),
                            ctx.clone(),
                            0,
                        ));
                    }
                };
                if let Some(key) = keyitem {
                    groups
                        .entry(key.to_string())
                        .or_default()
                        .push(item.clone());
                } else {
                    return Err(RuntimeError::common(
                        format!("no such key found in map: `{k}`").into(),
                        ctx.clone(),
                        0,
                    ));
                }
            }
        }
        _ => {
            return Err(RuntimeError::common(
                "group requires key-func or key".to_string().into(),
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::from(groups))
}

fn remove_at(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("remove_at", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let index = get_integer_ref(&args[1], ctx)?;
    let count = if args.len() == 3 {
        get_integer_ref(&args[2], ctx)?
    } else {
        1
    };

    if count <= 0 {
        return Ok(Expression::List(list.clone()));
    }

    let list_len = list.as_ref().len() as Int;
    let start_idx = if index < 0 {
        (list_len + index).max(0) as usize
    } else {
        (index as usize).min(list_len as usize)
    };

    let end_idx = (start_idx + count as usize).min(list_len as usize);

    if start_idx >= list_len as usize {
        return Ok(Expression::List(list.clone()));
    }

    let mut new_list = Vec::new();
    new_list.extend(list.as_ref().iter().take(start_idx).cloned());
    new_list.extend(list.as_ref().iter().skip(end_idx).cloned());

    Ok(Expression::List(Rc::new(new_list)))
}
fn remove(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("remove", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let item = &args[1];

    let all = if args.len() == 3 {
        if let &Expression::Boolean(b) = &args[2] {
            b
        } else {
            false
        }
    } else {
        false
    };

    if all {
        let new_list = list
            .iter()
            .filter(|x| *x != item)
            .cloned()
            .collect::<Vec<_>>();
        Ok(Expression::from(new_list))
    } else if let Some(pos) = list.iter().position(|x| x == item) {
        let mut new_list = list.as_ref().clone();
        new_list.remove(pos);
        Ok(Expression::from(new_list))
    } else {
        Ok(Expression::List(list.clone()))
    }
}

fn set(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set", args, 3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

    let val = &args[2];

    let index = n as usize;
    if index < list.as_ref().len() {
        let mut result = list.as_ref().clone();
        result[index] = val.clone();
        Ok(Expression::from(result))
    } else {
        Err(RuntimeError::common(
            format!(
                "index {} out of bounds for list of length {}",
                n,
                list.as_ref().len()
            )
            .into(),
            ctx.clone(),
            0,
        ))
    }
}
// 创建操作函数
fn concat(
    args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    Ok(Expression::List(Rc::new(args.to_vec())))
}

fn from(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => Err(RuntimeError::common(
            "requires a range (a..b) or some elements as arguments"
                .to_string()
                .into(),
            ctx.clone(),
            0,
        )),
        1 => match &args[0] {
            Expression::Range(r, step) => Ok(Expression::from(
                r.clone().step_by(step.clone()).collect::<Vec<Int>>(),
            )),
            _ => Err(RuntimeError::common(
                "the only arg should be a range (a..b)".to_string().into(),
                ctx.clone(),
                0,
            )),
        },
        2.. => Ok(Expression::from(args.to_vec())),
    }
}
// 遍历操作函数
fn map(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("map", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let func = &args[1];

    let (var_name, ind_name, body) = if check_fn_arg(&func, 2, ctx).is_ok() {
        match &func {
            Expression::Function(_, p, _, body, _) => (p[1].0.clone(), Some(p[0].0.clone()), body),
            Expression::Lambda(p, body, _) => (p[1].clone(), Some(p[0].clone()), body),
            _ => unreachable!(),
        }
    } else if check_fn_arg(&func, 1, ctx).is_ok() {
        match &func {
            Expression::Function(_, p, _, body, _) => (p[0].0.clone(), None, body),
            Expression::Lambda(p, body, _) => (p[0].clone(), None, body),
            _ => unreachable!(),
        }
    } else {
        return Err(RuntimeError::common(
            ("your func/lambda should define 1..2 param").into(),
            ctx.clone(),
            0,
        ));
    };

    // if !need_index {
    let mut state = State::new(is_strict(env));
    state.set(State::IN_ASSIGN);
    let count = list.iter().count();
    let iterator = list.as_ref().clone().into_iter();
    execute_iteration(
        var_name,
        ind_name,
        iterator,
        count,
        body.as_ref(),
        &mut state,
        env,
        0,
    )
}

fn items(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("items", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let items = list
        .as_ref()
        .iter()
        .enumerate()
        .map(|(i, v)| Expression::from(vec![(i as Int).into(), v.clone()]))
        .collect();
    Ok(Expression::List(Rc::new(items)))
}

fn filter(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filter", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let mut result = Vec::new();
    let fn_arg_count = match args[1].clone() {
        Expression::Lambda(params, ..) => params.len(),
        Expression::Function(_, params, _, _, _) => params.len(),
        _ => {
            return Err(RuntimeError::common(
                "expected a func/lambda as filter-function".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let cond = Rc::new(args[1].clone());
    match fn_arg_count {
        1 => {
            for item in list.as_ref() {
                if let Expression::Boolean(true) =
                    Expression::Apply(Rc::clone(&cond), Rc::new(vec![item.clone()])).eval(env)?
                {
                    result.push(item.clone());
                }
            }
        }
        2 => {
            for (i, item) in list.as_ref().iter().enumerate() {
                if let Expression::Boolean(true) = Expression::Apply(
                    Rc::clone(&cond),
                    Rc::new(vec![Expression::Integer(i as i64), item.clone()]),
                )
                .eval(env)?
                {
                    result.push(item.clone());
                }
            }
        }
        _ => {
            return Err(RuntimeError::common(
                "expected 1..2 params for filter-function".into(),
                ctx.clone(),
                0,
            ));
        }
    }

    Ok(Expression::List(Rc::new(result)))
}

fn filter_map(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filter_map", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let func = &args[1];
    check_fn_arg(&func, 1, ctx)?;

    let mut result = Vec::new();
    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::None => continue,
            val => result.push(val),
        }
    }
    Ok(Expression::List(Rc::new(result)))
}

// fn reduce(
//     args: &[Expression],
//     env: &mut Environment,
//     ctx: &Expression,
// ) -> Result<Expression, RuntimeError> {
//     if args.len() < 3 {
//         Ok(Expression::Apply(
//             Rc::new(
//                 crate::parse("(f,acc,list) -> { for item in list { let acc = f acc item } acc }")
//                     .map_err(|e| RuntimeError::common(format!("{}", e).into(), ctx.clone(), 0))?,
//             ),
//             Rc::new(args.to_vec()),
//         )
//         .eval(env)?)
//     } else {
//         check_exact_args_len("reduce", args, 3, ctx)?;
//         let list = get_list_arg(args[0].eval(env)?, ctx)?;
//         let f = args[1].eval(env)?;
//         let mut acc = args[2].eval(env)?;

//         for item in list.as_ref().iter() {
//             acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()]))
//                 .eval(env)?;
//         }
//         Ok(acc)
//     }
// }

fn any(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("any", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let func = &args[1];
    check_fn_arg(&func, 1, ctx)?;

    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::Boolean(true) => return Ok(Expression::Boolean(true)),
            _ => continue,
        }
    }

    Ok(Expression::Boolean(false))
}

fn all(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("all", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let func = &args[1];
    check_fn_arg(&func, 1, ctx)?;

    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::Boolean(false) => return Ok(Expression::Boolean(false)),
            _ => continue,
        }
    }

    Ok(Expression::Boolean(true))
}
// 转换操作函数
fn join(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("join", args, 2, ctx)?;

    let list = get_list_ref(&args[0], ctx)?;
    let separator = get_string_ref(&args[1], ctx)?;

    let mut joined = String::new();
    for (i, item) in list.as_ref().iter().enumerate() {
        if i != 0 {
            joined.push_str(&separator);
        }
        joined.push_str(&item.to_string());
    }
    Ok(Expression::String(joined))
}

fn to_map(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("to_map", args, 1..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let (key_func, val_func) = match args.len() {
        3 => (Some(args[1].clone()), Some(args[2].clone())),
        2 => (Some(args[1].clone()), None),
        _ => {
            let mut map = BTreeMap::new();
            for i in (0..list.len()).step_by(2) {
                map.insert(list[i].to_string(), list[i + 1].clone());
            }
            return Ok(Expression::from(map));
        }
    };

    let mut map = BTreeMap::new();
    for item in list.as_ref().iter() {
        let key = match key_func {
            Some(ref kf) => {
                match Expression::Apply(Rc::new(kf.clone()), Rc::new(vec![item.clone()]))
                    .eval(env)?
                {
                    Expression::String(s) => s,
                    other => other.to_string(),
                }
            }
            None => item.to_string(),
        };
        let value = match val_func {
            Some(ref vf) => {
                Expression::Apply(Rc::new(vf.clone()), Rc::new(vec![item.clone()])).eval(env)?
            }
            None => item.clone(),
        };
        map.insert(key, value);
    }
    Ok(Expression::from(map))
}
// 结构操作函数
fn transpose(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("transpose", args, 1, ctx)?;
    let matrix = get_list_ref(&args[0], ctx)?;

    if matrix.as_ref().is_empty() {
        return Ok(Expression::List(Rc::new(vec![])));
    }

    let row_len = match matrix.as_ref().first() {
        Some(Expression::List(row)) => row.as_ref().len(),
        _ => {
            return Err(RuntimeError::common(
                "transpose requires list of lists as argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    for row in matrix.as_ref().iter() {
        if let Expression::List(r) = row {
            if r.as_ref().len() != row_len {
                return Err(RuntimeError::common(
                    "all rows must have the same length".into(),
                    ctx.clone(),
                    0,
                ));
            }
        } else {
            return Err(RuntimeError::common(
                "transpose requires list of lists as argument".into(),
                ctx.clone(),
                0,
            ));
        }
    }

    // Perform transpose
    let mut transposed = Vec::with_capacity(row_len);
    for i in 0..row_len {
        let mut new_row = Vec::with_capacity(matrix.as_ref().len());
        for row in matrix.as_ref().iter() {
            if let Expression::List(r) = row {
                new_row.push(r.as_ref()[i].clone());
            }
        }
        transposed.push(Expression::List(Rc::new(new_row)));
    }

    Ok(Expression::List(Rc::new(transposed)))
}

fn chunk(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("chunk", args, 2, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let n = get_integer_ref(&args[1], ctx)?;

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

fn foldl(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("foldl", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let f = &args[1];
    let mut acc = args.get(2).map_or(Expression::Integer(0), |x| x.clone());

    check_fn_arg(&f, 2, ctx)?;
    for item in list.as_ref().iter() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone(), acc])).eval(env)?;
    }
    // let mut state = State::new(is_strict(env));
    // match f {
    //     Expression::Function(_, params, _, body, _) => {
    //         let count = list.iter().count();
    //         let iterator = list.as_ref().clone().into_iter();
    //         return execute_iteration(
    //             &params[0].0,
    //             iterator,
    //             count,
    //             body.as_ref(),
    //             &mut state,
    //             env,
    //             0,
    //         );
    //     }
    //     _ => {}
    // }
    Ok(acc)
}
fn foldr(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("foldr", args, 2..=3, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;
    let f = &args[1];
    let mut acc = args.get(2).map_or(Expression::Integer(0), |x| x.clone());

    check_fn_arg(&f, 2, ctx)?;
    for item in list.as_ref().iter().rev() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone(), acc])).eval(env)?;
    }
    Ok(acc)
}

fn zip(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("zip", args, 2, ctx)?;
    let list1 = get_list_ref(&args[0], ctx)?;
    let list2 = get_list_ref(&args[1], ctx)?;

    let mut result = Vec::with_capacity(list1.as_ref().len().min(list2.as_ref().len()));
    for (item1, item2) in list1.as_ref().iter().zip(list2.as_ref().iter()) {
        result.push(Expression::List(Rc::new(vec![
            item1.clone(),
            item2.clone(),
        ])));
    }
    Ok(Expression::List(Rc::new(result)))
}

fn unzip(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("unzip", args, 1, ctx)?;
    let list = get_list_ref(&args[0], ctx)?;

    let mut list1 = Vec::with_capacity(list.as_ref().len());
    let mut list2 = Vec::with_capacity(list.as_ref().len());

    for item in list.as_ref().iter() {
        if let Expression::List(pair) = item {
            if pair.as_ref().len() != 2 {
                return Err(RuntimeError::common(
                    "unzip requires list of pairs".into(),
                    ctx.clone(),
                    0,
                ));
            }
            list1.push(pair.as_ref()[0].clone());
            list2.push(pair.as_ref()[1].clone());
        } else {
            return Err(RuntimeError::common(
                "unzip requires list of pairs".into(),
                ctx.clone(),
                0,
            ));
        }
    }

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(list1)),
        Expression::List(Rc::new(list2)),
    ])))
}

// fn get_list_arg(expr: Expression, ctx: &Expression) -> Result<Rc<Vec<Expression>>, RuntimeError> {
//     match expr {
//         Expression::List(s) => Ok(s),
//         Expression::Range(r, step) => Ok(Rc::new(
//             r.step_by(step).map(Expression::Integer).collect::<Vec<_>>(),
//         )),
//         e => Err(RuntimeError::new(
//             RuntimeErrorKind::TypeError {
//                 expected: "List".to_string(),
//                 found: e.type_name(),
//                 sym: e.to_string(),
//             },
//             ctx.clone(),
//             0,
//         )),
//     }
// }
fn get_list_ref<'a>(
    expr: &'a Expression,
    ctx: &Expression,
) -> Result<&'a Rc<Vec<Expression>>, RuntimeError> {
    match expr {
        Expression::List(s) => Ok(s),
        // Expression::Range(r, step) => Ok(Rc::new(
        //     r.step_by(*step)
        //         .map(Expression::Integer)
        //         .collect::<Vec<_>>(),
        // )),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "List".to_string(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}
