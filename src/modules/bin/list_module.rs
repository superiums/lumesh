use super::get_string_arg;
use super::math_module::{average, max, min, sum};
use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::rc::Rc;

pub fn get() -> Expression {
    (hash_map! {
        // 打印
        String::from("pprint") => Expression::builtin("pprint", super::pretty_print, "pretty print", "<list>"),

        // 数学统计
        String::from("max") => Expression::builtin("max", max, "get max value in an array or multi args", "<num1> <num2> ... | <array>"),
        String::from("min") => Expression::builtin("min", min, "get min value in an array or multi args", "<num1> <num2> ... | <array>"),
        String::from("sum") => Expression::builtin("sum", sum, "sum a list of numbers", "<num1> <num2> ... | <array>"),
        String::from("average") => Expression::builtin("average", average, "get the average of a list of numbers", "<num1> <num2> ... | <array>"),

        // 读取操作
        String::from("get") => Expression::builtin("get", super::get, "get value from nested map/list/range using dot notation path", "<path> <map|list|range>"),
        String::from("len") => Expression::builtin("len", super::len, "get length of list", "<list>"),
        String::from("insert") => Expression::builtin("insert", super::insert, "insert item into list", "<index> <value> <list>"),
        String::from("rev") => Expression::builtin("rev", super::rev, "reverse sequence", "<list>"),
        String::from("flatten") => Expression::builtin("flatten", super::flatten_wrapper, "flatten nested structure", "<collection>"),

        String::from("first") => Expression::builtin("first", first, "get the first element of a list", "<list>"),
        String::from("last") => Expression::builtin("last", last, "get the last element of a list", "<list>"),
        String::from("at") => Expression::builtin("at", at, "get the nth element of a list", "<index> <list>"),
        String::from("take") => Expression::builtin("take", take, "take the first n elements of a list", "<count> <list>"),
        String::from("drop") => Expression::builtin("drop", drop, "drop the first n elements of a list", "<count> <list>"),
        // 查找操作
        String::from("contains") => Expression::builtin("contains", contains, "check if list contains an item", "<item> <list>"),
        String::from("find") => Expression::builtin("find", find_index, "find first index of matching element", "<item|fn> [start_index] <list>"),
        String::from("find_last") => Expression::builtin("find_last", find_last_index, "find last index of item", "<item|fn> [start_index] <list>"),

        // 修改操作
        String::from("append") => Expression::builtin("append", append, "append an element to a list", "<element> <list>"),
        String::from("prepend") => Expression::builtin("prepend", prepend, "prepend an element to a list", "<element> <list>"),
        String::from("unique") => Expression::builtin("unique", unique, "remove duplicates from a list while preserving order", "<list>"),
        String::from("split_at") => Expression::builtin("split_at", split_at, "split a list at a given index", "<index> <list>"),
        // String::from("splice") => Expression::builtin("splice", splice, "change contents by removing/adding elements", "<start> <deleteCount> [items...] <list>"),
        String::from("sort") => Expression::builtin("sort", sort, "sort a string/list, optionally with a key function or key_list", "[key_fn|key_list|keys...] <string|list>"),
        String::from("group") => Expression::builtin("group", group_by, "group list elements by key function", "<key_fn|key> <list>"),
        String::from("remove_at") => Expression::builtin("remove_at", remove_at, "remove n elements starting from index", "<index> [count] <list>"),
        String::from("remove") => Expression::builtin("remove", remove, "remove first matching element", "<item> [all?] <list>"),
        String::from("set") => Expression::builtin("set", set_list, "set element at existing index", "<index> <value> <list>"),
        // 创建操作
        String::from("concat") => Expression::builtin("concat", concat, "concatenate multiple lists into one", "<list1|item1> <list2|item2> ..."),
        String::from("from") => Expression::builtin("from", from, "create a list from a range", "<range|item...>"),

        // 遍历操作
        String::from("each") => Expression::builtin("each", for_each, "execute function for each element", "<fn> <list>"),
        String::from("items") => Expression::builtin("items", entries, "iterate over index-value pairs", "<list>"),
        String::from("map") => Expression::builtin("map", map, "apply function to each element", "<fn> <list>"),
        String::from("filter") => Expression::builtin("filter", filter, "filter elements by condition", "<fn> <list>"),
        String::from("filter_map") => Expression::builtin("filter_map", filter_map, "filter and map in one pass", "<fn> <list>"),
        String::from("reduce") => Expression::builtin("reduce", reduce, "reduce list with accumulator function", "<fn> <init> <list>"),
        String::from("any") => Expression::builtin("any", any, "test if any element passes condition", "<fn> <list>"),
        String::from("all") => Expression::builtin("all", all, "test if all elements pass condition", "<fn> <list>"),

        // 转换操作
        String::from("join") => Expression::builtin("join", join, "join string list with separator", "<separator> <list>"),
        String::from("to_map") => Expression::builtin("to_map", to_map, "convert list to map using key function", "[key_fn] [val_fn] <list>"),

        // 结构操作
        String::from("transpose") => Expression::builtin("transpose", transpose, "transpose matrix (list of lists)", "<matrix>"),
        String::from("chunk") => Expression::builtin("chunk", chunk, "split list into chunks of size n", "<size> <list>"),
        String::from("foldl") => Expression::builtin("foldl", foldl, "fold list from left with function", "<fn> <init> <list>"),
        String::from("foldr") => Expression::builtin("foldr", foldr, "fold list from right with function", "<fn> <init> <list>"),
        String::from("zip") => Expression::builtin("zip", zip, "zip two lists into list of pairs", "<list1> <list2>"),
        String::from("unzip") => Expression::builtin("unzip", unzip, "unzip list of pairs into two lists", "<list_of_pairs>"),
    })
    .into()
}

fn set_list(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("set", args, 3)?;

    let val = args[1].eval(env)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[2].eval(env)?)?;

    let index = n as usize;
    if index < list.as_ref().len() {
        let mut result = list.as_ref().clone();
        result[index] = val;
        Ok(Expression::from(result))
    } else {
        Err(LmError::CustomError(format!(
            "index {} out of bounds for list of length {}",
            n,
            list.as_ref().len()
        )))
    }
}

fn concat(args: &[Expression], _env: &mut Environment) -> Result<Expression, LmError> {
    Ok(Expression::List(Rc::new(args.to_vec())))
}

fn last(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("last", args, 1)?;
    let list = get_list_arg(args[0].eval(env)?)?;

    list.as_ref()
        .last()
        .cloned()
        .ok_or_else(|| LmError::CustomError("cannot get last of empty list".to_string()))
}

fn first(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("first", args, 1)?;
    let list = get_list_arg(args[0].eval(env)?)?;

    list.as_ref()
        .first()
        .cloned()
        .ok_or_else(|| LmError::CustomError("cannot get first of empty list".to_string()))
}

fn chunk(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("chunk", args, 2)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[1].eval(env)?)?;

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

fn prepend(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cons", args, 2)?;
    let head = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let mut new_list = Vec::with_capacity(list.as_ref().len() + 1);
    new_list.push(head);
    new_list.extend(list.as_ref().iter().cloned());
    Ok(Expression::List(Rc::new(new_list)))
}

fn append(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("append", args, 2)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let item = args[0].eval(env)?;
    let mut new_list = list.as_ref().to_vec();
    new_list.push(item);
    Ok(Expression::List(Rc::new(new_list)))
}

fn from(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => Err(LmError::CustomError(
            "range requires a range (a..b) or some elements as arguments".to_string(),
        )),
        1 => match args[0].eval(env)? {
            Expression::Range(r, step) => {
                Ok(Expression::from(r.step_by(step).collect::<Vec<Int>>()))
            }
            _ => Err(LmError::CustomError(
                "the only arg should be a range (a..b)".to_string(),
            )),
        },
        2.. => Ok(Expression::from(args.to_vec())),
    }
}

fn foldl(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("foldl", args, 3)?;
    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = get_list_arg(args[2].eval(env)?)?;

    for item in list.as_ref().iter() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()])).eval(env)?;
    }
    Ok(acc)
}

fn foldr(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("foldr", args, 3)?;
    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = get_list_arg(args[2].eval(env)?)?;

    for item in list.as_ref().iter().rev() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone(), acc])).eval(env)?;
    }
    Ok(acc)
}

fn zip(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("zip", args, 2)?;
    match (args[0].eval(env)?, args[1].eval(env)?) {
        (Expression::List(list1), Expression::List(list2)) => {
            let mut result = Vec::with_capacity(list1.as_ref().len().min(list2.as_ref().len()));
            for (item1, item2) in list1.as_ref().iter().zip(list2.as_ref().iter()) {
                result.push(Expression::List(Rc::new(vec![
                    item1.clone(),
                    item2.clone(),
                ])));
            }
            Ok(Expression::List(Rc::new(result)))
        }
        _ => Err(LmError::CustomError(
            "zip requires two lists as arguments".to_string(),
        )),
    }
}

fn unzip(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("unzip", args, 1)?;
    let list = get_list_arg(args[0].eval(env)?)?;

    let mut list1 = Vec::with_capacity(list.as_ref().len());
    let mut list2 = Vec::with_capacity(list.as_ref().len());

    for item in list.as_ref().iter() {
        if let Expression::List(pair) = item {
            if pair.as_ref().len() != 2 {
                return Err(LmError::CustomError(
                    "unzip requires list of pairs".to_string(),
                ));
            }
            list1.push(pair.as_ref()[0].clone());
            list2.push(pair.as_ref()[1].clone());
        } else {
            return Err(LmError::CustomError(
                "unzip requires list of pairs".to_string(),
            ));
        }
    }

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(list1)),
        Expression::List(Rc::new(list2)),
    ])))
}

fn take(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("take", args, 2)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let taken = list.as_ref().iter().take(n as usize).cloned().collect();
    Ok(Expression::List(Rc::new(taken)))
}

fn drop(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("drop", args, 2)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let dropped = list.as_ref().iter().skip(n as usize).cloned().collect();
    Ok(Expression::List(Rc::new(dropped)))
}

fn split_at(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("split_at", args, 2)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let taken = list.as_ref().iter().take(n as usize).cloned().collect();
    let dropped = list.as_ref().iter().skip(n as usize).cloned().collect();

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(taken)),
        Expression::List(Rc::new(dropped)),
    ])))
}

fn at(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("at", args, 2)?;
    let n = super::get_integer_arg(args[0].eval(env)?)?;
    let list = get_list_arg(args[1].eval(env)?)?;

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

fn map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("map", args, 2)?;
    let f = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let mut result = Vec::with_capacity(list.as_ref().len());
    for item in list.as_ref().iter() {
        result.push(Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone()])).eval(env)?);
    }
    Ok(Expression::List(Rc::new(result)))
}
fn for_each(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("forEach", args, 2)?;
    let func = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let need_index = match &func {
        Expression::Function(_, p, c, _, _) => p.len() > 1 || c.is_some(),
        Expression::Lambda(p, _) => p.len() > 1,
        o => {
            return Err(LmError::TypeError {
                expected: "Function/Lambda".into(),
                sym: args[0].to_string(),
                found: o.type_name(),
            });
        }
    };
    if need_index {
        for (index, item) in list.as_ref().iter().enumerate() {
            Expression::Apply(
                Rc::new(func.clone()),
                Rc::new(vec![Expression::Integer(index as Int), item.clone()]),
            )
            .eval(env)?;
        }
    } else {
        for item in list.as_ref().iter() {
            Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)?;
        }
    }

    Ok(Expression::None)
}

fn filter(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("filter", args, 2)?;
    let data = get_list_arg(args[1].eval(env)?)?;

    let mut result = Vec::new();
    let fn_arg_count = match args[0].clone() {
        Expression::Lambda(params, _) => params.len(),
        Expression::Function(_, params, _, _, _) => params.len(),
        _ => {
            let mut row_env = env.fork();
            row_env.define("LINES", Expression::Integer(data.len() as i64));
            for (i, item) in data.as_ref().iter().enumerate() {
                row_env.define("LINENO", Expression::Integer(i as i64));
                if let Expression::Boolean(true) = args[0].eval(&mut row_env)? {
                    result.push(item.clone())
                }
            }
            return Ok(Expression::List(Rc::new(result)));
        }
    };

    let cond = Rc::new(args[0].clone());
    match fn_arg_count {
        1 => {
            for item in data.as_ref() {
                if let Expression::Boolean(true) =
                    Expression::Apply(Rc::clone(&cond), Rc::new(vec![item.clone()])).eval(env)?
                {
                    result.push(item.clone());
                }
            }
        }
        2 => {
            for (i, item) in data.as_ref().iter().enumerate() {
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
            return Err(LmError::CustomError(
                "expected 1..2 params for filter-fn".into(),
            ));
        }
    }

    Ok(Expression::List(Rc::new(result)))
}

fn reduce(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() < 3 {
        Ok(Expression::Apply(
            Rc::new(crate::parse(
                "(f,acc,list) -> { for item in list { let acc = f acc item } acc }",
            )?),
            Rc::new(args.to_vec()),
        )
        .eval(env)?)
    } else {
        super::check_exact_args_len("reduce", args, 3)?;
        let f = args[0].eval(env)?;
        let mut acc = args[1].eval(env)?;
        let list = get_list_arg(args[2].eval(env)?)?;

        for item in list.as_ref().iter() {
            acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()]))
                .eval(env)?;
        }
        Ok(acc)
    }
}

fn group_by(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("group", args, 2)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let key_func = args[0].eval(env)?;
    let mut groups: BTreeMap<String, Vec<Expression>> = BTreeMap::new();

    match key_func {
        Expression::Lambda(..) | Expression::Function(..) => {
            let key_f = Rc::new(key_func);
            for item in list.as_ref().iter() {
                let key = match Expression::Apply(Rc::clone(&key_f), Rc::new(vec![item.clone()]))
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
                let keyitem = match item {
                    Expression::Map(m) => m.get(&k),
                    Expression::HMap(m) => m.get(&k),
                    _ => {
                        return Err(LmError::CustomError(
                            "group by key can only apply to a map".to_string(),
                        ));
                    }
                };
                if let Some(key) = keyitem {
                    groups
                        .entry(key.to_string())
                        .or_default()
                        .push(item.clone());
                } else {
                    return Err(LmError::CustomError(format!(
                        "no such key found in map: `{k}`"
                    )));
                }
            }
        }
        _ => {
            return Err(LmError::CustomError(
                "group requires key-func or key".to_string(),
            ));
        }
    };

    // let result = groups
    //     .into_iter()
    //     .map(|(k, v)| Expression::from(vec![Expression::String(k), Expression::from(v)]))
    //     .collect::<Vec<Expression>>();
    Ok(Expression::from(groups))
}

fn filter_map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("filter_map", args, 2)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    let func = args[0].eval(env)?;
    let mut result = Vec::new();

    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::None => continue,
            val => result.push(val),
        }
    }
    Ok(Expression::List(Rc::new(result)))
}

fn sort(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("sort", args, 1..)?;
    let key_func = args[0].eval(env)?;
    let (func, headers) = match args.len() {
        2 => match key_func {
            Expression::Lambda(..) | Expression::Function(..) => (Some(Rc::new(key_func)), None),
            Expression::Symbol(s) | Expression::String(s) => (None, Some(vec![s])),
            Expression::List(s) => (
                None,
                Some(s.iter().map(|e| e.to_string()).collect::<Vec<_>>()),
            ),
            _ => (None, None),
        },
        3.. => {
            let cols = super::get_string_args(&args[..args.len() - 1], env)?;
            (None, Some(cols))
        }

        0..2 => (None, None),
    };

    let mut sorted: Vec<_> = match args.last().unwrap().eval(env)? {
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
            return Err(LmError::TypeError {
                expected: "List as last argument".to_string(),
                found: s.type_name(),
                sym: s.to_string(),
            });
        }
    };

    if let Some(sort_func) = func {
        sorted.sort_by(|a, b| {
            let sort_result =
                Expression::Apply(Rc::clone(&sort_func), Rc::new(vec![a.clone(), b.clone()]))
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
                _ => {
                    Ordering::Equal
                    // return Err(LmError::CustomError(
                    //     "sort func should return 1,0,-1 or boolean".to_string(),
                    // ));
                } // e=> return e?;
            }
        });
    } else if let Some(heads) = headers {
        // sorted.sort_by_key(|item| {
        //     if let Expression::Map(m) = item {
        //         heads
        //             .iter()
        //             .map(|col| m.get(col).unwrap_or(&Expression::None))
        //             .collect::<Vec<_>>()
        //     } else {
        //         vec![]
        //     }
        // });
        sorted.sort_by(|a, b| {
            match (a, b) {
                (Expression::Map(map_a), Expression::Map(map_b)) => {
                    // 提取每个键对应的值，并返回一个元组
                    let key_a = heads
                        .iter()
                        .map(|col| map_a.get(col).unwrap_or(&Expression::None))
                        .collect::<Vec<_>>();
                    let key_b = heads
                        .iter()
                        .map(|col| map_b.get(col).unwrap_or(&Expression::None))
                        .collect::<Vec<_>>();

                    // 使用 PartialOrd 进行比较
                    key_a
                        .iter()
                        .zip(key_b.iter())
                        .find_map(|(a_val, b_val)| match a_val.partial_cmp(b_val) {
                            Some(Ordering::Equal) => None,
                            other => other,
                        })
                        .unwrap_or(Ordering::Equal) // 如果所有值都相等，返回 Equal
                }
                (Expression::HMap(map_a), Expression::HMap(map_b)) => {
                    // 提取每个键对应的值，并返回一个元组
                    let key_a = heads
                        .iter()
                        .map(|col| map_a.get(col).unwrap_or(&Expression::None))
                        .collect::<Vec<_>>();
                    let key_b = heads
                        .iter()
                        .map(|col| map_b.get(col).unwrap_or(&Expression::None))
                        .collect::<Vec<_>>();

                    // 使用 PartialOrd 进行比较
                    key_a
                        .iter()
                        .zip(key_b.iter())
                        .find_map(|(a_val, b_val)| match a_val.partial_cmp(b_val) {
                            Some(Ordering::Equal) => None,
                            other => other,
                        })
                        .unwrap_or(Ordering::Equal) // 如果所有值都相等，返回 Equal
                }
                _ => Ordering::Equal, // 对于其他类型，返回 Equal
            }
        });
    } else {
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    Ok(Expression::List(Rc::new(sorted)))
}

fn unique(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("unique", args, 1)?;
    let list = get_list_arg(args[0].eval(env)?)?;

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for item in list.as_ref().iter() {
        if seen.insert(item.to_string()) {
            result.push(item.clone());
        }
    }
    Ok(Expression::List(Rc::new(result)))
}

fn to_map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("to_map", args, 1..=3)?;
    let list = get_list_arg(args.last().unwrap().eval(env)?)?;

    let (key_func, val_func) = match args.len() {
        3 => (Some(args[0].eval(env)?), Some(args[1].eval(env)?)),
        2 => (Some(args[0].eval(env)?), None),
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

fn transpose(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("transpose", args, 1)?;
    let matrix = get_list_arg(args[0].eval(env)?)?;

    if matrix.as_ref().is_empty() {
        return Ok(Expression::List(Rc::new(vec![])));
    }

    // Verify all rows have same length
    let row_len = match matrix.as_ref().first() {
        Some(Expression::List(row)) => row.as_ref().len(),
        _ => {
            return Err(LmError::CustomError(
                "transpose requires list of lists as argument".to_string(),
            ));
        }
    };

    for row in matrix.as_ref().iter() {
        if let Expression::List(r) = row {
            if r.as_ref().len() != row_len {
                return Err(LmError::CustomError(
                    "all rows must have same length for transpose".to_string(),
                ));
            }
        } else {
            return Err(LmError::CustomError(
                "transpose requires list of lists as argument".to_string(),
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

fn join(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("join", args, 2)?;
    let separator = get_string_arg(args[0].eval(env)?)?;

    match args[1].eval(env)? {
        Expression::List(list) => {
            let mut joined = String::new();
            for (i, item) in list.as_ref().iter().enumerate() {
                if i != 0 {
                    joined.push_str(&separator);
                }
                joined.push_str(&item.to_string());
            }
            Ok(Expression::String(joined))
        }
        _ => Ok(Expression::None),
    }
}

fn entries(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("items", args, 1)?;
    let expr = args[0].eval(env)?;

    Ok(match expr {
        Expression::List(list) => {
            let items = list
                .as_ref()
                .iter()
                .enumerate()
                .map(|(i, v)| Expression::from(vec![(i as Int).into(), v.clone()]))
                .collect();
            Expression::List(Rc::new(items))
        }
        _ => Expression::None,
    })
}

fn contains(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("includes", args, 2)?;
    let item = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    Ok(Expression::Boolean(list.as_ref().contains(&item)))
}

fn any(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("some", args, 2)?;
    let func = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::Boolean(true) => return Ok(Expression::Boolean(true)),
            _ => continue,
        }
    }

    Ok(Expression::Boolean(false))
}

fn all(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("every", args, 2)?;
    let func = args[0].eval(env)?;
    let list = get_list_arg(args[1].eval(env)?)?;

    for item in list.as_ref().iter() {
        match Expression::Apply(Rc::new(func.clone()), Rc::new(vec![item.clone()])).eval(env)? {
            Expression::Boolean(false) => return Ok(Expression::Boolean(false)),
            _ => continue,
        }
    }

    Ok(Expression::Boolean(true))
}

fn find_index(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("findIndex", args, 2..=3)?;
    let target = args[0].eval(env)?;
    let list = get_list_arg(args.last().unwrap().eval(env)?)?;
    let start = if args.len() == 3 {
        super::get_integer_arg(args[1].eval(env)?)? as usize
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
            match list.as_ref().iter().skip(start).position(|x| *x == target) {
                Some(index) => Expression::Integer(index as Int),
                None => Expression::None,
            },
        ),
    }
}

fn find_last_index(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("findIndex", args, 2..=3)?;
    let target = args[0].eval(env)?;
    let list = get_list_arg(args.last().unwrap().eval(env)?)?;
    let start = if args.len() == 3 {
        super::get_integer_arg(args[1].eval(env)?)? as usize
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
                .position(|x| *x == target)
            {
                Some(index) => Expression::Integer(index as Int),
                None => Expression::None,
            },
        ),
    }
}

fn remove_at(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("remove_at", args, 2..=3)?;

    let index = super::get_integer_arg(args[0].eval(env)?)?;

    let count = if args.len() == 3 {
        super::get_integer_arg(args[1].eval(env)?)?
    } else {
        1
    };

    let list = get_list_arg(args.last().unwrap().eval(env)?)?;

    if count <= 0 {
        return Ok(Expression::List(list));
    }

    let list_len = list.as_ref().len() as Int;
    let start_idx = if index < 0 {
        (list_len + index).max(0) as usize
    } else {
        (index as usize).min(list_len as usize)
    };

    let end_idx = (start_idx + count as usize).min(list_len as usize);

    if start_idx >= list_len as usize {
        return Ok(Expression::List(list));
    }

    let mut new_list = Vec::new();
    new_list.extend(list.as_ref().iter().take(start_idx).cloned());
    new_list.extend(list.as_ref().iter().skip(end_idx).cloned());

    Ok(Expression::List(Rc::new(new_list)))
}

fn remove(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("remove", args, 2..=3)?;

    let item = args[0].eval(env)?;

    let all = if args.len() == 3 {
        if let Expression::Boolean(b) = args[1].eval(env)? {
            b
        } else {
            false
        }
    } else {
        false
    };

    let list = get_list_arg(args.last().unwrap().eval(env)?)?;
    if all {
        let new_list = list
            .iter()
            .filter(|x| **x != item)
            .cloned()
            .collect::<Vec<_>>();
        Ok(Expression::from(new_list))
    } else if let Some(pos) = list.iter().position(|x| *x == item) {
        let mut new_list = list.as_ref().clone();
        new_list.remove(pos);
        Ok(Expression::from(new_list))
    } else {
        Ok(Expression::List(list))
    }
}

fn get_list_arg(expr: Expression) -> Result<Rc<Vec<Expression>>, LmError> {
    match expr {
        Expression::List(s) => Ok(s),
        Expression::Range(r, step) => Ok(Rc::new(
            r.step_by(step).map(Expression::Integer).collect::<Vec<_>>(),
        )),
        e => Err(LmError::TypeError {
            expected: "List".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
