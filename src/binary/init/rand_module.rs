use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
use rand::{Rng, distributions::Uniform, prelude::SliceRandom};

pub fn get() -> Expression {
    (hash_map! {
        String::from("int") => Expression::builtin("int", int, "get a random integer between two numbers (exclusive)"),
        String::from("choose") => Expression::builtin("choose", choose, "choose a random item in a list"),
        String::from("shuffle") => Expression::builtin("shuffle", shuffle, "shuffle a list randomly"),
    })
    .into()
}

fn int(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("int", &args, 2)?;
    match (args[0].eval(env)?, args[1].eval(env)?) {
        (Expression::Integer(l), Expression::Integer(h)) => {
            let mut rng = rand::thread_rng();
            let n = Uniform::new(l, h);
            Ok(Expression::Integer(rng.sample(n)))
        }
        (l, h) => Err(LmError::CustomError(format!(
            "expected two integers, but got {} and {}",
            l, h
        ))),
    }
}

fn choose(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("choose", &args, 1)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::thread_rng();
            let n = Uniform::new(0, list.as_ref().len());
            Ok(list.as_ref()[rng.sample(n)].clone())
        }
        otherwise => Err(LmError::CustomError(format!(
            "expected a list, but got {}",
            otherwise
        ))),
    }
}

fn shuffle(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("shuffle", &args, 1)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::thread_rng();
            list.as_ref().clone().shuffle(&mut rng);
            Ok(Expression::List(list))
        }
        otherwise => Err(LmError::CustomError(format!(
            "expected a list, but got {}",
            otherwise
        ))),
    }
}
