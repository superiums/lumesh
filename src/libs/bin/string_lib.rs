use std::collections::HashMap;

use crate::{
    Environment, Expression, Int, RuntimeError, RuntimeErrorKind,
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
use std::sync::LazyLock;

// use crate::libs::bin::into_module::strip_str;

use crate::libs::pprint::pretty_printer;
use crate::runtime::{IFS_STR, ifs_contains};
use common_macros::hash_map;
use regex_lite::Regex;

static COLOR_MAP: LazyLock<HashMap<&'static str, (i64, i64, i64)>> =
    LazyLock::new(|| init_color_map());

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // pprint,
        // 转换
        // to_int, to_float, to_filesize, to_time, to_table,
        // 基础检查
        is_empty, is_whitespace, is_alpha, is_alphanumeric, is_numeric, is_lower, is_upper, is_title, len,
        // 子串检查
        starts_with, ends_with, contains,
        // 分割操作
        split, split_at, chars, words, words_quoted, lines, paragraphs, concat,
        // 修改操作
        repeat, replace, substring, remove_prefix, remove_suffix, trim, trim_start, trim_end, to_lower, to_upper, to_title,
        // 高级操作
        get_width, grep,
        // caesar,
        // strip,
        // 格式化
        pad_start, pad_end, center, wrap, format,
        // 样式
        href, bold, faint, italics, underline, blink, invert, strike,
        // 标准颜色
        black, red, green, yellow, blue, magenta, cyan, white,
        // 高级颜色
        color256, color256_bg, color, color_bg, colors,
    })
}

pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({
        pprint => "convert to table and pretty print", "[headers|header...]"

        // 转换
        to_int => "convert a float or string to an int", "<value>"
        to_float => "convert an int or string to a float", "<value>"
        to_filesize => "parse a string representing a file size into bytes", "<size_str>"
        to_time => "convert a string to a datetime", "<datetime_str> [datetime_template]"
        to_table => "convert third-party command output to a table", "<command_output>"

        // 基础检查
        is_empty => "is this string empty?", "<string>"
        is_whitespace => "is this string whitespace?", "<string>"
        is_alpha => "is this string alphabetic?", "<string>"
        is_alphanumeric => "is this string alphanumeric?", "<string>"
        is_numeric => "is this string numeric?", "<string>"
        is_lower => "is this string lowercase?", "<string>"
        is_upper => "is this string uppercase?", "<string>"
        is_title => "is this string title case?", "<string>"
        len => "get length of string", "<string>"

        // 子串检查
        starts_with => "check if a string starts with a given substring", "<substring> <string>"
        ends_with => "check if a string ends with a given substring", "<substring> <string>"
        contains => "check if a string contains a given substring", "<substring> <string>"

        // 分割操作
        split => "split a string on a given character", "[delimiter] <string>"
        split_at => "split a string at a given index", "<index> <string>"
        chars => "split a string into characters", "<string>"
        words => "split a string into words", "<string>"
        words_quoted => "split a string into words,quoted as one", "<string>"
        lines => "split a string into lines", "<string>"
        paragraphs => "split a string into paragraphs", "<string>"
        concat => "concat strings", "<string>..."

        // 修改操作
        repeat => "repeat string specified number of times", "<count> <string>"
        replace => "replace all instances of a substring", "<old> <new> <string>"
        substring => "get substring from start to end indices", "<start> <end> <string>"
        remove_prefix => "remove prefix if present", "<prefix> <string>"
        remove_suffix => "remove suffix if present", "<suffix> <string>"
        trim => "trim whitespace from a string", "<string>"
        trim_start => "trim whitespace from the start", "<string>"
        trim_end => "trim whitespace from the end", "<string>"
        to_lower => "convert a string to lowercase", "<string>"
        to_upper => "convert a string to uppercase", "<string>"
        to_title => "convert a string to title case", "<string>"

        // 高级操作
        caesar => "encrypt a string using a caesar cipher", "<shift> <string>"
        get_width => "get the width of a string", "<string>"
        grep => "find lines which contains the substring", "<substring> <string>"
        strip => "remove all ANSI escape codes from string", "<string>"

        // 格式化
        pad_start => "pad string to specified length at start", "<length> [pad_char] <string>"
        pad_end => "pad string to specified length at end", "<length> [pad_char] <string>"
        center => "center string by padding both ends", "<length> [pad_char] <string>"
        wrap => "wrap text to fit in specific number of columns", "<width> <string>"
        format => "format string using {} placeholders", "<format_string> <args>..."

        // 样式
        href => "create terminal hyperlink", "<url> <text>"
        bold => "apply bold styling", "<string>"
        faint => "apply faint/dim styling", "<string>"
        italics => "apply italic styling", "<string>"
        underline => "apply underline styling", "<string>"
        blink => "apply blinking effect", "<string>"
        invert => "invert foreground/background colors", "<string>"
        strike => "apply strikethrough styling", "<string>"

        // 标准颜色
        black => "apply black foreground", "<string>"
        red => "apply red foreground", "<string>"
        green => "apply green foreground", "<string>"
        yellow => "apply yellow foreground", "<string>"
        blue => "apply blue foreground", "<string>"
        magenta => "apply magenta foreground", "<string>"
        cyan => "apply cyan foreground", "<string>"
        white => "apply white foreground", "<string>"

        // 高级颜色
        color256 => "apply color using 256-color code", "<color_spec> <string>"
        color256_bg => "apply background color using 256-color code", "<color_spec> <string>"
        color => "apply true color using RGB values or color_name", "<hex_color|color_name|r,g,b> <string>"
        color_bg => "apply True Color background using RGB values or color_name", "<hex_color|color_name|r,g,b> <string>"
        colors => "list all color_name for True Color", "[skip_colorized?]"
    })
}

fn init_color_map() -> HashMap<&'static str, (i64, i64, i64)> {
    hash_map! {
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "cyan" => (0, 255, 255),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkgrey" => (169, 169, 169),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" => (255, 0, 255),
        "gainsboro" => (221, 221, 221),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" => (128, 128, 128),
        "green" => (0, 255, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 245, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightgrey" => (211, 211, 211),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "magenta" => (255, 0, 255),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 209, 209),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (183, 105, 224),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 150),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (238, 130, 238),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 128),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 255, 50),
    }
}

// Basic Check Functions
fn is_empty(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_empty", args, 1, ctx)?;
    let text = get_exact_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.is_empty()))
}

fn is_whitespace(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_whitespace", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_whitespace())))
}

fn is_alpha(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_alpha", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_alphabetic())))
}

fn is_alphanumeric(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_alphanumeric", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(
        text.chars().all(|c| c.is_alphanumeric()),
    ))
}

fn is_numeric(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_numeric", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_numeric())))
}

fn is_lower(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_lower", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_lowercase())))
}

fn is_upper(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_upper", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::Boolean(text.chars().all(|c| c.is_uppercase())))
}

fn is_title(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("is_title", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    let title = to_title(&[args[0].clone()], env, ctx)?;
    Ok(Expression::Boolean(text == title.to_string()))
}

fn len(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("len", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => {
            Ok(Expression::Integer(x.chars().count() as Int))
        }
        otherwise => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "String".into(),
                found: otherwise.type_name(),
                sym: otherwise.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}
// Substring Check Functions
fn starts_with(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("starts_with", args, 2, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.starts_with(prefix)))
}

fn ends_with(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ends_with", args, 2, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.ends_with(suffix)))
}

fn contains(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("contains", args, 2, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [substring, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::Boolean(text.contains(substring)))
}
// Splitting Operations
fn split(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("split", args, 1..=2, ctx)?;

    let string_args = get_string_args(args, env, ctx)?;
    let text = string_args.last().unwrap().to_owned();

    let ifs = env.get("IFS");
    let delimiter = if args.len() > 1 {
        string_args.first()
    } else {
        match (
            crate::runtime::ifs_contains(crate::runtime::IFS_STR, env),
            &ifs,
        ) {
            (true, Some(Expression::String(fs))) => Some(fs),
            _ => None,
        }
    };

    let parts: Vec<Expression> = match delimiter {
        Some(sep) => text
            .split(sep)
            .map(|s| Expression::String(s.to_string()))
            .collect(),
        _ => text
            .split_whitespace()
            .map(|s| Expression::String(s.to_string()))
            .collect(),
    };

    Ok(Expression::from(parts))
}

fn split_at(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("split_at", args, 2, ctx)?;
    let text = get_string_arg(args[1].eval(env)?, ctx)?;
    let index = get_integer_arg(args[0].eval(env)?, ctx)? as usize;

    if index > text.len() {
        return Ok(Expression::from(vec![
            Expression::String(text.clone()),
            Expression::String(String::new()),
        ]));
    }

    let (left, right) = text.split_at(index);
    Ok(Expression::from(vec![
        Expression::String(left.to_string()),
        Expression::String(right.to_string()),
    ]))
}

fn chars(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("chars", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let chars = text
        .chars()
        .map(|c| Expression::String(c.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(chars))
}

fn words(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("words", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let words = text
        .split_whitespace()
        .map(|word| Expression::String(word.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(words))
}

fn words_quoted(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("words_quoted", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let re = regex_lite::Regex::new(r#""((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|(\S+)"#).unwrap();
    let words = re
        .captures_iter(&text)
        .filter_map(|cap| {
            cap.get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .map(|m| Expression::String(m.as_str().to_string()))
        })
        .collect::<Vec<Expression>>();

    Ok(Expression::from(words))
}

fn lines(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("lines", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let lines = text
        .lines()
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

fn paragraphs(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("paragraphs", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let paragraphs = text
        .split("\n\n")
        .map(|para| Expression::String(para.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(paragraphs))
}
// Modification Operations
fn concat(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("concat", args, 2.., ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    let others = args[1..]
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .concat();

    Ok(Expression::from(text + &others))
}

fn repeat(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("repeat", args, 2, ctx)?;
    let count = get_integer_arg(args[0].eval(env)?, ctx)?;
    let text = get_string_arg(args[1].eval(env)?, ctx)?;

    Ok(Expression::String(text.repeat(count.max(0) as usize)))
}

fn replace(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("replace", args, 3, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [from, to, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(text.replace(from, to)))
}

fn substring(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("substring", args, 2..=3, ctx)?;
    let text = get_string_arg(args.last().unwrap().eval(env)?, ctx)?;

    let start = get_integer_arg(args[0].eval(env)?, ctx)?;
    let start_idx = if start < 0 {
        (text.len() as i64 + start).max(0) as usize
    } else {
        start.min(text.len() as i64) as usize
    };

    let end_idx = if args.len() == 3 {
        let end = get_integer_arg(args[1].eval(env)?, ctx)?;
        if end < 0 {
            (text.len() as i64 + end).max(0) as usize
        } else {
            end.min(text.len() as i64) as usize
        }
    } else {
        text.len()
    };

    if start_idx >= end_idx || start_idx >= text.len() {
        return Ok(Expression::String(String::new()));
    }

    let result: String = text
        .chars()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect();
    Ok(Expression::String(result))
}

fn remove_prefix(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove_prefix", args, 2, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [prefix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_prefix(prefix).unwrap_or(text).to_string(),
    ))
}

fn remove_suffix(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("remove_suffix", args, 2, ctx)?;
    let string_args = get_string_args(args, env, ctx)?;
    let [suffix, text] = string_args.as_slice() else {
        unreachable!()
    };

    Ok(Expression::String(
        text.strip_suffix(suffix).unwrap_or(text).to_string(),
    ))
}

fn trim(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::String(text.trim().to_string()))
}

fn trim_start(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim_start", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::String(text.trim_start().to_string()))
}

fn trim_end(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trim_end", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::String(text.trim_end().to_string()))
}

fn to_lower(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_lower", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::String(text.to_lowercase()))
}

fn to_upper(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_upper", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    Ok(Expression::String(text.to_uppercase()))
}

fn to_title(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("to_title", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let mut title = String::with_capacity(text.len());
    let mut capitalize = true;

    for c in text.chars() {
        if capitalize {
            title.extend(c.to_uppercase());
            capitalize = false;
        } else {
            title.extend(c.to_lowercase());
        }
        if c.is_whitespace() {
            capitalize = true;
        }
    }
    Ok(Expression::String(title))
}

// Formatting Operations (continued)
fn pad_start(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pad_start", args, 2..=3, ctx)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };
    let s_val = match args.last().unwrap().eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(RuntimeError::common(
                "pad_start requires a string as last argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(RuntimeError::common(
                "pad_start requires an integer as length".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    pad_start_impl(len, pad_ch, s_val)
}

fn pad_end(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("pad_end", args, 2..=3, ctx)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };

    let s_val = match args.last().unwrap().eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(RuntimeError::common(
                "pad_end requires a string as last argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(RuntimeError::common(
                "pad_end requires an integer as length".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    pad_end_impl(len, pad_ch, s_val)
}

fn center(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("center", args, 2..=3, ctx)?;
    let (length, pad_char) = match args.len() {
        2 => (args[0].clone(), " ".to_string()),
        3 => (args[0].clone(), args[1].clone().to_string()),
        _ => unreachable!(),
    };

    let s_val = match args.last().unwrap().eval(env)? {
        Expression::Symbol(x) | Expression::String(x) => x,
        _ => {
            return Err(RuntimeError::common(
                "center requires a string as last argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let len = match length.eval(env)? {
        Expression::Integer(n) => n.max(0) as usize,
        _ => {
            return Err(RuntimeError::common(
                "center requires an integer as length".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    if s_val.len() >= len {
        return Ok(Expression::String(s_val));
    }

    let pad_ch = pad_char.chars().next().unwrap_or(' ');
    center_impl(len, pad_ch, s_val)
}

fn wrap(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("wrap", args, 2, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(columns) => {
            Ok(textwrap::fill(&args[1].eval(env)?.to_string(), columns as usize).into())
        }
        otherwise => Err(RuntimeError::common(
            format!("expected number of columns in wrap, but got `{otherwise}`").into(),
            ctx.clone(),
            0,
        )),
    }
}
// Format Function
fn format(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("format", args, 1.., ctx)?;

    let format_str = match args[0].eval(env)? {
        Expression::String(s) => s,
        _ => {
            return Err(RuntimeError::common(
                "format requires format string as first argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let mut result = format_str;
    for (i, arg) in args.iter().enumerate().skip(1) {
        let value = arg.eval(env)?.to_string();
        result = result.replace(&format!("{{{}}}", i - 1), &value);
    }

    Ok(Expression::String(result))
}
// Style Functions
fn href(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("href", args, 2, ctx)?;
    let url = args[0].eval(env)?.to_string();
    let text = args[1].eval(env)?.to_string();
    Ok(format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text).into())
}

fn bold(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bold", args, 1, ctx)?;
    Ok(format!("\x1b[1m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn faint(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("faint", args, 1, ctx)?;
    Ok(format!("\x1b[2m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn italics(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("italics", args, 1, ctx)?;
    Ok(format!("\x1b[3m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn underline(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("underline", args, 1, ctx)?;
    Ok(format!("\x1b[4m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn blink(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("blink", args, 1, ctx)?;
    Ok(format!("\x1b[5m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn invert(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("invert", args, 1, ctx)?;
    Ok(format!("\x1b[7m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn strike(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("strike", args, 1, ctx)?;
    Ok(format!("\x1b[9m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}
// Standard Color Functions
fn black(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("black", args, 1, ctx)?;
    Ok(format!("\x1b[90m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn red(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("red", args, 1, ctx)?;
    Ok(format!("\x1b[91m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn green(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("green", args, 1, ctx)?;
    Ok(format!("\x1b[92m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn yellow(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("yellow", args, 1, ctx)?;
    Ok(format!("\x1b[93m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn blue(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("blue", args, 1, ctx)?;
    Ok(format!("\x1b[94m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn magenta(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("magenta", args, 1, ctx)?;
    Ok(format!("\x1b[95m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn cyan(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cyan", args, 1, ctx)?;
    Ok(format!("\x1b[96m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}

fn white(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("white", args, 1, ctx)?;
    Ok(format!("\x1b[97m{}\x1b[m\x1b[0m", args[0].eval(env)?).into())
}
// Advanced Color Functions
fn color256(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("color256", args, 2, ctx)?;
    let color = args[0].eval(env)?.to_string();
    let text = args[1].eval(env)?.to_string();
    Ok(format!("\x1b[38;5;{}m{}\x1b[m\x1b[0m", color, text).into())
}

fn color256_bg(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("color256_bg", args, 2, ctx)?;
    let color = args[0].eval(env)?.to_string();
    let text = args[1].eval(env)?.to_string();
    Ok(format!("\x1b[48;5;{}m{}\x1b[m\x1b[0m", color, text).into())
}

fn color(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    true_color(args, false, env, ctx)
}

fn color_bg(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    true_color(args, true, env, ctx)
}

fn colors(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    if args.len() > 0 && !args[0].eval(env)?.is_truthy() {
        Ok(Expression::from(
            COLOR_MAP
                .iter()
                .map(|(&k, _)| Expression::String(k.to_owned()))
                .collect::<Vec<_>>(),
        ))
    } else {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();

        for (i, (text, (r, g, b))) in COLOR_MAP.iter().enumerate() {
            let pad_len = 20 - text.len();
            let padding: String = std::iter::repeat_n(" ", pad_len).collect();
            write!(
                &mut stdout,
                "\x1b[48;2;{r};{g};{b}m     \x1b[m\x1b[0m \x1b[38;2;{r};{g};{b}m{text}\x1b[m\x1b[0m{padding}"
            ).unwrap();
            if i % 3 == 2 {
                writeln!(&mut stdout, "\n").unwrap();
            }
        }
        writeln!(&mut stdout).unwrap();
        Ok(Expression::None)
    }
}
// Helper Implementation Functions
fn pad_start_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{padding}{s}")))
}

fn pad_end_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    if s.len() >= len {
        return Ok(Expression::String(s));
    }
    let pad_len = len - s.len();
    let padding: String = std::iter::repeat_n(pad_ch, pad_len).collect();
    Ok(Expression::String(format!("{s}{padding}")))
}

fn center_impl(len: usize, pad_ch: char, s: String) -> Result<Expression, RuntimeError> {
    let total_pad = len - s.len();
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    let left: String = std::iter::repeat_n(pad_ch, left_pad).collect();
    let right: String = std::iter::repeat_n(pad_ch, right_pad).collect();
    Ok(Expression::String(format!("{left}{s}{right}")))
}

fn true_color(
    args: &[Expression],
    is_bg: bool,
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("true_color", args, 2, ctx)?;
    let color_spec = args[0].eval(env)?.to_string();
    let text = args[1].eval(env)?.to_string();

    let color_code = if let Some((r, g, b)) = COLOR_MAP.get(&color_spec.as_str()) {
        format!("{};{};{}", r, g, b)
    } else if color_spec.starts_with('#') {
        // Parse hex color
        let hex = color_spec.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            format!("{};{};{}", r, g, b)
        } else {
            return Err(RuntimeError::common(
                "invalid hex color format".into(),
                ctx.clone(),
                0,
            ));
        }
    } else {
        // Parse RGB values
        let parts: Vec<&str> = color_spec.split(',').collect();
        if parts.len() == 3 {
            format!("{}", parts.join(";"))
        } else {
            return Err(RuntimeError::common(
                "invalid color format, expected hex or r,g,b".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let prefix = if is_bg { "48" } else { "38" };
    Ok(format!("\x1b[{};2;{}m{}\x1b[m\x1b[0m", prefix, color_code, text).into())
}

// Additional Functions
fn caesar_cipher(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("caesar", args, 1..=2, ctx)?;

    let text = get_string_arg(args[0].eval(env)?, ctx)?;
    let shift = if args.len() > 1 {
        get_integer_arg(args[1].eval(env)?, ctx)?
    } else {
        13
    };

    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
            let offset = (c as u8 - base) as i64;
            let shifted = ((offset + shift).rem_euclid(26) as u8 + base) as char;
            result.push(shifted);
        } else {
            result.push(c);
        }
    }
    Ok(Expression::String(result))
}

fn get_width(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("get_width", args, 1, ctx)?;
    let text = get_string_arg(args[0].eval(env)?, ctx)?;

    let max_width = text.lines().map(|line| line.len()).max().unwrap_or(0);

    Ok(Expression::Integer(max_width as Int))
}

fn grep(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("grep", args, 2, ctx)?;
    let pat = get_string_arg(args[0].eval(env)?, ctx)?;
    let text = get_string_arg(args[1].eval(env)?, ctx)?;

    let lines = text
        .lines()
        .filter(|x| x.contains(&pat))
        .map(|line| Expression::String(line.to_string()))
        .collect::<Vec<Expression>>();
    Ok(Expression::from(lines))
}

// fn table_pprint(
//     args: &[Expression],
//     env: &mut Environment,
//     ctx: &Expression,
// ) -> Result<Expression, RuntimeError> {
//     check_args_len("pprint", args, 1.., ctx)?;
//     let table = from_module::parse_command_output(args, env, ctx)?;
//     pprint::pretty_printer(&table)
// }
