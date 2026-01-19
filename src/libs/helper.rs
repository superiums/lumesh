// Helper functions

use crate::{Environment, Expression, RuntimeError, RuntimeErrorKind};

// use std::rc::Rc;

// 函数注册宏 - 自动推导函数名
#[macro_export]
macro_rules! reg_lazy {
    ({ $($func:ident),* $(,)? }) => {
       {
        let module = LazyModule::new();
        $(
            module.register(stringify!($func), || {
                std::rc::Rc::new($func)
            });
        )*
        module
       }
    };
}
#[macro_export]
macro_rules! reg_all {
    ({ $($func:ident),* $(,)? }) => {
        {
            let mut module: HashMap<&'static str, Rc<BuiltinFunc>> = HashMap::new();
            $(
                module.insert(stringify!($func), std::rc::Rc::new($func));
            )*
            module
        }
    };
}
#[macro_export]
macro_rules! reg_info {
    ({ $($func:ident => $desc:expr, $hint:expr)* $(;)? }) => {
        {
            let mut info :HashMap<&'static str, BuiltinInfo> = HashMap::new();
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
                expected.start_bound(),
                expected.end_bound(),
                args.len()
            )
            .into(),
            ctx.clone(),
            0,
        ))
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

pub fn get_exact_string_arg(expr: Expression, ctx: &Expression) -> Result<String, RuntimeError> {
    match expr {
        Expression::String(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "String".to_string(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}
pub fn get_string_arg(expr: Expression, ctx: &Expression) -> Result<String, RuntimeError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "String".to_string(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}

pub fn get_string_args(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Vec<String>, RuntimeError> {
    args.iter()
        .map(|arg| get_string_arg(arg.eval(env)?, ctx))
        .collect()
}

pub fn get_integer_arg(expr: Expression, ctx: &Expression) -> Result<i64, RuntimeError> {
    match expr {
        Expression::Integer(i) => Ok(i),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer".to_string(),
                sym: e.to_string(),
                found: e.type_name(),
            },
            ctx.clone(),
            0,
        )),
    }
}
