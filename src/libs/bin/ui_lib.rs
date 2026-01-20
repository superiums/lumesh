use crate::{
    Environment, Expression, Int, RuntimeError,
    libs::{
        BuiltinInfo,
        helper::{
            check_args_len, check_exact_args_len, get_exact_string_arg, get_integer_arg,
            get_string_arg, get_string_args,
        },
        lazy_module::LazyModule,
    },
    reg_info, reg_lazy,
};

// Refactored ui_module
use std::{collections::HashMap, rc::Rc};

use inquire::{Confirm, CustomType, MultiSelect, Password, PasswordDisplayMode, Select, Text};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        int, float, text, passwd, confirm, pick, multi_pick,
        widget, joinx, joiny, join_flow,
    })
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({
        int => "read an int from input", "<msg>"
        float => "read a float from input", "<msg>"
        text => "read a text input ", "<msg>"
        passwd => "read a passwd input", "<msg> [confirm?]"
        confirm => "ask user to confirm", "<msg>"
        pick => "select one from list/string", "[msg|cfg_map] <list|items...>"
        multi_pick => "select multi from list/string", "[msg|cfg_map] <list|items...>"

        widget => "create a text widget","<title> <content> [width] [height]"
        joinx => "join two widgets horizontally","<widget1> <widget2>"
        joiny => "join two widgets vertically","<widget1> <widget2>"
        join_flow => "join widgets with flow layout","<max_width> <widgets...>"
    })
}

fn int(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("int", args, 1, ctx)?;
    let msg = get_string_arg(args[0].eval(env)?, ctx)?;

    let amount = CustomType::<i64>::new(msg.as_str())
        .with_formatter(&|i| format!("${i:.0}"))
        .with_error_message("Please type a valid int")
        .with_help_message("Type an Integer");

    match amount.prompt() {
        Ok(s) => Ok(Expression::Integer(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.int: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn float(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("float", args, 1, ctx)?;
    let msg = get_string_arg(args[0].eval(env)?, ctx)?;

    let amount = CustomType::<f64>::new(msg.as_str())
        .with_formatter(&|i| format!("${i:.2}"))
        .with_error_message("Please type a valid number")
        .with_help_message("Type the amount using a decimal point as a separator");

    match amount.prompt() {
        Ok(s) => Ok(Expression::Float(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.float: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn text(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("text", args, 1, ctx)?;
    let msg = get_string_arg(args[0].eval(env)?, ctx)?;

    let ans = Text::new(msg.as_str());
    match ans.prompt() {
        Ok(s) => Ok(Expression::String(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.text: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn passwd(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("passwd", args, 1..=2, ctx)?;
    let msg = get_string_arg(args[0].eval(env)?, ctx)?;
    let confirm = if args.len() == 2 {
        args[1].eval(env)?.is_truthy()
    } else {
        false
    };

    let mut ans = Password::new(msg.as_str()).with_display_mode(PasswordDisplayMode::Masked);
    if !confirm {
        ans = ans.without_confirmation();
    }
    match ans.prompt() {
        Ok(s) => Ok(Expression::String(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.passwd: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn confirm(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("confirm", args, 1, ctx)?;
    let msg = get_string_arg(args[0].eval(env)?, ctx)?;

    match Confirm::new(msg.as_str()).prompt() {
        Ok(s) => Ok(Expression::Boolean(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.confirm: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn pick(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pick", args, 1.., ctx)?;

    let (msg, options) = if args.len() == 1 {
        ("".to_string(), args[0].clone())
    } else {
        (get_string_arg(args[0].eval(env)?, ctx)?, args[1].clone())
    };

    let options = match options.eval(env)? {
        Expression::String(s) => s
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        Expression::List(l) => l.as_ref().iter().map(|e| e.to_string()).collect::<Vec<_>>(),
        _ => {
            return Err(RuntimeError::common(
                "pick requires a string or list as options".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    match Select::new(&msg, options).prompt() {
        Ok(s) => Ok(Expression::String(s)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.pick: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn multi_pick(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("multi_pick", args, 1.., ctx)?;

    let (msg, options) = if args.len() == 1 {
        ("".to_string(), args[0].clone())
    } else {
        (get_string_arg(args[0].eval(env)?, ctx)?, args[1].clone())
    };

    let options = match options.eval(env)? {
        Expression::String(s) => s
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        Expression::List(l) => l.as_ref().iter().map(|e| e.to_string()).collect::<Vec<_>>(),
        _ => {
            return Err(RuntimeError::common(
                "multi_pick requires a string or list as options".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    match MultiSelect::new(&msg, options).prompt() {
        Ok(s) => Ok(Expression::List(Rc::new(
            s.into_iter().map(Expression::String).collect(),
        ))),
        Err(e) => Err(RuntimeError::common(
            format!("ui.multi_pick: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn widget(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // 支持2-4个参数：title, content, [width], [height]
    check_args_len("widget", args, 2..=4, ctx)?;

    let title = args[0].eval(env)?.to_string();
    let title_len = title.chars().count();
    let content = args[1].eval(env)?.to_string();

    // 自动计算宽度
    let auto_width = calculate_auto_width(&title, &content);
    let text_width = if args.len() >= 3 {
        match args[2].eval(env)? {
            Expression::Integer(n) if n > 4 => n as usize,
            _ => {
                return Err(RuntimeError::common(
                    "widget width must be an integer".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    } else {
        auto_width
    } - 2;

    // 自动计算高度
    let auto_height = calculate_auto_height(&content, text_width);
    let widget_height = if args.len() >= 4 {
        match args[3].eval(env)? {
            Expression::Integer(n) if n >= 3 => n as usize,
            otherwise => {
                return Err(RuntimeError::common(
                    "widget height must be an integer".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    } else {
        auto_height
    };

    let format_width = text_width * 2 / 3;
    let text = textwrap::fill(&format!("{content:format_width$}"), text_width);

    if text_width < title_len {
        return Err(RuntimeError::common(
            "width is less than title length".into(),
            ctx.clone(),
            0,
        ));
    }

    let mut left_border_half = "─".repeat(((text_width - title_len) as f64 / 2.0).round() as usize);
    let right_border_half = left_border_half.clone();
    let left_len = left_border_half.chars().count();
    if (left_len * 2 + title_len + 2) > text_width + 2 {
        left_border_half.pop();
    }

    let mut result = format!("┌{left_border_half}{title}{right_border_half}┐\n");
    let width = result.chars().count() - 1;

    let mut lines = 1;
    let mut i = 0;
    for ch in text.replace('\r', "").chars() {
        if i == 0 {
            result.push(' ');
            i += 1;
        }

        if ch == '\n' {
            lines += 1;
            result += &" ".repeat(width - i);
            i = width;
        } else {
            result.push(ch);
        }

        if lines == widget_height - 1 {
            break;
        }

        if i >= width - 1 {
            result += "\n";
            i = 0;
        } else {
            i += 1;
        }
    }

    result += &" ".repeat(width - i);

    while result.lines().count() < widget_height - 1 {
        result += "\n";
        result += &" ".repeat(width);
    }

    result += &format!(
        "\n└{left_side}{}{right_side}┘",
        "─".repeat(title_len),
        left_side = left_border_half,
        right_side = right_border_half
    );

    Ok(result.into())
}

// 计算自动宽度
fn calculate_auto_width(title: &str, content: &str) -> usize {
    let title_len = title.chars().count();
    let content_lines = content.lines();
    let max_content_width = content_lines
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    // 取标题长度和内容最大行宽度的较大值，加上边框和内边距
    std::cmp::max(title_len + 4, max_content_width + 4)
}

// 计算自动高度
fn calculate_auto_height(content: &str, text_width: usize) -> usize {
    let wrapped_content = textwrap::fill(content, text_width);
    let content_lines = wrapped_content.lines().count();
    // 加上标题行和底部边框
    content_lines + 3
}

fn joinx(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joinx", args, 2, ctx)?;

    let widget1 = args[0].eval(env)?;
    let widget2 = args[1].eval(env)?;

    // Horizontal join implementation would go here
    Ok(Expression::String(format!(
        "Horizontal join: {} + {}",
        widget1, widget2
    )))
}

fn joiny(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joiny", args, 2, ctx)?;

    let widget1 = args[0].eval(env)?;
    let widget2 = args[1].eval(env)?;

    // Vertical join implementation would go here
    Ok(Expression::String(format!(
        "Vertical join: {} + {}",
        widget1, widget2
    )))
}

fn join_flow(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("join_flow", args, 2.., ctx)?;

    let max_width = match args[0].eval(env)? {
        Expression::Integer(w) => w as usize,
        _ => {
            return Err(RuntimeError::common(
                "join_flow max_width must be an integer".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let widgets: Vec<String> = args[1..]
        .iter()
        .map(|w| w.eval(env).map(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    // Flow layout implementation would go here
    Ok(Expression::String(format!(
        "Flow layout (width {}): {:?}",
        max_width, widgets
    )))
}
