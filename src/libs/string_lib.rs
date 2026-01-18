use std::collections::HashMap;

use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    libs::{
        BuiltinInfo,
        helper::{check_exact_args_len, get_string_arg},
        lazy_module::LazyModule,
    },
    reg_lazy,
};

pub fn regist_lazy() -> LazyModule {
    let module = LazyModule::new();

    reg_lazy!(
        module,
        {
            // split => "split a string on a given character", "[delimiter] <string>";
            // join => "join strings", "<string>...";
            // len => "get length of string", "<string>";
            // trim => "trim whitespace from a string", "<string>";
            to_lower => "convert a string to lowercase", "<string>";
            to_upper => "convert a string to uppercase", "<string>";
            // replace => "replace all instances of a substring", "<old> <new> <string>";
            // contains => "check if a string contains a given substring", "<substring> <string>";
        }
    );

    module
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    let mut info = HashMap::with_capacity(100);
    info.insert(
        "to_lower",
        BuiltinInfo {
            descr: "convert a string to lowercase",
            hint: "<string>",
        },
    );
    info
}
// pub fn get_string_function(name: &str) -> Option<Expression> {
//     STRING_MODULE.with(|m| m.get_function(name))
// }

fn to_lower(
    base: &Expression,
    args: &[Expression],
    env: &mut Environment,
    contex: &Expression,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    // check_exact_args_len("to_lower", args, 1)?;
    // let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(base.to_string().to_lowercase()))
}

fn to_upper(
    base: &Expression,
    args: &[Expression],
    env: &mut Environment,
    contex: &Expression,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    // check_exact_args_len("to_upper", args, 1)?;
    // let text = get_string_arg(args[0].eval(env)?)?;
    Ok(Expression::String(base.to_string().to_uppercase()))
}
