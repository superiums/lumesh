use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
use rand::distr::SampleString;
use rand::prelude::*;
use rand::{Rng, prelude::SliceRandom};

pub fn get() -> Expression {
    (hash_map! {
        String::from("ratio") => Expression::builtin("ratio", ratio, "get a bool with ratio"),
        String::from("alpha") => Expression::builtin("alpha", alpha, "get random char(s)"),
        String::from("alphanum") => Expression::builtin("alphanum", alphanum, "get random alphanumeric string"),
        String::from("int") => Expression::builtin("int", int, "get a random integer between two numbers (exclusive)"),
        String::from("choose") => Expression::builtin("choose", choose, "choose a random item in a list"),
        String::from("shuffle") => Expression::builtin("shuffle", shuffle, "shuffle a list randomly"),
    })
    .into()
}

fn ratio(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let b = rng.random_bool(0.5);
            Ok(Expression::Boolean(b))
        }
        1 => {
            if let Expression::Float(f) = args[0].eval(env)? {
                let mut rng = rand::rng();
                let b = rng.random_bool(f);
                Ok(Expression::Boolean(b))
            } else {
                Err(LmError::CustomError(
                    "rand.ratio expected integer args".to_string(),
                ))
            }
        }
        2 => match (args[0].eval(env)?, args[1].eval(env)?) {
            (Expression::Integer(numerator), Expression::Integer(denominator)) => {
                let mut rng = rand::rng();
                let b = rng.random_ratio(numerator as u32, denominator as u32);
                Ok(Expression::Boolean(b))
            }
            (l, h) => Err(LmError::CustomError(format!(
                "rand.ratio expected two integers, but got {} and {}",
                l, h
            ))),
        },
        _ => Err(LmError::CustomError(
            "rand.ratio expected 0, or 1 arguments".to_string(),
        )),
    }
}
fn alpha(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let c = rng.random::<char>();
            Ok(Expression::String(c.to_string()))
        }
        1 => {
            if let Expression::Integer(size) = args[0].eval(env)? {
                let mut rng = rand::rng();
                let a = rand::distr::Alphabetic.sample_string(&mut rng, size as usize);
                Ok(Expression::String(a))
            } else {
                Err(LmError::CustomError(
                    "rand.alpha expected integer args".to_string(),
                ))
            }
        }
        _ => Err(LmError::CustomError(
            "rand.alpha expected 0, or 1 arguments".to_string(),
        )),
    }
}
fn alphanum(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => {
            let mut rng = rand::rng();
            let c = rng.sample(rand::distr::Alphanumeric) as char;
            Ok(Expression::String(c.to_string()))
        }
        1 => {
            let mut rng = rand::rng();
            if let Expression::Integer(size) = args[0].eval(env)? {
                let a = rand::distr::Alphanumeric.sample_string(&mut rng, size as usize);
                Ok(Expression::String(a))
            } else {
                Err(LmError::CustomError(
                    "rand.alphanum expected integer args".to_string(),
                ))
            }
        }
        _ => Err(LmError::CustomError(
            "rand.alphanum expected 0, or 1 arguments".to_string(),
        )),
    }
}
///生成随机整数
///支持 0、1 或 2 个参数：
///
///0 参数：返回一个随机的 i64 整数（范围为 i64::MIN 到 i64::MAX）。
///
///1 参数：返回一个介于 0 和 max 之间的整数（包含 max）。
///
///2 参数：返回一个介于 min 和 max 之间的整数（包含 min 和 max）。
fn int(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => {
            let n: i64 = rand::rng().random();
            Ok(Expression::Integer(n))
        }
        1 => {
            if let Expression::Integer(max) = args[0].eval(env)? {
                let mut rng = rand::rng();
                if max < 0 {
                    let n = rng.random_range(max..=0);
                    Ok(Expression::Integer(n))
                } else {
                    let n = rng.random_range(0..=max);
                    Ok(Expression::Integer(n))
                }
            } else {
                Err(LmError::CustomError(
                    "rand.int expected integer args".to_string(),
                ))
            }
        }
        2 => match (args[0].eval(env)?, args[1].eval(env)?) {
            (Expression::Integer(l), Expression::Integer(h)) => {
                let mut rng = rand::rng();
                Ok(Expression::Integer(rng.random_range(l..h)))
            }
            (l, h) => Err(LmError::CustomError(format!(
                "rand.int expected two integers, but got {} and {}",
                l, h
            ))),
        },
        _ => Err(LmError::CustomError(
            "rand.int expected 0, 1 or 2 arguments".to_string(),
        )),
    }
}

///接受一个列表参数，返回一个随机选择的元素。
fn choose(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("choose", args, 1)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::rng();
            Ok(match list.choose(&mut rng) {
                Some(s) => s.clone(),
                None => Expression::None,
            })
        }
        otherwise => Err(LmError::CustomError(format!(
            "rand.choose expected a list, but got {}",
            otherwise
        ))),
    }
}

///接受一个列表参数，返回一个新的被打乱顺序的列表。
fn shuffle(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("shuffle", args, 1)?;
    match args[0].eval(env)? {
        Expression::List(list) => {
            let mut rng = rand::rng();
            list.as_ref().clone().shuffle(&mut rng);
            Ok(Expression::List(list))
        }
        otherwise => Err(LmError::CustomError(format!(
            "rand.shuffle expected a list, but got {}",
            otherwise
        ))),
    }
}
