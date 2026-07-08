use crate::{
    Environment, Expression, RuntimeError,
    expression::eval2::{glob_expand, ifs_split},
    libs::{
        BuiltinInfo,
        helper::{check_args_len, check_exact_args_len, get_integer_ref, get_string_ref},
        lazy_module::LazyModule,
    },
    reg_info, reg_lazy,
    utils::expand_home,
};

// Refactored ui_module
use std::collections::BTreeMap;
use std::rc::Rc;

use chrono::{NaiveDate, Weekday};
use inquire::{
    Confirm, CustomType, DateSelect, MultiSelect, Password, PasswordDisplayMode, Select, Text,
};
use inquire::{list_option::ListOption, validator::Validation};
pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        int, float, text, passwd, confirm, pick, multi_pick, date_pick, editor,
        widget, joinx, joiny, join_flow,
    })
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        int => "read an int from input", "<msg>"
        float => "read a float from input", "<msg> [decimal_places]"        text => "read a text input ", "<msg>"
        passwd => "read a passwd input", "<msg> [confirm?]"
        confirm => "ask user to confirm", "<msg>"
        pick => "select one from list/string", "<list|items...> [msg|cfg_map]"
        multi_pick => "select multi from list/string", "<list|items...> [msg|cfg_map]"
        date_pick => "pick a date from calendar", "[msg|cfg_map]"
        editor => "open editor for multiline text input", "[msg|cfg_map]"

        widget => "create a text widget","<content> <title> [width] [height]"
        joinx => "join two widgets horizontally","<widget1> <widget2>"
        joiny => "join two widgets vertically","<widget1> <widget2>"
        join_flow => "join widgets with flow layout","<max_width> <widgets...>"

    })
}

macro_rules! apply_select_cfg {
    ($ans:ident, $cfgs:expr, $fns:expr, $ctx:expr) => {
        if let Some(m) = &$cfgs {
            if let Some(v) = cfg_get_usize(m, "page_size", $ctx)? {
                $ans = $ans.with_page_size(v);
            }
            if let Some(v) = cfg_get_usize(m, "starting_cursor", $ctx)? {
                $ans = $ans.with_starting_cursor(v);
            }
            if let Some(v) = cfg_get_bool(m, "vim_mode") {
                $ans = $ans.with_vim_mode(v);
            }
            if let Some(v) = cfg_get_bool(m, "reset_cursor") {
                $ans = $ans.with_reset_cursor(v);
            }
            if let Some(false) = cfg_get_bool(m, "filter_input_enabled") {
                $ans = $ans.without_filtering();
            }
            if let Some(v) = cfg_get_str(m, "help_message") {
                $ans = $ans.with_help_message(v);
            }
            if let Some(v) = cfg_get_str(m, "starting_filter_input") {
                $ans = $ans.with_starting_filter_input(v);
            }

            if let Some(ref f) = $fns.scorer {
                $ans = $ans.with_scorer(f.as_ref());
            }
        }
    };
}

fn int(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("int", &args, 1, ctx)?;
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("float", &args, 1..=2, ctx)?; // Allow 1-2 arguments
    let msg = get_string_ref(&args[0], ctx)?;

    // Get decimal places from second argument, default to 2
    let decimal_places = if args.len() == 2 {
        match &args[1] {
            Expression::Integer(n) => *n as usize,
            _ => {
                return Err(RuntimeError::common(
                    "decimal places must be an integer".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    } else {
        2 // Default to 2 decimal places
    };

    // Create dynamic format string
    let aformatter = &|i| format!("{:.*}", decimal_places, i);
    let hmsg = format!("with {} digits of precision", decimal_places);
    let amount = CustomType::<f64>::new(msg.as_str())
        .with_formatter(aformatter)
        .with_error_message("Please type a valid number")
        .with_help_message(&hmsg);

    match amount.prompt() {
        Ok(s) => {
            // 四舍五入到指定小数位
            let rounded = if decimal_places > 0 {
                let factor = 10_f64.powi(decimal_places as i32);
                (s * factor).round() / factor
            } else {
                s.round()
            };
            Ok(Expression::Float(rounded))
        }
        Err(e) => Err(RuntimeError::common(
            format!("ui.float: {e}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn text(
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("text", &args, 1, ctx)?;
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("passwd", &args, 1..=2, ctx)?;
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("confirm", &args, 1, ctx)?;
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
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    selector_wrapper(false, args, env, ctx)
}
fn multi_pick(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    selector_wrapper(true, args, env, ctx)
}

fn selector_wrapper(
    multi: bool,
    mut args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let (options, cfgs) = match args.len() {
        1 => (
            extract_options(args.into_iter().next().unwrap(), env, ctx)?,
            None,
        ),
        2 => {
            let mut it = args.into_iter();
            (
                extract_options(it.next().unwrap(), env, ctx)?,
                Some(extract_cfg(it.next().unwrap(), ctx)?),
            )
        }
        3.. => {
            let last = args.split_off(args.len() - 1);
            (
                args.to_vec(),
                Some(extract_cfg(last.into_iter().next().unwrap(), ctx)?),
            )
        }
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
        true => multi_select_wrapper(&msg, options, cfgs, env, ctx),
        false => single_select_wrapper(&msg, options, cfgs, env, ctx),
    }
}

fn single_select_wrapper(
    msg: &str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let fns = build_select_fns(&cfgs, env, ctx)?;
    let mut ans = Select::new(msg, options);
    apply_select_cfg!(ans, cfgs, fns, ctx);
    if let Some(ref f) = fns.formatter {
        ans = ans.with_formatter(f.as_ref());
    }
    if let Some(ref f) = fns.sorter {
        ans = ans.with_sorter(f.as_ref());
    }
    ans.prompt()
        .map_err(|e| RuntimeError::common(format!("ui.pick: {e}").into(), ctx.clone(), 0))
}

fn multi_select_wrapper(
    msg: &str,
    options: Vec<Expression>,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let fns = build_select_fns(&cfgs, env, ctx)?;
    let mut ans = MultiSelect::new(msg, options);
    apply_select_cfg!(ans, cfgs, fns, ctx);

    if let Some(m) = &cfgs {
        if let Some(true) = cfg_get_bool(m, "all_selected_by_default") {
            ans = ans.with_all_selected_by_default();
        }
        if let Some(v) = cfg_get_bool(m, "keep_filter") {
            ans = ans.with_keep_filter(v);
        }

        // validator 只属于 MultiSelect，直接传具体闭包，不经过 Box<dyn ...>
        if let Some(func) = cfg_get_lambda(&cfgs, "validator", ctx)? {
            let call = make_caller(func, env); // Clone + 'static
            ans = ans.with_validator(move |input: &[ListOption<&Expression>]| {
                let args = vec![Expression::from(
                    input
                        .iter()
                        .map(|item| item.value.clone())
                        .collect::<Vec<_>>(),
                )];
                match call(args) {
                    Some(Expression::Boolean(true)) => Ok(Validation::Valid),
                    Some(Expression::Boolean(false)) => {
                        Ok(Validation::Invalid("Invalid selection".into()))
                    }
                    Some(Expression::String(msg)) => Ok(Validation::Invalid(msg.into())),
                    _ => Ok(Validation::Valid),
                }
            });
        }
    }
    ans.prompt()
        .map(Expression::from)
        .map_err(|e| RuntimeError::common(format!("ui.pick: {e}").into(), ctx.clone(), 0))
}

fn extract_options(
    expr: Expression,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Vec<Expression>, RuntimeError> {
    match expr {
        Expression::List(list) => Ok(list.as_ref().clone()),
        Expression::BSet(bset) => Ok(bset.iter().cloned().collect()),
        Expression::Range(range, step) => Ok(range.step_by(step).map(Expression::from).collect()),
        Expression::Symbol(str) if str.contains('*') => {
            let s = expand_home(str.as_ref());

            let owned_items: Vec<Expression> = glob_expand(&s)
                .into_iter()
                .map(Expression::String)
                .collect();
            Ok(owned_items)
        }
        Expression::String(str) => {
            let s = expand_home(str.as_ref());

            let owned_items: Vec<Expression> = ifs_split(&s, env)
                .into_iter()
                .map(Expression::String)
                .collect();
            Ok(owned_items)
        }
        Expression::Map(map) => Ok(map
            .keys()
            .map(|k| Expression::from(k.clone()))
            .collect::<Vec<_>>()),
        Expression::HMap(map) => Ok(map
            .keys()
            .map(|k| Expression::from(k.clone()))
            .collect::<Vec<_>>()),
        Expression::Table(table) => {
            use std::collections::BTreeMap;

            let rows: Vec<Expression> = table
                .rows()
                .iter()
                .map(|row| {
                    let map: BTreeMap<String, Expression> = table
                        .headers()
                        .iter()
                        .enumerate()
                        .map(|(i, header)| {
                            let value = row.get(i).cloned().unwrap_or(Expression::None);
                            (header.clone(), value)
                        })
                        .collect();
                    Expression::from(map)
                })
                .collect();
            Ok(rows)
        }
        // Expression::Table(table) => Ok(table
        //     .rows()
        //     .iter()
        //     .map(|row| Expression::from(row))
        //     .collect::<Vec<_>>()),
        _ => Err(RuntimeError::common(
            "pick requires a list/set/range/table/glob/string as options".into(),
            ctx.clone(),
            0,
        )),
    }
}
fn extract_cfg(
    expr: Expression,
    ctx: &Expression,
) -> Result<Rc<BTreeMap<String, Expression>>, RuntimeError> {
    match expr {
        Expression::Map(cfg) => Ok(cfg), // 返回引用
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // 支持2-4个参数：title, content, [width], [height]
    check_args_len("widget", &args, 2..=4, ctx)?;

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
    let auto_height = calculate_auto_height(content, text_width);
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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joinx", &args, 2, ctx)?;

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
    args: Vec<Expression>,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("joiny", &args, 2, ctx)?;

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
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("join_flow", &args, 2.., ctx)?;

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
            result_rows.push(row.into_iter().next().unwrap())
        } else {
            // 使用现有的joinx逻辑
            let row_expressions: Vec<Expression> =
                row.into_iter().map(Expression::String).collect();
            match joinx(row_expressions, env, ctx)? {
                Expression::String(joined) => result_rows.push(joined),
                _ => return Err(RuntimeError::common("joinx failed".into(), ctx.clone(), 0)),
            }
        }
    }

    Ok(result_rows.join("\n").into())
}

// 辅助函数
fn cfg_get_lambda(
    cfgs: &Option<Rc<BTreeMap<String, Expression>>>,
    key: &str,
    ctx: &Expression,
) -> Result<Option<Expression>, RuntimeError> {
    match cfgs {
        Some(m) => match m.get(key) {
            None => Ok(None),
            Some(expr @ (Expression::Lambda(..) | Expression::Function(..))) => {
                Ok(Some(expr.clone()))
            }
            _ => Err(RuntimeError::common(
                format!("{key} should be a lambda").into(),
                ctx.clone(),
                0,
            )),
        },
        _ => Ok(None),
    }
}

fn make_caller(
    func: Expression,
    env: &Environment,
) -> impl Fn(Vec<Expression>) -> Option<Expression> + 'static + Clone {
    use std::cell::RefCell;
    let env_cell = RefCell::new(env.clone());
    move |args: Vec<Expression>| {
        let mut state = crate::eval::State::new();
        let mut env_ref = env_cell.borrow_mut();
        func.eval_apply(&func, &args, &mut state, &mut *env_ref, 0)
            .ok()
    }
}

fn cfg_get_usize(
    m: &BTreeMap<String, Expression>,
    key: &str,
    ctx: &Expression,
) -> Result<Option<usize>, RuntimeError> {
    match m.get(key) {
        None => Ok(None),
        Some(Expression::Integer(n)) => Ok(Some(*n as usize)),
        _ => Err(RuntimeError::common(
            format!("{key} should be an Integer").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn cfg_get_bool(m: &BTreeMap<String, Expression>, key: &str) -> Option<bool> {
    match m.get(key) {
        Some(v) => Some(v.is_truthy()),
        None => None,
    }
}

fn cfg_get_str<'a>(m: &'a BTreeMap<String, Expression>, key: &str) -> Option<&'a str> {
    match m.get(key) {
        Some(Expression::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

struct SelectFns {
    formatter: Option<Box<dyn Fn(ListOption<&Expression>) -> String>>,
    scorer: Option<Box<dyn Fn(&str, &Expression, &str, usize) -> Option<i64>>>,
    sorter: Option<Box<dyn Fn(&mut [(usize, i64)])>>,
}

fn build_select_fns(
    cfgs: &Option<Rc<BTreeMap<String, Expression>>>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<SelectFns, RuntimeError> {
    Ok(SelectFns {
        formatter: if let Some(func) = cfg_get_lambda(cfgs, "formatter", ctx)? {
            let call = make_caller(func, env);
            Some(Box::new(move |i: ListOption<&Expression>| {
                match call(vec![Expression::Integer(i.index as i64), i.value.clone()]) {
                    Some(Expression::String(s)) => s,
                    _ => format!("[Option {}]: {}", i.index + 1, i.value),
                }
            }))
        } else {
            None
        },

        scorer: if let Some(func) = cfg_get_lambda(cfgs, "scorer", ctx)? {
            let call = make_caller(func, env);
            Some(Box::new(
                move |input: &str, opt: &Expression, _: &str, _: usize| match call(vec![
                    Expression::String(input.to_string()),
                    opt.clone(),
                ]) {
                    Some(Expression::Integer(n)) => Some(n),
                    _ => None,
                },
            ))
        } else {
            None
        },

        sorter: if let Some(func) = cfg_get_lambda(cfgs, "sorter", ctx)? {
            let call = make_caller(func, env);
            Some(Box::new(move |items: &mut [(usize, i64)]| {
                items.sort_by(|a, b| {
                    match call(vec![Expression::Integer(a.1), Expression::Integer(b.1)]) {
                        Some(Expression::Integer(n)) if n > 0 => std::cmp::Ordering::Greater,
                        Some(Expression::Integer(0)) => std::cmp::Ordering::Equal,
                        _ => std::cmp::Ordering::Less,
                    }
                });
            }))
        } else {
            None
        },
    })
}

/// data pick
/// 关键设计说明：
///
/// cfg key	类型	说明
/// msg	String	提示语
/// starting_date	DateTime / "YYYY-MM-DD"	初始光标日期
/// min_date / max_date	DateTime / "YYYY-MM-DD"	可选范围
/// week_start	Integer 0-6 或 "Mon" 等	每周起始日
/// vim_mode	Boolean	hjkl 导航
/// help_message	String	底部帮助文字
/// formatter	Lambda |dt| ...	接收 DateTime，返回 String
fn date_pick(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("date_pick", &args, 0..=1, ctx)?;

    let cfgs = if args.is_empty() {
        None
    } else {
        Some(extract_cfg(args.into_iter().next().unwrap(), ctx)?)
    };

    let msg = match &cfgs {
        None => "select a date:".to_string(),
        Some(m) => m
            .get("msg")
            .map(|v| v.to_string())
            .unwrap_or("select a date:".to_string()),
    };

    date_select_wrapper(&msg, cfgs, env, ctx)
}

fn date_select_wrapper(
    msg: &str,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    // formatter 需在顶层声明，生命周期覆盖 ans.prompt()
    // DateFormatter<'a> = &'a dyn Fn(NaiveDate) -> String，NaiveDate 是 owned，无 HRTB 问题
    let formatter_fn: Option<Box<dyn Fn(NaiveDate) -> String>> =
        if let Some(func) = cfg_get_lambda(&cfgs, "formatter", ctx)? {
            let call = make_caller(func, env);
            Some(Box::new(move |date: NaiveDate| {
                let dt = date.and_hms_opt(0, 0, 0).unwrap();
                match call(vec![Expression::DateTime(dt)]) {
                    Some(Expression::String(s)) => s,
                    _ => date.format("%Y-%m-%d").to_string(),
                }
            }))
        } else {
            None
        };

    let mut ans = DateSelect::new(msg);

    if let Some(m) = &cfgs {
        if let Some(v) = cfg_get_naive_date(m, "starting_date", ctx)? {
            ans = ans.with_starting_date(v);
        }
        if let Some(v) = cfg_get_naive_date(m, "min_date", ctx)? {
            ans = ans.with_min_date(v);
        }
        if let Some(v) = cfg_get_naive_date(m, "max_date", ctx)? {
            ans = ans.with_max_date(v);
        }
        if let Some(v) = cfg_get_weekday(m, "week_start", ctx)? {
            ans = ans.with_week_start(v);
        }
        if let Some(v) = cfg_get_str(m, "help_message") {
            ans = ans.with_help_message(v);
        }
        if let Some(ref f) = formatter_fn {
            ans = ans.with_formatter(f.as_ref());
        }
    }

    // validator：NaiveDate 是 owned，无 HRTB，直接传具体闭包
    if let Some(func) = cfg_get_lambda(&cfgs, "validator", ctx)? {
        let call = make_caller(func, env); // Clone + 'static
        ans = ans.with_validator(move |date: NaiveDate| {
            let dt = date.and_hms_opt(0, 0, 0).unwrap();
            match call(vec![Expression::DateTime(dt)]) {
                Some(Expression::Boolean(true)) => Ok(Validation::Valid),
                Some(Expression::Boolean(false)) => Ok(Validation::Invalid("Invalid date".into())),
                Some(Expression::String(msg)) => Ok(Validation::Invalid(msg.into())),
                _ => Ok(Validation::Valid),
            }
        });
    }

    ans.prompt()
        .map(|date| Expression::DateTime(date.and_hms_opt(0, 0, 0).unwrap()))
        .map_err(|e| RuntimeError::common(format!("ui.date_pick: {e}").into(), ctx.clone(), 0))
}

fn cfg_get_naive_date(
    m: &BTreeMap<String, Expression>,
    key: &str,
    ctx: &Expression,
) -> Result<Option<NaiveDate>, RuntimeError> {
    match m.get(key) {
        None => Ok(None),
        Some(Expression::DateTime(dt)) => Ok(Some(dt.date())),
        Some(Expression::String(s)) => {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| {
                    RuntimeError::common(
                        format!("{key} should be a date string YYYY-MM-DD or DateTime").into(),
                        ctx.clone(),
                        0,
                    )
                })
        }
        _ => Err(RuntimeError::common(
            format!("{key} should be a date string or DateTime").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn cfg_get_weekday(
    m: &BTreeMap<String, Expression>,
    key: &str,
    ctx: &Expression,
) -> Result<Option<Weekday>, RuntimeError> {
    match m.get(key) {
        None => Ok(None),
        Some(Expression::Integer(n)) => Ok(Some(match n % 7 {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            _ => Weekday::Sun,
        })),
        Some(Expression::String(s)) => {
            use std::str::FromStr;
            Weekday::from_str(s).map(Some).map_err(|_| {
                RuntimeError::common(
                    format!("{key} should be weekday name (Mon/Tue/...) or integer 0-6").into(),
                    ctx.clone(),
                    0,
                )
            })
        }
        _ => Err(RuntimeError::common(
            format!("{key} should be a weekday").into(),
            ctx.clone(),
            0,
        )),
    }
}

///editor
///
fn editor(
    args: Vec<Expression>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("editor", &args, 0..=1, ctx)?;

    let cfgs = if args.is_empty() {
        None
    } else {
        Some(extract_cfg(args.into_iter().next().unwrap(), ctx)?)
    };

    let msg = match &cfgs {
        None => "edit:".to_string(),
        Some(m) => m
            .get("msg")
            .map(|v| v.to_string())
            .unwrap_or("edit:".to_string()),
    };

    editor_wrapper(&msg, cfgs, env, ctx)
}

fn editor_wrapper(
    msg: &str,
    cfgs: Option<Rc<BTreeMap<String, Expression>>>,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    use inquire::Editor;
    use inquire::validator::Validation;
    use std::ffi::OsStr;

    // 1. formatter 在顶层声明（生命周期覆盖 ans.prompt()）
    //    StringFormatter<'a> = &'a dyn Fn(&str) -> String，需要 HRTB
    let formatter_fn: Option<Box<dyn for<'s> Fn(&'s str) -> String>> =
        if let Some(func) = cfg_get_lambda(&cfgs, "formatter", ctx)? {
            let call = make_caller(func, env);
            // 显式类型标注强制 HRTB 推导
            let f: Box<dyn for<'s> Fn(&'s str) -> String> =
                Box::new(
                    move |s: &str| match call(vec![Expression::String(s.to_string())]) {
                        Some(Expression::String(r)) => r,
                        _ => s.to_string(),
                    },
                );
            Some(f)
        } else {
            None
        };

    // 2. editor_command / editor_command_args：
    //    owned String 在顶层声明，确保 &OsStr 引用在 ans.prompt() 前有效
    let editor_cmd_str: Option<String> = cfgs.as_ref().and_then(|m| {
        if let Some(Expression::String(s)) = m.get("editor_command") {
            Some(s.clone())
        } else {
            None
        }
    });
    let editor_args_strs: Vec<String> = cfgs
        .as_ref()
        .and_then(|m| m.get("editor_command_args"))
        .and_then(|v| {
            if let Expression::List(list) = v {
                Some(
                    list.iter()
                        .filter_map(|e| {
                            if let Expression::String(s) = e {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();
    // 必须在 editor_args_strs 之后声明，确保借用有效
    let editor_args_refs: Vec<&OsStr> = editor_args_strs
        .iter()
        .map(|s| OsStr::new(s.as_str()))
        .collect();

    let mut ans = Editor::new(msg);

    if let Some(m) = &cfgs {
        if let Some(v) = cfg_get_str(m, "help_message") {
            ans = ans.with_help_message(v);
        }
        if let Some(v) = cfg_get_str(m, "predefined_text") {
            ans = ans.with_predefined_text(v);
        }
        if let Some(v) = cfg_get_str(m, "file_extension") {
            ans = ans.with_file_extension(v);
        }

        if let Some(ref cmd) = editor_cmd_str {
            ans = ans.with_editor_command(OsStr::new(cmd.as_str()));
            if !editor_args_refs.is_empty() {
                ans = ans.with_args(&editor_args_refs);
            }
        }

        if let Some(ref f) = formatter_fn {
            ans = ans.with_formatter(f.as_ref());
        }
    }

    // 3. validators：接受单个 lambda 或 lambda 列表
    //    StringValidator blanket impl: Fn(&str) -> Result<Validation, CustomUserError> + Clone
    //    直接传具体闭包（不用 Box<dyn StringValidator>，原因同 MultiSelect validator）
    let validator_lambdas: Vec<Expression> = cfgs
        .as_ref()
        .and_then(|m| m.get("validators"))
        .map(|v| match v {
            Expression::List(list) => list
                .iter()
                .filter(|e| matches!(e, Expression::Lambda(..) | Expression::Function(..)))
                .cloned()
                .collect(),
            expr @ (Expression::Lambda(..) | Expression::Function(..)) => vec![expr.clone()],
            _ => vec![],
        })
        .unwrap_or_default();

    for func in validator_lambdas {
        let call = make_caller(func, env); // Clone + 'static
        ans = ans.with_validator(move |input: &str| {
            match call(vec![Expression::String(input.to_string())]) {
                Some(Expression::Boolean(true)) => Ok(Validation::Valid),
                Some(Expression::Boolean(false)) => Ok(Validation::Invalid("Invalid input".into())),
                Some(Expression::String(msg)) => Ok(Validation::Invalid(msg.into())),
                _ => Ok(Validation::Valid),
            }
        });
    }

    ans.prompt()
        .map(Expression::String)
        .map_err(|e| RuntimeError::common(format!("ui.editor: {e}").into(), ctx.clone(), 0))
}
