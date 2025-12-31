// Helper functions

use crate::{Environment, Expression, LmError};

pub fn check_args_len(
    name: impl ToString,
    args: &[Expression],
    expected_len: impl std::ops::RangeBounds<usize>,
) -> Result<(), LmError> {
    if expected_len.contains(&args.len()) {
        Ok(())
    } else {
        Err(LmError::CustomError(format!(
            "mismatched count of arguments for function {}",
            name.to_string()
        )))
    }
}

pub fn check_exact_args_len(
    name: impl ToString,
    args: &[Expression],
    expected_len: usize,
) -> Result<(), LmError> {
    if args.len() == expected_len {
        Ok(())
    } else {
        Err(LmError::ArgumentMismatch {
            name: name.to_string(),
            expected: expected_len,
            received: args.len(),
        })
    }
}

// pub fn get_list_arg(expr: Expression) -> Result<Rc<Vec<Expression>>, LmError> {
//     match expr {
//         Expression::List(s) => Ok(s),
//         _ => Err(LmError::CustomError("expected string".to_string())),
//     }
// }

// pub fn get_list_args(
//     args: &[Expression],
//     env: &mut Environment,
// ) -> Result<Vec<Rc<Vec<Expression>>>, LmError> {
//     args.iter()
//         .map(|arg| get_list_arg(arg.eval(env)?))
//         .collect()
// }

pub fn get_exact_string_arg(expr: Expression) -> Result<String, LmError> {
    match expr {
        Expression::String(s) => Ok(s),
        e => Err(LmError::TypeError {
            expected: "String".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
pub fn get_string_arg(expr: Expression) -> Result<String, LmError> {
    match expr {
        Expression::Symbol(s) | Expression::String(s) => Ok(s),
        e => Err(LmError::TypeError {
            expected: "String".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}

pub fn get_string_args(args: &[Expression], env: &mut Environment) -> Result<Vec<String>, LmError> {
    args.iter()
        .map(|arg| get_string_arg(arg.eval(env)?))
        .collect()
}

pub fn get_integer_arg(expr: Expression) -> Result<i64, LmError> {
    match expr {
        Expression::Integer(i) => Ok(i),
        e => Err(LmError::TypeError {
            expected: "Integer".to_string(),
            found: e.type_name(),
            sym: e.to_string(),
        }),
    }
}
