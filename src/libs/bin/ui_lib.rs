use crate::{
    Environment, Expression, RuntimeError,
    libs::{
        BuiltinInfo,
        helper::{check_args_len, check_exact_args_len, get_integer_ref, get_string_ref},
        lazy_module::LazyModule,
    },
    reg_info, reg_lazy,
    runtime::{IFS_PCK, ifs_contains},
};

// Refactored ui_module
use std::collections::BTreeMap;
use std::rc::Rc;

use inquire::{Confirm, CustomType, MultiSelect, Password, PasswordDisplayMode, Select, Text};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        int, float, text, passwd, confirm, pick, multi_pick,
        widget, joinx, joiny, join_flow,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("int", args, 1, ctx)?;
    let msg = get_string_ref(&args[0], ctx)?;

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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("float", args, 1, ctx)?;
    let msg = get_string_ref(&args[0], ctx)?;

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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("text", args, 1, ctx)?;
    let msg = get_string_ref(&args[0], ctx)?;

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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("passwd", args, 1..=2, ctx)?;
    let msg = get_string_ref(&args[0], ctx)?;
    let confirm = if args.len() == 2 {
        args[1].is_truthy()
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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("confirm", args, 1, ctx)?;
    let msg = get_string_ref(&args[0], ctx)?;

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
    selector_wrapper(false, args, env, ctx)
}
fn multi_pick(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    selector_wrapper(true, args, env, ctx)
}

fn selector_wrapper(
    multi: bool,
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let ifs = env.get("IFS");
    let delimiter = match (ifs_contains(IFS_PCK, env), &ifs) {
        (true, Some(Expression::String(fs))) => fs,
        _ => "\n",
    };

    let (cfgs, options) = match args.len() {
        1 => (None, extract_options(delimiter, &args[0], ctx)?),
        2 => (
            Some(extract_cfg(&args[1], ctx)?),
            extract_options(delimiter, &args[0], ctx)?,
        ),
        3.. => (
            Some(extract_cfg(&args.last().unwrap(), ctx)?),
            args[..args.len() - 2].to_vec(),
        ),
        0 => {
            return Err(RuntimeError::common(
                "pick requires a string or list as options".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let msg = match &cfgs {
        None => "your choice:".to_string(),
        Some(m) => m
            .get("msg")
            .map(|v| v.to_string())
            .unwrap_or("your choice:".to_string()),
    };

    match multi {
        true => multi_select_wrapper(&msg, options, cfgs, ctx),
        false => single_select_wrapper(&msg, options, cfgs, ctx),
    }
}

fn single_select_wrapper(
    msg: &str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut ans = Select::new(msg, options);
    if let Some(m) = cfgs {
        let page_size = m.get("page_size");
        if let Some(ps) = page_size {
            match ps {
                Expression::Integer(size) => {
                    ans = ans.with_page_size(*size as usize);
                }
                _ => {
                    return Err(RuntimeError::common(
                        "page_size should be an Integer".into(),
                        ctx.clone(),
                        0,
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
                    return Err(RuntimeError::common(
                        "starting_cursor should be an Integer".into(),
                        ctx.clone(),
                        0,
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
    match ans.prompt() {
        Ok(choice) => Ok(choice),
        Err(e) => Err(RuntimeError::common(
            format!("ui.pick: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}
fn multi_select_wrapper(
    msg: &str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut ans = MultiSelect::new(msg, options);
    if let Some(m) = cfgs {
        let page_size = m.get("page_size");
        if let Some(ps) = page_size {
            match ps {
                Expression::Integer(size) => {
                    ans = ans.with_page_size(*size as usize);
                }
                _ => {
                    return Err(RuntimeError::common(
                        "page_size should be an Integer".into(),
                        ctx.clone(),
                        0,
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
                    return Err(RuntimeError::common(
                        "starting_cursor should be an Integer".into(),
                        ctx.clone(),
                        0,
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
    match ans.prompt() {
        Ok(choice) => Ok(Expression::from(choice)),
        Err(e) => Err(RuntimeError::common(
            format!("ui.pick: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}
fn extract_options(
    delimiter: &str,
    expr: &Expression,
    ctx: &Expression,
) -> Result<Vec<Expression>, RuntimeError> {
    match expr {
        Expression::List(list) => Ok(list.as_ref().clone()),
        Expression::String(str) => Ok(str
            .split_terminator(delimiter)
            .map(|line| Expression::String(line.to_string()))
            .collect::<Vec<_>>()),
        _ => Err(RuntimeError::common(
            "pick requires a list/string as options".into(),
            ctx.clone(),
            0,
        )),
    }
}
fn extract_cfg(
    expr: &Expression,
    ctx: &Expression,
) -> Result<Rc<BTreeMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::Map(cfg) => Ok(cfg.clone()), // 返回引用
        Expression::String(msg) | Expression::Symbol(msg) => {
            // 创建一个新的 BTreeMap 并返回
            let mut map = BTreeMap::new();
            map.insert(String::from("msg"), Expression::String(msg.to_string()));
            Ok(Rc::new(map))
        }
        _ => Err(RuntimeError::common(
            "cfg should be a string msg or map".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn widget(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // 支持2-4个参数：title, content, [width], [height]
    check_args_len("widget", args, 2..=4, ctx)?;

    let title = get_string_ref(&args[1], ctx)?;
    let title_len = title.chars().count();
    let content = get_string_ref(&args[0], ctx)?;

    // 自动计算宽度
    let auto_width = calculate_auto_width(title, content);
    let text_width = if args.len() >= 3 {
        get_integer_ref(&args[2], ctx)? as usize
    } else {
        auto_width
    } - 2;

    // 自动计算高度
    let auto_height = calculate_auto_height(&content, text_width);
    let widget_height = if args.len() >= 4 {
        get_integer_ref(&args[3], ctx)? as usize
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
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joinx", args, 2, ctx)?;

    let mut string_args = vec![];
    let mut max_height = 0;

    // 收集所有widget并找到最大高度
    for arg in args.iter() {
        let s = get_string_ref(arg, ctx)?;
        let lines = s.lines().map(ToString::to_string).collect::<Vec<String>>();
        let lines_len = lines.len();
        string_args.push(lines);
        max_height = std::cmp::max(max_height, lines_len);
    }

    // 将所有widget填充到相同高度
    for widget_lines in &mut string_args {
        while widget_lines.len() < max_height {
            let width = widget_lines
                .first()
                .map(|line| line.chars().count())
                .unwrap_or(0);
            widget_lines.push(" ".repeat(width));
        }
    }

    let mut result = String::new();
    for line_n in 0..max_height {
        for widget_lines in &string_args {
            result += &widget_lines[line_n].replace('\r', "");
        }
        result += "\n";
    }

    Ok(result.into())
}

fn joiny(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joiny", args, 2, ctx)?;

    let mut string_args = vec![];
    let mut max_width = 0;

    // 收集所有widget并找到最大宽度
    for arg in args.iter() {
        match arg {
            Expression::String(s) => {
                let trimmed = s.trim();
                let width = trimmed
                    .lines()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0);
                max_width = std::cmp::max(max_width, width);
                string_args.push(trimmed.to_string());
            }
            otherwise => {
                return Err(RuntimeError::common(
                    format!("expected string, but got {otherwise}").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    }

    // 将所有widget填充到相同宽度
    let mut padded_widgets = vec![];
    for widget in string_args {
        let padded_lines: Vec<String> = widget
            .lines()
            .map(|line| {
                let line_width = line.chars().count();
                if line_width < max_width {
                    format!("{}{}", line, " ".repeat(max_width - line_width))
                } else {
                    line.to_string()
                }
            })
            .collect();
        padded_widgets.push(padded_lines.join("\n"));
    }

    Ok(padded_widgets
        .into_iter()
        .map(|x| x.replace('\r', ""))
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn join_flow(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("join_flow", args, 2.., ctx)?;

    let max_width = match &args[0] {
        Expression::Integer(w) if *w > 0 => *w as usize,
        other => {
            return Err(RuntimeError::common(
                format!("expected positive integer for max_width, got {other}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let mut rows = vec![];
    let mut current_row = vec![];
    let mut current_width = 0;

    for arg in &args[1..] {
        match arg {
            Expression::String(widget) => {
                let widget_width = widget
                    .lines()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0);

                // 如果当前行加上新widget会超过最大宽度，则开始新行
                if !current_row.is_empty() && current_width + widget_width > max_width {
                    rows.push(current_row);
                    current_row = vec![];
                    current_width = 0;
                }

                current_row.push(widget.to_string());
                current_width += widget_width;
            }
            otherwise => {
                return Err(RuntimeError::common(
                    format!("expected string widget, got {otherwise}").into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    }

    if !current_row.is_empty() {
        rows.push(current_row);
    }

    // 将每行的widgets水平连接，然后将所有行垂直连接
    let mut result_rows = vec![];
    for row in rows {
        if row.len() == 1 {
            result_rows.push(row[0].clone());
        } else {
            // 使用现有的joinx逻辑
            let row_expressions: Vec<Expression> =
                row.into_iter().map(Expression::String).collect();
            match joinx(&row_expressions, env, ctx)? {
                Expression::String(joined) => result_rows.push(joined),
                _ => return Err(RuntimeError::common("joinx failed".into(), ctx.clone(), 0)),
            }
        }
    }

    Ok(result_rows.join("\n").into())
}
