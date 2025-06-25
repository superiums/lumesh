use std::{collections::BTreeMap, rc::Rc};

use crate::{Environment, Expression, LmError};
use common_macros::hash_map;
// use inquire::ui::RenderConfig;
use inquire::{Confirm, CustomType, MultiSelect, Password, PasswordDisplayMode, Select, Text};

pub fn get() -> Expression {
    (hash_map! {
        String::from("int") => Expression::builtin("int", int, "read an int from input", "<msg>"),
        String::from("float") => Expression::builtin("float", float, "read a float from input", "<msg>"),
        String::from("text") => Expression::builtin("text", text, "read a text input ", "<msg>"),
        String::from("passwd") => Expression::builtin("passwd", passwd, "read a passwd input", "<msg> [confirm?]"),
        String::from("confirm") => Expression::builtin("confirm", confirm, "ask user to confirm", "<msg>"),
        String::from("pick") => Expression::builtin("pick", pick, "select one from list/string", "[msg|cfg_map] <list|items...>"),
        String::from("multi_pick") => Expression::builtin("multi_pick", multi_pick, "select multi from list/string", "[msg|cfg_map] <list|items...>"),
    })
    .into()
}

fn int(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("text", args, 1)?;
    let msg = super::get_string_arg(args.last().unwrap().eval(env)?)?;
    // let n = super::get_integer_arg(args[0].eval(env)?)?;

    let amount = CustomType::<i64>::new(msg.as_str())
        .with_formatter(&|i| format!("${:.0}", i))
        .with_error_message("Please type a valid int")
        .with_help_message("Type an Integer");

    match amount.prompt() {
        Ok(s) => Ok(Expression::Integer(s)),
        Err(e) => Err(LmError::CustomError(format!("ui.text: {}", e.to_string()))),
    }
}
fn float(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("text", args, 1)?;
    let msg = super::get_string_arg(args.last().unwrap().eval(env)?)?;
    // let n = super::get_integer_arg(args[0].eval(env)?)?;

    let amount = CustomType::<f64>::new(msg.as_str())
        .with_formatter(&|i| format!("${:.2}", i))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount using a decimal point as a separator");

    match amount.prompt() {
        Ok(s) => Ok(Expression::Float(s)),
        Err(e) => Err(LmError::CustomError(format!("ui.text: {}", e.to_string()))),
    }
}
fn text(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("text", args, 1)?;
    let msg = super::get_string_arg(args.last().unwrap().eval(env)?)?;

    let ans = Text::new(msg.as_str());
    match ans.prompt() {
        Ok(s) => Ok(Expression::String(s)),
        Err(e) => Err(LmError::CustomError(format!("ui.text: {}", e.to_string()))),
    }
}

fn passwd(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("passwd", args, 1..=2)?;
    let msg = super::get_string_arg(args[0].eval(env)?)?;
    let confirm = args[1].eval(env)?.is_truthy();
    let mut ans = Password::new(msg.as_str()).with_display_mode(PasswordDisplayMode::Masked);
    if !confirm {
        ans = ans.without_confirmation();
    }
    match ans.prompt() {
        Ok(s) => Ok(Expression::String(s)),
        Err(e) => Err(LmError::CustomError(format!(
            "ui.passwd: {}",
            e.to_string()
        ))),
    }
}
fn confirm(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("confirm", args, 1)?;
    let msg = super::get_string_arg(args[0].eval(env)?)?;
    let ans = Confirm::new(msg.as_str()).prompt();
    match ans {
        Ok(s) => Ok(Expression::Boolean(s)),
        Err(e) => Err(LmError::CustomError(format!(
            "ui.confirm: {}",
            e.to_string()
        ))),
    }
}
fn pick(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    selector_wrapper(false, args, env)
}
fn multi_pick(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    selector_wrapper(true, args, env)
}

fn selector_wrapper(
    multi: bool,
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    let delimiter = match env.get("IFS") {
        Some(Expression::String(fs)) => fs,
        _ => " ".to_string(), // 使用空格作为默认分隔符
    };

    let (cfgs, options) = match args.len() {
        1 => (
            None,
            extract_options(delimiter.as_str(), args[0].eval(env)?)?,
        ),
        2 => (
            Some(extract_cfg(args[0].eval(env)?)?),
            extract_options(delimiter.as_str(), args[1].eval(env)?)?,
        ),
        3.. => (Some(extract_cfg(args[0].eval(env)?)?), args[1..].to_vec()),
        0 => {
            return Err(LmError::CustomError(
                "fzp requires a list as argument".to_string(),
            ));
        }
    };

    let msg = match &cfgs {
        None => "your choice:".to_string(),
        Some(m) => m
            .get("msg")
            .and_then(|v| Some(v.to_string()))
            .unwrap_or("your choice:".to_string()),
    };

    match multi {
        true => multi_select_wrapper(&msg, options, cfgs),
        false => single_select_wrapper(&msg, options, cfgs),
    }
}

fn single_select_wrapper<'a>(
    msg: &'a str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
) -> Result<Expression, LmError> {
    let mut ans = Select::new(&msg, options);
    match cfgs {
        Some(m) => {
            let page_size = m.get("page_size");
            if let Some(ps) = page_size {
                match ps {
                    Expression::Integer(size) => {
                        ans = ans.with_page_size(*size as usize);
                    }
                    _ => {
                        return Err(LmError::CustomError(
                            "page_size should be an Integer".to_string(),
                        ));
                    }
                }
            }
            let starting_cursor = m.get("starting_cursor");
            if let Some(c) = starting_cursor {
                match c {
                    Expression::Integer(c) => {
                        ans = ans.with_starting_cursor(*c as usize);
                    }
                    _ => {
                        return Err(LmError::CustomError(
                            "starting_cursor should be an Integer".to_string(),
                        ));
                    }
                }
            }
            // let help = m.get("help");
            // if let Some(h) = help {
            //     if let Expression::String(h_msg) = h {
            //         let hh: Cow<str> = Cow::Borrowed(h_msg); // 使用借用
            //         ans = ans.with_help_message(&hh);
            //     }
            // }
        }
        _ => {}
    }
    match ans.prompt() {
        Ok(choice) => Ok(choice),
        Err(e) => Err(LmError::CustomError(format!("ui.pick: {}", e.to_string()))),
    }
}
fn multi_select_wrapper<'a>(
    msg: &'a str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
) -> Result<Expression, LmError> {
    let mut ans = MultiSelect::new(&msg, options);
    match cfgs {
        Some(m) => {
            let page_size = m.get("page_size");
            if let Some(ps) = page_size {
                match ps {
                    Expression::Integer(size) => {
                        ans = ans.with_page_size(*size as usize);
                    }
                    _ => {
                        return Err(LmError::CustomError(
                            "page_size should be an Integer".to_string(),
                        ));
                    }
                }
            }
            let starting_cursor = m.get("starting_cursor");
            if let Some(c) = starting_cursor {
                match c {
                    Expression::Integer(c) => {
                        ans = ans.with_starting_cursor(*c as usize);
                    }
                    _ => {
                        return Err(LmError::CustomError(
                            "starting_cursor should be an Integer".to_string(),
                        ));
                    }
                }
            }
            // let help = m.get("help");
            // if let Some(h) = help {
            //     if let Expression::String(h_msg) = h {
            //         ans = ans.with_help_message(h_msg.as_str());
            //     }
            // }
        }
        _ => {}
    }
    match ans.prompt() {
        Ok(choice) => Ok(Expression::from(choice)),
        Err(e) => Err(LmError::CustomError(format!(
            "ui.multi_pick: {}",
            e.to_string()
        ))),
    }
}
fn extract_options(delimiter: &str, expr: Expression) -> Result<Vec<Expression>, LmError> {
    match expr {
        Expression::List(list) => Ok(list.as_ref().clone()),
        Expression::String(str) => Ok(str
            .split_terminator(delimiter)
            .map(|line| Expression::String(line.to_string()))
            .collect::<Vec<_>>()),
        _ => Err(LmError::CustomError(
            "pick requires a list as argument".to_string(),
        )),
    }
}
fn extract_cfg(expr: Expression) -> Result<Rc<BTreeMap<String, Expression>>, LmError> {
    match expr {
        Expression::Map(cfg) => Ok(cfg), // 返回引用
        Expression::String(msg) | Expression::Symbol(msg) => {
            // 创建一个新的 BTreeMap 并返回
            let mut map = BTreeMap::new();
            map.insert(String::from("msg"), Expression::String(msg));
            Ok(Rc::new(map))
        }
        _ => Err(LmError::CustomError(
            "cfg should be a string msg or map".to_string(),
        )),
    }
}
