use std::rc::Rc;

use crate::{Environment, RuntimeError};

use super::{CatchType, Expression, Int, eval::State};
use common_macros::b_tree_map;
// use common_macros::hash_map;

pub fn catch_error(
    e: RuntimeError,
    body: &Rc<Expression>,
    typ: &CatchType,
    deeling: &Option<Rc<Expression>>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    match typ {
        CatchType::Deel => match deeling {
            Some(deel) => match deel.as_ref() {
                Expression::Symbol(..) | Expression::Lambda(..) | Expression::Function(..) => {
                    // dbg!(&deel.type_name());
                    let deeled_result = deel
                        .as_ref()
                        .append_args(vec![Expression::from(b_tree_map! {
                            // String::from("type") => Expression::String(e.type_name()),
                            String::from("msg") => Expression::String(e.to_string()),
                            String::from("code") => Expression::Integer(Int::from(e.code())),
                            String::from("expr") => Expression::Quote(body.clone())
                        })])
                        .eval_mut(state, env, depth);
                    deeled_result
                }
                _ => deel.as_ref().eval_mut(state, env, depth),
            },
            _ => Ok(Expression::None),
        },
        CatchType::Ignore => Ok(Expression::None),
        CatchType::PrintStd => {
            println!("[Err->Std] {:?}", e);
            Ok(Expression::None)
        }
        CatchType::PrintErr => {
            eprintln!("\x1b[38;5;9m[Err] {:?}\x1b[m\x1b[0m", e);
            Ok(Expression::None)
        }
        CatchType::PrintOver => Ok(Expression::String(e.to_string())),
    }
    // Ok(Expression::None)
}
