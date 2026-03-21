use crate::expression::table::TableData;
use crate::libs::helper::{
    check_args_len, check_exact_args_len, get_integer_arg, get_integer_ref, get_table_arg,
};
use crate::libs::lazy_module::LazyModule;
use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind, libs::BuiltinInfo, libs::State,
    reg_info, reg_lazy,
};
use std::collections::BTreeMap;

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        len, header_len,
        get, select, headers,
        rows, first, last, nth, grep, find, find_last, filter,
        sortby
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        len => "count rows", "<table>"
        header_len => "count headers", "<table>"
        get => "get column by header/index", "<table> <header|index>"
        select => "select columns", "<table> <cols...>"
        headers => "list headers", "<table>"
        rows => "list rows", "<table> <to_map?>"
        first => "get first row", "<table> <to_map?>"
        last => "get last row", "<table> <to_map?>"
        nth => "get nth row", "<table> <to_map?>"
        grep => "grep rows which contains the string", "<table> <string>"
        find => "find first row index of matching cell", "<list> <cell|fn> [start_index]"
        find_last => "find last row index of matching cell", "<list> <cell|fn> [start_index]"
        filter => "filter rows by condition/cell match", "<list> <cell|fn>"
        sortby => "sort a table by column", "<table> <col>"
    })
}

fn len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", &args, 1, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    Ok(Expression::Integer(table.row_count() as i64))
}
fn header_len(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("header_len", &args, 1, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    Ok(Expression::Integer(table.column_count() as i64))
}
fn get(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("get", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let key = it.next().unwrap();
    let idx = match key {
        Expression::Integer(i) => i as usize,
        Expression::String(s) | Expression::Symbol(s) => table
            .headers()
            .iter()
            .position(|x| x == &s)
            .ok_or(RuntimeError::common(
                format!("column {} not found", &s).into(),
                ctx.clone(),
                0,
            ))?,
        e => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "String/Index as key".into(),
                    found: e.type_name(),
                    sym: e.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }
    };
    Ok(table
        .get_column(idx)
        .map_or(Expression::None, Expression::from))
}
pub fn select(
    mut args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("select", &args, 2.., ctx)?;
    let headers: Vec<String> = match args.split_off(1) {
        s if s.len() == 1 => match s.first().unwrap() {
            Expression::List(list) => list.as_ref().iter().map(|x| x.to_string()).collect(),
            Expression::BSet(list) => list.as_ref().iter().map(|x| x.to_string()).collect(),
            _ => s.iter().map(|x| x.to_string()).collect(),
        },
        s => s.iter().map(|x| x.to_string()).collect(),
    };
    // let data = get_list_ref(&args[0], ctx)?;
    let data_expr = args.into_iter().next().unwrap();
    let data = get_table_arg(data_expr, ctx)?;

    match data.get_columns(&data.get_header_indexes(&headers)) {
        Some(rows) => Ok(Expression::Table(TableData::new(headers, rows))),
        None => Ok(Expression::None),
    }
}
fn headers(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("headers", &args, 1, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    Ok(Expression::from(table.headers().to_vec()))
}

fn rows(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("rows", &args, 1..=2, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let to_map = it.next().map_or(false, |x| x.is_truthy());
    if to_map {
        let r: Vec<BTreeMap<String, Expression>> = table
            .rows()
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, x)| {
                        (
                            table
                                .headers()
                                .get(i)
                                .cloned()
                                .unwrap_or("unkown".to_string()),
                            x.clone(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .collect();
        Ok(Expression::from(r))
    } else {
        Ok(Expression::from(table.rows().to_vec()))
    }
}

fn first(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("frist", &args, 1..=2, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let to_map = it.next().map_or(false, |x| x.is_truthy());

    let row = table.rows().first();
    match row {
        None => Ok(Expression::None),
        Some(row) => {
            if to_map {
                let r = row
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        (
                            table
                                .headers()
                                .get(i)
                                .cloned()
                                .unwrap_or("unkown".to_string()),
                            x.clone(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                Ok(Expression::from(r))
            } else {
                Ok(Expression::from(row.clone()))
            }
        }
    }
}
fn last(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("last", &args, 1..=2, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let to_map = it.next().map_or(false, |x| x.is_truthy());

    let row = table.rows().last();
    match row {
        None => Ok(Expression::None),
        Some(row) => {
            if to_map {
                let r = row
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        (
                            table
                                .headers()
                                .get(i)
                                .cloned()
                                .unwrap_or("unkown".to_string()),
                            x.clone(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                Ok(Expression::from(r))
            } else {
                Ok(Expression::from(row.clone()))
            }
        }
    }
}
fn nth(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("nth", &args, 2..=3, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let index = it.next().unwrap();
    let idx = get_integer_arg(index, ctx)? as usize;
    let to_map = it.next().map_or(false, |x| x.is_truthy());

    let row = table.rows().iter().nth(idx);
    match row {
        None => Ok(Expression::None),
        Some(row) => {
            if to_map {
                let r = row
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        (
                            table
                                .headers()
                                .get(i)
                                .cloned()
                                .unwrap_or("unkown".to_string()),
                            x.clone(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                Ok(Expression::from(r))
            } else {
                Ok(Expression::from(row.clone()))
            }
        }
    }
}

pub fn sortby(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sortby", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let list = it.next().unwrap();
    let key = it.next().unwrap();

    let table = get_table_arg(list, ctx)?;

    let col = match key {
        Expression::Integer(i) => i as usize,
        Expression::String(s) | Expression::Symbol(s) => {
            table.headers().iter().position(|x| x == &s).unwrap_or(0)
        }
        e => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "Integer/String as 2nd arg to sort a table".into(),
                    found: e.type_name(),
                    sym: e.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }
    };
    Ok(Expression::Table(table.sort_by_column(col)))
}

fn grep(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("grep", &args, 2, ctx)?;
    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let keyword = it.next().unwrap().to_string();

    let r: Vec<Vec<Expression>> = table
        .rows()
        .iter()
        .filter(|x| x.iter().any(|c| c.to_string().contains(&keyword)))
        .cloned()
        .collect();
    Ok(Expression::from(r))
}

fn find(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("find", &args, 2..=3, ctx)?;

    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let target = it.next().unwrap();
    let start = if let Some(start_expr) = it.next() {
        get_integer_ref(&start_expr, ctx)? as usize
    } else {
        0
    };

    match &target {
        Expression::Function(..) | Expression::Lambda(..) => {
            let state = &mut State::new();
            for (i, row) in table.rows().iter().enumerate().skip(start) {
                let r = &target.eval_apply(
                    &target,
                    &vec![Expression::from(row.clone())],
                    state,
                    env,
                    0,
                )?;
                if let Expression::Boolean(true) = r {
                    return Ok(Expression::Integer(i as i64));
                }
            }
            Ok(Expression::None)
        }
        _ => Ok(
            match table
                .rows()
                .iter()
                .skip(start)
                .position(|x| x.iter().any(|c| c == &target))
            {
                Some(index) => Expression::Integer(index as i64),
                None => Expression::None,
            },
        ),
    }
}

fn find_last(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("find_last", &args, 2..=3, ctx)?;

    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let target = it.next().unwrap();
    let start = if let Some(start_expr) = it.next() {
        get_integer_ref(&start_expr, ctx)? as usize
    } else {
        0
    };

    match &target {
        Expression::Function(..) | Expression::Lambda(..) => {
            let state = &mut State::new();
            for (i, row) in table.rows().iter().enumerate().rev().skip(start) {
                let r = &target.eval_apply(
                    &target,
                    &vec![Expression::from(row.clone())],
                    state,
                    env,
                    0,
                )?;
                if let Expression::Boolean(true) = r {
                    return Ok(Expression::Integer(i as i64));
                }
            }
            Ok(Expression::None)
        }
        _ => Ok(
            match table
                .rows()
                .iter()
                .rev()
                .skip(start)
                .position(|x| x.iter().any(|c| c == &target))
            {
                Some(index) => Expression::Integer(index as i64),
                None => Expression::None,
            },
        ),
    }
}

fn filter(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("filter", &args, 2, ctx)?;

    let mut it = args.into_iter();
    let data = it.next().unwrap();
    let table = get_table_arg(data, ctx)?;
    let target = it.next().unwrap();

    let result: Vec<Vec<Expression>> = match &target {
        Expression::Function(..) | Expression::Lambda(..) => {
            let state = &mut State::new();
            table
                .rows()
                .iter()
                .filter(|&row| {
                    target
                        .eval_apply(&target, &vec![Expression::from(row.clone())], state, env, 0)
                        .map_or(false, |r| r.is_truthy())
                })
                .cloned()
                .collect()
        }
        _ => table
            .rows()
            .iter()
            .filter(|row| row.iter().any(|col| col == &target))
            .cloned()
            .collect(),
    };
    Ok(Expression::from(result))
}
