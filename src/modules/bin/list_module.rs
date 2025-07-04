use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::rc::Rc;

pub fn get() -> Expression {
    (hash_map! {
        // 读取操作
               String::from("first") => Expression::builtin("first", first, "get the first element of a list", "<list>"),
               String::from("last") => Expression::builtin("last", last, "get the last element of a list", "<list>"),
               String::from("nth") => Expression::builtin("nth", nth, "get the nth element of a list", "<index> <list>"),
               String::from("take") => Expression::builtin("take", take, "take the first n elements of a list", "<count> <list>"),
               String::from("drop") => Expression::builtin("drop", drop, "drop the first n elements of a list", "<count> <list>"),

               // 修改操作
               String::from("append") => Expression::builtin("append", append, "append an element to a list", "<element> <list>"),
               String::from("prepend") => Expression::builtin("prepend", prepend, "prepend an element to a list", "<element> <list>"),
               String::from("unique") => Expression::builtin("unique", unique, "remove duplicates from a list while preserving order", "<list>"),
               String::from("split_at") => Expression::builtin("split_at", split_at, "split a list at a given index", "<index> <list>"),
               String::from("sort") => Expression::builtin("sort", sort, "sort a string/list, optionally with a key function or key_list", "[key_fn|key_list|keys...] <string|list>"),
               String::from("group") => Expression::builtin("group", group_by, "group list elements by key function", "<key_fn|key> <list>"),

               // 创建操作
               String::from("concat") => Expression::builtin("concat", concat, "concatenate multiple lists into one", "<list1|item1> <list2|item2> ..."),
               String::from("from") => Expression::builtin("from", from, "create a list from a range", "<range|item...>"),

               // 遍历操作
               String::from("emulate") => Expression::builtin("emulate", emulate, "iterate over index-value pairs", "<list>"),
               String::from("map") => Expression::builtin("map", map, "apply function to each element", "<fn> <list>"),
               String::from("filter") => Expression::builtin("filter", filter, "filter elements by condition", "<fn> <list>"),
               String::from("filter_map") => Expression::builtin("filter_map", filter_map, "filter and map in one pass", "<fn> <list>"),
               String::from("reduce") => Expression::builtin("reduce", reduce, "reduce list with accumulator function", "<fn> <init> <list>"),
               String::from("find") => Expression::builtin("find", find, "find index of matching element", "<item> <list>"),

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

fn concat(args: &Vec<Expression>, _env: &mut Environment) -> Result<Expression, LmError> {
    Ok(Expression::List(Rc::new(args.clone())))
}

fn last(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("last", args, 1)?;
    let list = match args[0].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "last requires a list as argument".to_string(),
            ));
        }
    };

    list.as_ref()
        .last()
        .cloned()
        .ok_or_else(|| LmError::CustomError("cannot get last of empty list".to_string()))
}

fn first(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("first", args, 1)?;
    let list = match args[0].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "first requires a list as argument".to_string(),
            ));
        }
    };

    list.as_ref()
        .first()
        .cloned()
        .ok_or_else(|| LmError::CustomError("cannot get first of empty list".to_string()))
}

fn chunk(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("chunk", args, 2)?;
    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "chunk requires integer as first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "chunk requires list as second argument".to_string(),
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

fn prepend(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cons", args, 2)?;
    let head = args[0].eval(env)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "cons requires list as second argument".to_string(),
            ));
        }
    };

    let mut new_list = Vec::with_capacity(list.as_ref().len() + 1);
    new_list.push(head);
    new_list.extend(list.as_ref().iter().cloned());
    Ok(Expression::List(Rc::new(new_list)))
}

fn append(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("append", args, 2)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "append requires list as last argument".to_string(),
            ));
        }
    };

    let item = args[0].eval(env)?;
    let mut new_list = list.as_ref().to_vec();
    new_list.push(item);
    Ok(Expression::List(Rc::new(new_list)))
}

fn from(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
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
        2.. => Ok(Expression::from(args.clone())),
    }
}

fn foldl(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("foldl", args, 3)?;
    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = match args[2].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "foldl requires list as third argument".to_string(),
            ));
        }
    };

    for item in list.as_ref().iter() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![acc, item.clone()])).eval(env)?;
    }
    Ok(acc)
}

fn foldr(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("foldr", args, 3)?;
    let f = args[0].eval(env)?;
    let mut acc = args[1].eval(env)?;
    let list = match args[2].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "foldr requires list as third argument".to_string(),
            ));
        }
    };

    for item in list.as_ref().iter().rev() {
        acc = Expression::Apply(Rc::new(f.clone()), Rc::new(vec![item.clone(), acc])).eval(env)?;
    }
    Ok(acc)
}

fn zip(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
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

fn unzip(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("unzip", args, 1)?;
    let list = match args[0].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "unzip requires list as argument".to_string(),
            ));
        }
    };

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

fn take(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("take", args, 2)?;
    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "take requires integer as first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "take requires list as second argument".to_string(),
            ));
        }
    };

    let taken = list.as_ref().iter().take(n as usize).cloned().collect();
    Ok(Expression::List(Rc::new(taken)))
}

fn drop(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("drop", args, 2)?;
    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "drop requires integer as first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "drop requires list as second argument".to_string(),
            ));
        }
    };

    let dropped = list.as_ref().iter().skip(n as usize).cloned().collect();
    Ok(Expression::List(Rc::new(dropped)))
}

fn split_at(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("split_at", args, 2)?;
    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "split_at requires integer as first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "split_at requires list as second argument".to_string(),
            ));
        }
    };

    let taken = list.as_ref().iter().take(n as usize).cloned().collect();
    let dropped = list.as_ref().iter().skip(n as usize).cloned().collect();

    Ok(Expression::List(Rc::new(vec![
        Expression::List(Rc::new(taken)),
        Expression::List(Rc::new(dropped)),
    ])))
}

fn nth(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("nth", args, 2)?;
    let n = match args[0].eval(env)? {
        Expression::Integer(n) => n,
        _ => {
            return Err(LmError::CustomError(
                "nth requires integer as first argument".to_string(),
            ));
        }
    };

    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "nth requires list as second argument".to_string(),
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

fn map(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() == 1 {
        Ok(Expression::Apply(
            Rc::new(crate::parse("(f,list) -> for item in list {f item}")?),
            Rc::new(args.clone()),
        )
        .eval(env)?)
    } else {
        super::check_exact_args_len("map", args, 2)?;
        let f = args[0].eval(env)?;
        let list = match args[1].eval(env)? {
            Expression::List(l) => l,
            _ => {
                return Err(LmError::CustomError(
                    "map requires list as second argument".to_string(),
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
fn filter(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("filter", args, 2)?;
    let data = if let Expression::List(list) = args[1].eval(env)? {
        list
    } else {
        return Err(LmError::CustomError("Expected list for filtering".into()));
    };

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

fn reduce(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() < 3 {
        Ok(Expression::Apply(
            Rc::new(crate::parse(
                "(f,acc,list) -> { for item in list { let acc = f acc item } acc }",
            )?),
            Rc::new(args.clone()),
        )
        .eval(env)?)
    } else {
        super::check_exact_args_len("reduce", args, 3)?;
        let f = args[0].eval(env)?;
        let mut acc = args[1].eval(env)?;
        let list = match args[2].eval(env)? {
            Expression::List(l) => l,
            _ => {
                return Err(LmError::CustomError(
                    "reduce requires list as third argument".to_string(),
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

fn find(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("find", args, 2)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "find requires list as last argument".to_string(),
            ));
        }
    };
    let target = args[0].eval(env)?;

    Ok(match list.as_ref().iter().position(|x| *x == target) {
        Some(index) => Expression::Integer(index as Int),
        None => Expression::None,
    })
}

fn group_by(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("group", args, 2)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "group requires list as last argument".to_string(),
            ));
        }
    };

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
                        "no such key found in map: `{}`",
                        k
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

fn filter_map(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("filter_map", args, 2)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "filter_map requires list as last argument".to_string(),
            ));
        }
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
}

fn sort(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
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

fn unique(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("unique", args, 1)?;
    let list = match args[0].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "unique requires list as argument".to_string(),
            ));
        }
    };

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for item in list.as_ref().iter() {
        if seen.insert(item.to_string()) {
            result.push(item.clone());
        }
    }
    Ok(Expression::List(Rc::new(result)))
}

fn to_map(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("to-map", args, 1..=3)?;
    let list = match args.last().unwrap().eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "to-map requires list as last argument".to_string(),
            ));
        }
    };

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

fn transpose(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("transpose", args, 1)?;
    let matrix = match args[0].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "transpose requires list of lists as argument".to_string(),
            ));
        }
    };

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

fn join(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
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

fn emulate(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("emulate", args, 1)?;
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

fn get_string_arg(expr: Expression) -> Result<String, LmError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        _ => Err(LmError::CustomError("expected string".to_string())),
    }
}
