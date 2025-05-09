use std::rc::Rc;

use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        String::from("try") => Expression::builtin("try", try_builtin,
            "try an expression or apply an error handler to an error"),
        String::from("codes") => LmError::codes()
    })
    .into()
}

fn try_builtin(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    // Try to evaluate the first argument, if it fails, apply the second argument to the error
    // message.
    if args.len() != 2 {
        return Err(LmError::CustomError(
            "try requires exactly two arguments: an expression to try to evaluate, and an error handler that takes an error as an argument".to_string(),
        ));
    }

    match args[0].eval(env) {
        Err(err) => {
            let handler = args[1].clone();

            Ok(Expression::Apply(
                Rc::new(handler),
                Rc::new(vec![Expression::from(hash_map! {
                    String::from("message") => Expression::String(err.to_string()),
                    String::from("code") => Expression::Integer(Int::from(err.code())),
                    String::from("expression") => Expression::Quote(Rc::new(args[0].clone()))
                })]),
            )
            .eval(env)?)
        }
        result => Ok(result?),
    }
}
