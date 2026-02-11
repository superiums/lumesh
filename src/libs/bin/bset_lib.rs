use crate::eval::State;
use crate::libs::BuiltinInfo;
use crate::libs::helper::*;
use crate::libs::lazy_module::LazyModule;
use crate::{Environment, Expression, RuntimeError, RuntimeErrorKind};
use crate::{reg_info, reg_lazy};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::rc::Rc;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 检查操作
        contains, is_empty,
        // 数据获取
        first,last,items, len,
        // 查找
        find, filter,
        // 结构修改
        add, remove,
        // 创建操作
        from_items,
        // 集合运算
        union, intersect, difference, is_subset, is_superset,
        // 转换操作
        map, to_list,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // 检查操作
        contains => "check if set contains item", "<set> <item>"
        is_empty => "check if set is empty", "<set>"

        // 数据获取
        first => "get first item of set", "<set>"
        last => "get last item of set", "<set>"
        items => "get all items from set", "<set>"
        len => "get size of set", "<set>"

        // 查找
        find => "find first item matching condition", "<set> <predicate_fn>"
        filter => "filter set by condition", "<set> <predicate_fn>"

        // 结构修改
        add => "add item to set", "<set> <item>"
        remove => "remove item from set", "<set> <item>"

        // 创建操作
        from_items => "create set from list", "<items>"

        // 集合运算
        union => "union of two sets", "<set1> <set2>"
        intersect => "intersection of two sets", "<set1> <set2>"
        difference => "difference of two sets", "<set1> <set2>"
        is_subset => "check if set1 is subset of set2", "<set1> <set2>"
        is_superset => "check if set1 is superset of set2", "<set1> <set2>"

        // 转换操作
        map => "apply function to each item", "<set> <fn>"
        to_list => "convert set to list", "<set>"
    })
}

// 检查操作函数
fn contains(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("contains", &args, 2, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;
    let item = &args[1];

    Ok(Expression::Boolean(set.contains(item)))
}

fn is_empty(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_empty", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    Ok(Expression::Boolean(set.is_empty()))
}

// 数据获取函数
fn first(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("first", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    set.as_ref()
        .first()
        .cloned()
        .ok_or_else(|| RuntimeError::common("cannot get first of empty set".into(), ctx.clone(), 0))
}

fn last(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("last", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    set.as_ref()
        .last()
        .cloned()
        .ok_or_else(|| RuntimeError::common("cannot get last of empty set".into(), ctx.clone(), 0))
}

fn items(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("items", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    let items = set.iter().cloned().collect::<Vec<_>>();
    Ok(Expression::from(items))
}

fn len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    Ok(Expression::Integer(set.len() as i64))
}

// 查找函数
fn find(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("find", &args, 2, ctx)?;

    let predicate = &args[1];
    check_fn_arg(&predicate, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    let mut state = State::new();
    for item in set.iter() {
        if predicate
            .eval_apply(predicate, &vec![item.clone()], &mut state, env, 0)?
            .is_truthy()
        {
            return Ok(item.clone());
        }
    }

    Ok(Expression::None)
}

fn filter(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filter", &args, 2, ctx)?;

    let predicate = &args[1];
    check_fn_arg(&predicate, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    let mut new_set = BTreeSet::new();
    let mut state = State::new();
    for item in set.iter() {
        if predicate
            .eval_apply(predicate, &vec![item.clone()], &mut state, env, 0)?
            .is_truthy()
        {
            new_set.insert(item.clone());
        }
    }

    Ok(Expression::from(new_set))
}

// 结构修改函数
fn add(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("add", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let set = into_bset(it.next().unwrap(), ctx)?;
    let item = it.next().unwrap();

    let mut new_set = set.as_ref().clone();
    new_set.insert(item);
    Ok(Expression::BSet(Rc::new(new_set)))
}

fn remove(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let set = into_bset(it.next().unwrap(), ctx)?;
    let item = it.next().unwrap();

    let mut new_set = set.as_ref().clone();
    new_set.remove(&item);
    Ok(Expression::BSet(Rc::new(new_set)))
}

// 创建操作函数
fn from_items(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("from_items", &args, 1, ctx)?;
    let expr = &args[0];

    if let Expression::List(list) = expr {
        let mut set = BTreeSet::new();
        for item in list.as_ref() {
            set.insert(item.clone());
        }
        Ok(Expression::from(set))
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "List".to_string(),
                found: expr.type_name(),
                sym: expr.to_string(),
            },
            ctx.clone(),
            0,
        ))
    }
}

// 集合运算函数
fn union(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("union", &args, 2, ctx)?;
    let set1 = get_bset_ref(&args[0], ctx)?;
    let set2 = get_bset_ref(&args[1], ctx)?;

    let mut new_set = set1.as_ref().clone();
    new_set.extend(set2.iter().cloned());
    Ok(Expression::BSet(Rc::new(new_set)))
}

fn intersect(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("intersect", &args, 2, ctx)?;
    let set1 = get_bset_ref(&args[0], ctx)?;
    let set2 = get_bset_ref(&args[1], ctx)?;

    let new_set = set1.intersection(set2).cloned().collect::<BTreeSet<_>>();
    Ok(Expression::from(new_set))
}

fn difference(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("difference", &args, 2, ctx)?;
    let set1 = get_bset_ref(&args[0], ctx)?;
    let set2 = get_bset_ref(&args[1], ctx)?;

    let new_set = set1.difference(set2).cloned().collect::<BTreeSet<_>>();
    Ok(Expression::from(new_set))
}

fn is_subset(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_subset", &args, 2, ctx)?;
    let set1 = get_bset_ref(&args[0], ctx)?;
    let set2 = get_bset_ref(&args[1], ctx)?;

    Ok(Expression::Boolean(set1.is_subset(set2)))
}

fn is_superset(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_superset", &args, 2, ctx)?;
    let set1 = get_bset_ref(&args[0], ctx)?;
    let set2 = get_bset_ref(&args[1], ctx)?;

    Ok(Expression::Boolean(set1.is_superset(set2)))
}

// 转换操作函数
fn map(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("map", &args, 2, ctx)?;

    let func = &args[1];
    check_fn_arg(&func, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    let mut new_set = BTreeSet::new();
    let mut state = State::new();
    for item in set.iter() {
        let new_item = func.eval_apply(func, &vec![item.clone()], &mut state, env, 0)?;
        new_set.insert(new_item);
    }

    Ok(Expression::from(new_set))
}

fn to_list(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_list", &args, 1, ctx)?;
    let set = get_bset_ref(&args[0], ctx)?;

    let list = set.iter().cloned().collect::<Vec<_>>();
    Ok(Expression::from(list))
}

// 辅助函数
fn get_bset_ref<'a>(
    expr: &'a Expression,
    ctx: &Expression,
) -> Result<&'a Rc<BTreeSet<Expression>>, RuntimeError> {
    match expr {
        Expression::BSet(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "BSet".to_string(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}

fn into_bset(expr: Expression, ctx: &Expression) -> Result<Rc<BTreeSet<Expression>>, RuntimeError> {
    match expr {
        Expression::BSet(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "BSet".to_string(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}
