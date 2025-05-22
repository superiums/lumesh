use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;
use std::collections::BTreeMap;
use std::rc::Rc;

pub fn get() -> Expression {
    (hash_map! {
        // read
        String::from("first") => Expression::builtin("first", first, "get the first of a list"),
        String::from("last") => Expression::builtin("last", last, "get the last of a list"),
        String::from("nth") => Expression::builtin("nth", nth, "get the nth element of a list"),
        String::from("take") => Expression::builtin("take", take, "take the first n elements of a list"),
        String::from("drop") => Expression::builtin("drop", drop, "drop the first n elements of a list"),

        // modify
        String::from("append") => Expression::builtin("append", append, "append an element to a list"),
        String::from("prepend") => Expression::builtin("prepend", prepend, "prepend an element to a list"),
        String::from("sort") => Expression::builtin("sort", sort, "sort a list, optionally with a key function"),
        String::from("unique") => Expression::builtin("unique", unique, "remove duplicates from a list while preserving order"),
        String::from("split-at") => Expression::builtin("split-at", split_at, "split a list at a given index"),

        // create
        String::from("concat") => Expression::builtin("concat", concat, "create a list from a variable number of arguments"),
        String::from("range") => Expression::builtin("range", range, "create a list from range"),

        // loop, walk on
        String::from("emulate") => Expression::builtin("emulate", emulate, "emulate over a list of index and values"),
        String::from("map") => Expression::builtin("map", map, "map a function over a list of values"),
        String::from("filter") => Expression::builtin("filter", filter, "filter a list of values with a condition function"),
        String::from("filter_map") => Expression::builtin("filter_map", filter_map, "filter and map list elements in one pass, skipping None values"),
        String::from("reduce") => Expression::builtin("reduce", reduce, "reduce a function over a list of values"),
        String::from("find") => Expression::builtin("find", find, "find the index of an element in a list, returns None if not found"),

        // transfer to
        String::from("join") => Expression::builtin("join", join, "join a list of strings with a separator"),
        String::from("to-map") => Expression::builtin("to-map", to_map, "convert list to map using a key function (default: use items themselves as keys)"),

        // flatten
        // transpose
        String::from("transpose") => Expression::builtin("transpose", transpose, "transpose a matrix (list of lists) by switching rows and columns"),
        String::from("group-by") => Expression::builtin("group-by", group_by, "group list elements by key function, returns list of [key, elements] pairs"),
        String::from("chunk") => Expression::builtin("chunk", chunk, "chunk a list into lists of n elements"),
        String::from("foldl") => Expression::builtin("foldl", foldl, "fold a list from the left"),
        String::from("foldr") => Expression::builtin("foldr", foldr, "fold a list from the right"),
        String::from("zip") => Expression::builtin("zip", zip, "zip two lists together"),
        String::from("unzip") => Expression::builtin("unzip", unzip, "unzip a list of pairs into a pair of lists"),
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

fn range(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("range", args, 1)?;
    match args[0].eval(env)? {
        Expression::Range(r) => Ok(Expression::from(r.collect::<Vec<Int>>())),
        _ => Err(LmError::CustomError(
            "range requires a range (a..b) as arguments".to_string(),
        )),
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
    if args.len() == 1 {
        Ok(Expression::Apply(
            Rc::new(crate::parse(
                r#"(f,list) -> { let result = []; for item in list { if (f item) { let result = result + [item]; } } result }"#,
            )?),
            Rc::new(args.clone()),
        ).eval(env)?)
    } else {
        super::check_exact_args_len("filter", args, 2)?;
        let f = args[0].eval(env)?;
        let list = match args[1].eval(env)? {
            Expression::List(l) => l,
            _ => {
                return Err(LmError::CustomError(
                    "filter requires list as second argument".to_string(),
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
    super::check_exact_args_len("group_by", args, 2)?;
    let list = match args[1].eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "group_by requires list as last argument".to_string(),
            ));
        }
    };

    let key_func = args[0].eval(env)?;
    let mut groups: BTreeMap<String, Vec<Expression>> = BTreeMap::new();

    for item in list.as_ref().iter() {
        let key = match Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![item.clone()]))
            .eval(env)?
        {
            Expression::String(s) => s,
            other => other.to_string(),
        };
        groups.entry(key).or_default().push(item.clone());
    }

    let result = groups
        .into_iter()
        .map(|(k, v)| Expression::from(vec![Expression::String(k), Expression::from(v)]))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(result))
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
    super::check_args_len("sort", args, 1..2)?;
    let list = match args.last().unwrap().eval(env)? {
        Expression::List(l) => l,
        s => {
            return Err(LmError::CustomError(format!(
                "sort requires list as last argument, found {}",
                s.type_name()
            )));
        }
    };
    // dbg!(&list);
    let mut sorted = list.as_ref().clone();
    // dbg!(&sorted);

    if args.len() == 2 {
        let key_func = args[0].eval(env)?;
        sorted.sort_by(|a, b| {
            let key_a = Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![a.clone()]))
                .eval(env)
                .unwrap_or(Expression::None);
            let key_b = Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![b.clone()]))
                .eval(env)
                .unwrap_or(Expression::None);
            key_a
                .partial_cmp(&key_b)
                .unwrap_or(std::cmp::Ordering::Equal)
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
    super::check_args_len("to-map", args, 1..2)?;
    let list = match args.last().unwrap().eval(env)? {
        Expression::List(l) => l,
        _ => {
            return Err(LmError::CustomError(
                "to-map requires list as last argument".to_string(),
            ));
        }
    };

    let key_func = if args.len() == 2 {
        args[0].eval(env)?
    } else {
        Expression::builtin("_id", |args, _| Ok(args[0].clone()), "identity function")
    };

    let mut map = BTreeMap::new();
    for item in list.as_ref().iter() {
        let key = match Expression::Apply(Rc::new(key_func.clone()), Rc::new(vec![item.clone()]))
            .eval(env)?
        {
            Expression::String(s) => s,
            other => other.to_string(),
        };
        map.insert(key, item.clone());
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
