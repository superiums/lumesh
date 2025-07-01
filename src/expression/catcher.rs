use std::rc::Rc;

use crate::{Environment, RuntimeError, RuntimeErrorKind};

use super::{CatchType, Expression, eval::State};
use common_macros::b_tree_map;
// use common_macros::hash_map;

pub fn catch_error(
    e: RuntimeError,
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
                    dbg!(&deel.type_name());

                    deel.as_ref()
                        .apply(vec![Expression::from(b_tree_map! {
                            String::from("code") => Expression::Integer(e.code()),
                            String::from("msg") => Expression::String(e.kind.to_string()),
                            String::from("expr") => Expression::String(e.context.to_string()),
                            String::from("ast") => Expression::String(format!("{:?}",e.context)),
                            String::from("type") => Expression::String(e.context.type_name()),
                            String::from("depth") => Expression::Integer(e.depth as i64),
                            // String::from("expr") => Expression::Quote(body.clone())
                        })])
                        .eval_mut(state, env, depth + 1)
                }
                _ => deel.as_ref().eval_mut(state, env, depth + 1),
            },
            _ => Ok(Expression::None),
        },
        CatchType::Ignore => Ok(Expression::None),
        CatchType::PrintStd => {
            println!("{:?}", e);
            Ok(Expression::None)
        }
        CatchType::PrintErr => {
            eprintln!("\x1b[38;5;9m{:?}\x1b[0m", e);
            Ok(Expression::None)
        }
        CatchType::PrintOver => Ok(Expression::from(b_tree_map! {
            String::from("code") => Expression::Integer(e.code()),
            String::from("msg") => Expression::String(e.kind.to_string()),
            String::from("expr") => Expression::String(e.context.to_string()),
            String::from("ast") => Expression::String(format!("{:?}",e.context)),
            String::from("type") => Expression::String(e.context.type_name()),
            String::from("depth") => Expression::Integer(e.depth as i64),
            // String::from("expr") => Expression::Quote(body.clone())
        })),
        CatchType::Terminate => Err(RuntimeError::new(
            RuntimeErrorKind::Terminated,
            e.context,
            e.depth,
        )),
    }
    // Ok(Expression::None)
}
