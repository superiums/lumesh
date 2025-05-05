use crate::{Environment, RuntimeError};

use super::{CatchType, Expression, Int};
use common_macros::b_tree_map;

pub fn catch_error(
    e: RuntimeError,
    body: Box<Expression>,
    typ: CatchType,
    deeling: Option<Box<Expression>>,
    env: &mut Environment,
) -> Result<Expression, RuntimeError> {
    return match typ {
        CatchType::Deel => match deeling {
            Some(deel) => {
                // dbg!(&deel.type_name());
                let deeled_result = deel
                    .append_args(vec![Expression::Map(b_tree_map! {
                        // String::from("type") => Expression::String(e.type_name()),
                        String::from("msg") => Expression::String(e.to_string()),
                        String::from("code") => Expression::Integer(Int::from(e.code())),
                        String::from("expr") => Expression::Quote(body)
                    })])
                    .eval_mut(true, env, 0);
                deeled_result
            }
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
    };
    // Ok(Expression::None)
}
