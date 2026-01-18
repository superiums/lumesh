use std::{collections::HashMap, rc::Rc};

use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    libs::{BuiltinFunc, BuiltinInfo},
    reg_all,
};

pub fn regist_all() -> HashMap<&'static str, Rc<BuiltinFunc>> {
    let mut module: HashMap<&'static str, Rc<BuiltinFunc>> = HashMap::with_capacity(10);

    reg_all!(module, {
        cd;
        pwd;
    });
    // module.insert("cd", Rc::new(cd));
    // module.insert("cd", Rc::new(pwd));
    module
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    let mut info = HashMap::with_capacity(100);
    info.insert(
        "cd",
        BuiltinInfo {
            descr: "cd",
            hint: "<string>",
        },
    );
    info
}

fn cd(
    args: &[Expression],
    env: &mut Environment,
    contex: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut path = if args.len() == 0 {
        "~".to_string()
    } else {
        match args[0].eval(env)? {
            Expression::Symbol(path) | Expression::String(path) => path,
            other => other.to_string(),
        }
    };
    if path == "-" {
        path = env.get("LWD").map_or("~".to_string(), |x| x.to_string());
    }
    let _ = std::env::current_dir()
        .and_then(|x| Ok(env.define("LWD", Expression::String(x.to_string_lossy().into()))));

    if path.starts_with("~") {
        if let Some(home_dir) = dirs::home_dir() {
            path = path.replace("~", home_dir.to_string_lossy().as_ref());
        }
    }
    std::env::set_current_dir(&path)
        .map_err(|e| RuntimeError::new(RuntimeErrorKind::from(e), contex.clone(), 0))?;

    env.define_in_root("PWD", Expression::String(path));
    Ok(Expression::None)
}

fn pwd(
    _: &[Expression],
    _: &mut Environment,
    contex: &Expression,
) -> Result<Expression, RuntimeError> {
    let path = std::env::current_dir()
        .map_err(|e| RuntimeError::from_io_error(e, "pwd".into(), contex.clone(), 0))?;
    // println!("{}", path.display());
    Ok(Expression::String(path.to_string_lossy().into_owned()))
}
