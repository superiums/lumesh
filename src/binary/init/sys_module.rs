use std::rc::Rc;

use crate::{Environment, Expression};
use common_macros::hash_map;

pub fn get() -> Expression {
    Expression::from(hash_map! {
        String::from("parse") => Expression::builtin("parse", |args, env| {
            super::check_exact_args_len("parse", &args, 1)?;
            let expr = args[0].eval(env)?;
            Ok(match crate::parse(&expr.to_string()) {
                Ok(expr) => expr,
                Err(_) => Expression::None
            })
        }, "parse an expression"),

        String::from("quote") => Expression::builtin("quote", |args, _env| {
            super::check_exact_args_len("quote", &args, 1)?;
            Ok(Expression::Quote(Rc::new(args[0].clone())))
        }, "quote an expression"),

        // String::from("eval") => Expression::builtin("eval", |args, env| {
        //     let mut new_env = env.clone();
        //     Ok(args[0].eval(env)?.eval(&mut new_env)?)
        // }, "evaluate an expression without changing the environment"),

        // String::from("exec") => Expression::builtin("exec", |args, env| {
        //     Ok(args[0].eval(env)?.eval(env)?)
        // }, "evaluate an expression in the current environment"),

        // Evaluate a file in the current environment.
        // String::from("include") => Expression::builtin("include", |args, env| {
        //     super::check_exact_args_len("include", &args, 1)?;

        //     let cwd = std::env::current_dir()?;
        //     let path = cwd.join(args[0].eval(env)?.to_string());

        //     if let Ok(canon_path) = dunce::canonicalize(&path) {
        //         // Read the file.
        //         let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| LmError::CustomError(format!("could not read file {}: {}", canon_path.display(), e)))?;
        //         // Evaluate the file.
        //         if let Ok(expr) = crate::parse(&contents) {
        //             Ok(expr.eval(env)?)
        //         } else {
        //             Err(LmError::CustomError(format!("could not parse file {}", canon_path.display())))
        //         }
        //     } else {
        //         Err(LmError::CustomError(format!("could not canonicalize path {}", path.display())))
        //     }
        // }, "evaluate a file in the current environment"),

        // Change the current working directory.
        // String::from("cd") => Expression::builtin("cd", |args, env| {
        //     super::check_exact_args_len("cd", &args, 1)?;
        //     let cwd = std::env::current_dir()?;
        //     let path = cwd.join(args[0].eval(env)?.to_string());

        //     if let Ok(canon_path) = dunce::canonicalize(&path) {
        //         // env.set_cwd(canon_path.to_str().unwrap().to_string());
        //         Ok(Expression::None)
        //     } else {
        //         Err(LmError::CustomError(format!("could not canonicalize path {}", path.display())))
        //     }
        // }, "change the current working directory"),

        // Get the current working directory.
        // String::from("cwd") => Expression::builtin("cwd", |_args, env| {
        //     let path = std::env::current_dir()?;
        //     Ok(Expression::String(path.))
        // }, "get the current working directory"),

        // Import a file (evaluate it in a new environment).
        // String::from("import") => Expression::builtin("import", |args, env| {
        //     super::check_exact_args_len("import", &args, 1)?;
        //     let cwd = std::env::current_dir()?;
        //     let path = cwd.join(args[0].eval(env)?.to_string());

        //     if let Ok(canon_path) = dunce::canonicalize(&path) {
        //         // Read the file.
        //         let contents = std::fs::read_to_string(canon_path.clone()).map_err(|e| LmError::CustomError(format!("could not read file {}: {}", canon_path.display(), e)))?;
        //         // Evaluate the file.
        //         if let Ok(expr) = crate::parse(&contents) {
        //             let mut new_env = env.clone();
        //             Ok(expr.eval(&mut new_env)?)
        //         } else {
        //             Err(LmError::CustomError(format!("could not parse file {}", canon_path.display())))
        //         }
        //     } else {
        //         Err(LmError::CustomError(format!("could not canonicalize path {}", path.display())))
        //     }
        // }, "import a file (evaluate it in a new environment)"),

        String::from("env") => Expression::builtin("env", |_args, env| {
            Ok(Expression::from(env.clone()))
        }, "get the current environment as a map"),
        String::from("vars") => Expression::builtin("vars", vars, "get a table of the defined variables"),

        String::from("set") => Expression::builtin("set", |args, env| {
            super::check_exact_args_len("set", &args, 2)?;
            let name = args[0].to_string();
            let expr = args[1].clone();
            env.define(&name, expr);
            Ok(Expression::None)
        }, "define a variable in the current environment"),

        String::from("unset") => Expression::builtin("unset", |args, env| {
            super::check_exact_args_len("unset", &args, 1)?;
            let name = args[0].to_string();
            env.undefine(&name);
            Ok(Expression::None)
        }, "undefine a variable in the current environment"),

        String::from("defined") => Expression::builtin("defined", |args, env| {
            super::check_exact_args_len("defined", &args, 1)?;
            let name = args[0].to_string();
            Ok(Expression::Boolean(env.is_defined(&name)))
        }, "check if a variable is defined in the current environment"),
    })
}

fn vars(_: &Vec<Expression>, env: &mut Environment) -> Result<Expression, crate::LmError> {
    Ok(Expression::from(env.get_bindings_map()))
}
