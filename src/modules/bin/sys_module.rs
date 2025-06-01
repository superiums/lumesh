use std::rc::Rc;

use crate::{Environment, Expression, RuntimeError};
use common_macros::hash_map;

pub fn get() -> Expression {
    Expression::from(hash_map! {

        String::from("quote") => Expression::builtin("quote", |args, _env| {
            super::check_exact_args_len("quote", args, 1)?;
            Ok(Expression::Quote(Rc::new(args[0].clone())))
        }, "quote an expression"),

        String::from("env") => Expression::builtin("env", |_args, env| {
            Ok(Expression::from(env.clone()))
        }, "get the current environment as a map"),
        String::from("vars") => Expression::builtin("vars", vars, "get a table of the defined variables"),

        String::from("set") => Expression::builtin("set", |args, env| {
            super::check_exact_args_len("set", args, 2)?;
            let name = args[0].to_string();
            let expr = args[1].clone();
            env.define(&name, expr);
            Ok(Expression::None)
        }, "define a variable in the current environment"),

        String::from("unset") => Expression::builtin("unset", |args, env| {
            super::check_exact_args_len("unset", args, 1)?;
            let name = args[0].to_string();
            env.undefine(&name);
            Ok(Expression::None)
        }, "undefine a variable in the current environment"),

        String::from("defined") => Expression::builtin("defined", |args, env| {
            super::check_exact_args_len("defined", args, 1)?;
            let name = args[0].to_string();
            Ok(Expression::Boolean(env.is_defined(&name)))
        }, "check if a variable is defined in the current environment"),

        String::from("err-codes") =>Expression::builtin("err-codes", |_,_| Ok(RuntimeError::codes()), "display runtime error codes"),

    })
}

fn vars(_: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_bindings_map()))
}
