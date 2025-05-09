use std::rc::Rc;

use crate::{Environment, RuntimeError};

use super::{CatchType, Expression, Int};
use common_macros::hash_map;

pub fn catch_error(
    e: RuntimeError,
    body: Rc<Expression>,
    typ: CatchType,
    deeling: Option<Rc<Expression>>,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    match typ {
        CatchType::Deel => match deeling {
            Some(deel) => match *deel {
                Expression::Symbol(..) | Expression::Lambda(..) | Expression::Function(..) => {
                    // dbg!(&deel.type_name());
                    let deeled_result = deel
                        .as_ref()
                        .clone()
                        .append_args(vec![Expression::Map(Rc::new(hash_map! {
                            // String::from("type") => Expression::String(e.type_name()),
                            String::from("msg") => Expression::String(e.to_string()),
                            String::from("code") => Expression::Integer(Int::from(e.code())),
                            String::from("expr") => Expression::Quote(body)
                        }))])
                        .eval_mut(true, env, depth);
                    deeled_result
                }
                _ => deel.as_ref().clone().eval_mut(true, env, depth),
            },
            _ => Ok(Expression::None),
        },
        CatchType::Ignore => Ok(Expression::None),
        CatchType::PrintStd => {
            println!("[Err->Std] {:?}", e);
            Ok(Expression::None)
        }
        CatchType::PrintErr => {
            eprintln!("[Err] {:?}", e);
            Ok(Expression::None)
        }
        CatchType::PrintOver => Ok(Expression::String(e.to_string())),
    }
    // Ok(Expression::None)
}
