// Helper functions

use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use crate::{Expression, RuntimeError, RuntimeErrorKind, expression::table::TableData};

// use std::rc::Rc;

// 函数注册宏 - 自动推导函数名
#[macro_export]
macro_rules! reg_lazy {
    ({ $($func:ident $( => $name:expr )? ),* $(,)? }) => {
       {
        let module = LazyModule::new();
        $(
            reg_lazy!(@insert module, $func, $($name)?);
            // module.register(stringify!($func), || {
            //     std::rc::Rc::new($func)
            // });
        )*
        module
       }
    };

    (@insert $module:ident, $func:ident, $name:expr) => {
        $module.register($name, ||{std::rc::Rc::new($func)});
    };

    (@insert $module:ident, $func:ident,) => {

        $module.register(stringify!($func), ||{std::rc::Rc::new($func)});
    };
}
#[macro_export]
macro_rules! reg_all {
    ({ $($item:ident $( => $name:expr )? ),* $(,)? }) => {
        {
            let mut module: HashMap<&'static str, std::rc::Rc<BuiltinFunc>> = HashMap::new();
            $(
                reg_all!(@insert module, $item, $($name)?);
            )*
            module
        }
    };

    (@insert $module:ident, $func:ident, $name:expr) => {
        $module.insert($name, std::rc::Rc::new($func));
    };

    (@insert $module:ident, $func:ident,) => {
        $module.insert(stringify!($func), std::rc::Rc::new($func));
    };
}
#[macro_export]
macro_rules! reg_info {
    ({ $($func:ident => $desc:expr, $hint:expr)* $(;)? }) => {
        {

            let mut info :BTreeMap<&'static str, BuiltinInfo> = BTreeMap::new();
            $(
                info.insert(stringify!($func), BuiltinInfo {
                    descr: $desc,
                    hint: $hint
                });
            )*
            info
        }
    };
}

pub fn check_args_len(
    name: impl ToString,
    args: &[Expression],
    expected: impl std::ops::RangeBounds<usize>,
    ctx: &Expression,
) -> Result<(), RuntimeError> {
    if expected.contains(&args.len()) {
        Ok(())
    } else {
        Err(RuntimeError::common(
            format!(
                "arguments for `{}` not match, expected {:?}..{:?}, found: {}",
                name.to_string(),
                get_bounds(expected.start_bound()),
                get_bounds(expected.end_bound()),
                args.len()
            )
            .into(),
            ctx.clone(),
            0,
        ))
    }
}
fn get_bounds(b: std::ops::Bound<&usize>) -> String {
    match b {
        std::ops::Bound::Included(&n) => n.to_string(),
        std::ops::Bound::Excluded(&n) => (n + 1).to_string(),
        std::ops::Bound::Unbounded => "_".to_string(),
    }
}
pub fn check_exact_args_len(
    name: impl ToString,
    args: &[Expression],
    expected: usize,
    ctx: &Expression,
) -> Result<(), RuntimeError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::ArgumentMismatch {
                name: name.to_string(),
                expected,
                received: args.len(),
            },
            ctx.clone(),
            0,
        ))
    }
}

// pub fn get_list_arg(expr: Expression) -> Result<Rc<Vec<Expression>>, RuntimeError> {
//     match expr {
//         Expression::List(s) => Ok(s),
//         _ => Err(LmError::CustomError("expected string".to_string())),
//     }
// }

// pub fn get_list_args(
//     args: &[Expression],
//     env: &mut Environment,
// ) -> Result<Vec<Rc<Vec<Expression>>>, RuntimeError> {
//     args.iter()
//         .map(|arg| get_list_arg(arg.eval(env)?))
//         .collect()
// }

// pub fn get_exact_string_arg(expr: Expression, ctx: &Expression) -> Result<String, RuntimeError> {
//     match expr {
//         Expression::String(s) => Ok(s),
//         e => Err(RuntimeError::new(
//             RuntimeErrorKind::TypeError {
//                 expected: "String".to_string(),
//                 sym: e.to_string(),
//                 found: e.type_name(),
//             },
//             ctx.clone(),
//             0,
//         )),
//     }
// }
pub fn get_string_arg(expr: Expression, ctx: &Expression) -> Result<String, RuntimeError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "String".into(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}
pub fn get_string_ref<'a>(
    expr: &'a Expression,
    ctx: &Expression,
) -> Result<&'a String, RuntimeError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "String".into(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}

// pub fn get_string_args(
//     args: &[Expression],
//     env: &mut Environment,
//     ctx: &Expression,
// ) -> Result<Vec<String>, RuntimeError> {
//     args.iter()
//         .map(|arg| get_string_arg(arg.clone(), ctx))
//         .collect()
// }

pub fn get_integer_arg(expr: Expression, ctx: &Expression) -> Result<i64, RuntimeError> {
    match expr {
        Expression::Integer(i) => Ok(i),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer".into(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}
pub fn get_integer_ref(expr: &Expression, ctx: &Expression) -> Result<i64, RuntimeError> {
    match expr {
        Expression::Integer(i) => Ok(*i),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer".into(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn get_map_ref<'a>(
    expr: &'a Expression,
    ctx: &Expression,
) -> Result<&'a Rc<BTreeMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::Map(m) => Ok(m),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Map".into(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn get_hmap_ref<'a>(
    expr: &'a Expression,
    ctx: &Expression,
) -> Result<&'a Rc<HashMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::HMap(m) => Ok(m),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "HMap".into(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn into_map(
    expr: Expression,
    ctx: &Expression,
) -> Result<Rc<BTreeMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::Map(m) => Ok(m),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Map".into(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn into_hmap(
    expr: Expression,
    ctx: &Expression,
) -> Result<Rc<HashMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::HMap(m) => Ok(m),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "HMap".into(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn check_fn_arg(
    fn_arg: &Expression,
    size: usize,
    ctx: &Expression,
) -> Result<(), RuntimeError> {
    let fn_arg_count = match fn_arg {
        Expression::Lambda(params, ..) => params.len(),
        Expression::Function(_, params, _, _, _) => params.len(),
        _ => {
            return Err(RuntimeError::common(
                "expect a func/lambda as param".into(),
                ctx.clone(),
                0,
            ));
        }
    };
    if fn_arg_count != size {
        return Err(RuntimeError::common(
            format!("your func/lambda should define {} param", size).into(),
            ctx.clone(),
            0,
        ));
    }
    Ok(())
}

// 辅助函数：将 List<Map> 转换为 TableData
pub fn convert_list_map_to_table(list: &Vec<Expression>) -> TableData {
    if list.is_empty() {
        return TableData::with_header(Vec::new());
    }

    // 收集所有可能的键作为表头
    let mut all_keys = std::collections::BTreeSet::new();
    for item in list.iter() {
        if let Expression::Map(map) = item {
            all_keys.extend(map.keys());
        }
    }
    let headers: Vec<String> = all_keys.into_iter().cloned().collect();

    // 转换数据
    let mut rows = Vec::new();
    for item in list.iter() {
        if let Expression::Map(map) = item {
            let row: Vec<Expression> = headers
                .iter()
                .map(|key| map.get(key).cloned().unwrap_or(Expression::None))
                .collect();
            rows.push(row);
        }
    }

    TableData::new(headers, rows)
}

pub fn get_table_arg(expr: Expression, ctx: &Expression) -> Result<TableData, RuntimeError> {
    match expr {
        Expression::List(list) => Ok(convert_list_map_to_table(&list)),
        Expression::Table(t) => Ok(t),
        e => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "Table/List as 1st arg for sortby".into(),
                    found: e.type_name(),
                    sym: e.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }
    }
}
