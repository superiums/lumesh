// 复用现有的高亮逻辑

use common_macros::hash_map;
use std::collections::{BTreeMap, HashMap};

use crate::{Diagnostic, Expression, TokenKind, libs::get_builtin_optimized, tokenize};

const DEFAULT: &str = "";

pub fn highlight_dark_theme(line: &str) -> String {
    highlight(line, &get_dark_theme())
}
pub fn highlight(line: &str, theme: &HashMap<String, String>) -> String {
    let (tokens, diagnostics) = tokenize(line);

    let mut result = String::new();
    let mut is_colored = false;

    for (token, diagnostic) in tokens.iter().zip(&diagnostics) {
        match (token.kind, token.range.to_str(line)) {
            (TokenKind::ValueSymbol, b) => {
                result.push_str(
                    theme
                        .get("value_symbol")
                        .unwrap_or(&"".to_string())
                        .as_str(),
                );
                is_colored = true;
                result.push_str(b);
            }
            // (
            //     TokenKind::Punctuation,
            //     o @ ("@" | "\'" | "=" | "|" | ">>" | "<<" | ">!" | "->" | "~>"),
            // ) => {
            //     result.push_str(get_color("punctuation_special",theme,&default));
            //     is_colored = true;
            //     result.push_str(o);
            // }
            (TokenKind::Punctuation, o) => {
                result.push_str(get_color("punctuation", theme));
                is_colored = true;
                result.push_str(o);
            }
            (TokenKind::Keyword, k) => {
                result.push_str(get_color("keyword", theme));
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::Operator, k) => {
                result.push_str(get_color("operator", theme));
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPrefix, k) => {
                result.push_str(get_color("operator_prefix", theme));
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorInfix, k) => {
                result.push_str(get_color("operator_infix", theme));
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPostfix, k) => {
                result.push_str(get_color("operator_postfix", theme));
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::StringRaw, s) => {
                result.push_str(get_color("string_raw", theme));
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringTemplate, s) => {
                result.push_str(get_color("string_template", theme));
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringLiteral, s) => {
                result.push_str(get_color("string_literal", theme));
                is_colored = true;

                if let Diagnostic::InvalidStringEscapes(ranges) = diagnostic {
                    let mut last_end = token.range.start();

                    for &range in ranges.iter() {
                        result.push_str(&line[last_end..range.start()]);
                        result.push_str(get_color("string_error", theme));
                        result.push_str(range.to_str(line));
                        result.push_str(get_color("string_literal", theme));
                        last_end = range.end();
                    }

                    result.push_str(&line[last_end..token.range.end()]);
                } else {
                    result.push_str(s);
                }
            }
            (TokenKind::IntegerLiteral | TokenKind::FloatLiteral, l) => {
                if let Diagnostic::InvalidNumber(e) = diagnostic {
                    result.push_str(get_color("number_error", theme));
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if is_colored {
                        result.push_str(get_color("reset", theme));
                        is_colored = false;
                    }
                    result.push_str(l);
                }
            }
            (TokenKind::Symbol, l) => {
                if let Diagnostic::IllegalChar(e) = diagnostic {
                    result.push_str(get_color("string_error", theme));
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if get_builtin_optimized("", l).is_some() {
                        // if matches!(l, "echo" | "exit" | "clear" | "cd" | "rm") {
                        result.push_str(get_color("builtin_cmd", theme));
                        is_colored = true;
                    } else if is_colored {
                        result.push_str(get_color("reset", theme));
                        is_colored = false;
                    }

                    result.push_str(l);
                }
            }
            (TokenKind::Whitespace, w) => {
                result.push_str(w);
            }
            (TokenKind::LineBreak, w) => {
                result.push_str(w);
            }
            (TokenKind::Comment, w) => {
                result.push_str(get_color("comment", theme));
                is_colored = true;
                result.push_str(w);
            }
            (TokenKind::Regex, s) => {
                result.push_str(get_color("regex", theme));
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::Time, s) => {
                result.push_str(get_color("time", theme));
                is_colored = true;
                result.push_str(s);
            }
        }
    }

    if diagnostics.len() > tokens.len() {
        for diagnostic in &diagnostics[tokens.len()..] {
            if let Diagnostic::NotTokenized(e) = diagnostic {
                result.push_str(get_color("string_error", theme));
                result.push_str(e.to_str(line));
                is_colored = true;
            }
        }
    }

    if is_colored {
        result.push_str(get_color("reset", theme));
    }

    result
}

fn get_color<'a>(color: &str, theme: &'a HashMap<String, String>) -> &'a str {
    match theme.get(color) {
        Some(c) => c.as_str(),
        _ => DEFAULT,
    }
}

pub fn get_merged_theme(
    mut base: HashMap<String, String>,
    modify: &BTreeMap<String, Expression>,
) -> HashMap<String, String> {
    for (k, v) in modify {
        if let Expression::String(vs) = v {
            base.insert(k.clone(), vs.clone());
        }
    }
    base
}

pub fn get_dark_theme() -> HashMap<String, String> {
    hash_map! {
        // 基础颜色
        String::from("reset") => "\x1b[m\x1b[0m".to_string(),
        String::from("bold") => "\x1b[1m".to_string(),
        String::from("dim") => "\x1b[2m".to_string(),

        // One Dark 核心语法颜色 - 每种都使用不同的颜色
        String::from("keyword") => "\x1b[38;5;170m".to_string(),           // 紫色 (#C678DD)
        String::from("value_symbol") => "\x1b[38;5;141m".to_string(),      // 淡紫色
        String::from("operator") => "\x1b[38;5;67m".to_string(),           // 蓝灰色 (#56B6C2)
        String::from("operator_prefix") => "\x1b[38;5;73m".to_string(),    // 青蓝色
        String::from("operator_infix") => "\x1b[38;5;80m".to_string(),     // 深青色
        String::from("operator_postfix") => "\x1b[38;5;87m".to_string(),   // 亮青色

        // 字符串相关颜色
        String::from("string_raw") => "\x1b[38;5;114m".to_string(),        // 绿色 (#98C379)
        String::from("string_template") => "\x1b[38;5;120m".to_string(),   // 亮绿色
        String::from("string_literal") => "\x1b[38;5;107m".to_string(),    // 橄榄绿
        String::from("string_error") => "\x1b[38;5;204m".to_string(),      // 红色 (#E06C75)

        // 数字和字面量
        String::from("number_literal") => "\x1b[38;5;209m".to_string(),    // 橙色 (#D19A66)
        String::from("number_error") => "\x1b[38;5;196m".to_string(),      // 亮红色
        String::from("integer_literal") => "\x1b[38;5;215m".to_string(),   // 金橙色
        String::from("float_literal") => "\x1b[38;5;221m".to_string(),     // 黄色

        // 符号和标识符
        String::from("symbol_none") => "\x1b[38;5;203m".to_string(),       // 粉红色
        String::from("builtin_cmd") => "\x1b[38;5;75m".to_string(),        // 蓝色 (#61AFEF)
        String::from("symbol") => "\x1b[38;5;81m".to_string(),             // 天蓝色

        // 注释和标点
        String::from("comment") => "\x1b[38;5;59m".to_string(),             // 灰色 (#5C6370)
        String::from("punctuation") => "\x1b[38;5;117m".to_string(), // 淡蓝色
        // String::from("punctuation_special") => "\x1b[38;5;145m".to_string(),        // 中灰色

        // REPL 和交互相关
        String::from("command_valid") => "\x1b[38;5;120m\x1b[1m".to_string(), // 绿色加粗
        String::from("hint") => "\x1b[38;5;102m".to_string(),               // 深灰色
        String::from("completion_cmd") => "\x1b[38;5;244m".to_string(),     // 浅灰色
        String::from("completion_ai") => "\x1b[38;5;111m".to_string(),      // 浅蓝色

        // 新增：Time token 颜色
        // String::from("time_literal") => "\x1b[38;5;180m".to_string(),      // 金黄色 (#E5C07B)
        // String::from("time_format") => "\x1b[38;5;186m".to_string(),       // 淡黄色
        String::from("time") => "\x1b[38;5;173m".to_string(),     // 棕黄色

        // 新增：Regex token 颜色
        // String::from("regex_pattern") => "\x1b[38;5;167m".to_string(),     // 玫瑰红
        // String::from("regex_flags") => "\x1b[38;5;176m".to_string(),       // 淡紫红
        String::from("regex") => "\x1b[38;5;183m".to_string(),      // 淡粉色
        // String::from("regex_group") => "\x1b[38;5;139m".to_string(),       // 紫灰色

        // 错误和警告
        // String::from("error") => "\x1b[38;5;196m".to_string(),             // 亮红色
        // String::from("warning") => "\x1b[38;5;214m".to_string(),           // 橙黄色
        // String::from("info") => "\x1b[38;5;117m".to_string(),              // 信息蓝

        // 特殊语法元素
        // String::from("whitespace") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("line_break") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("illegal_char") => "\x1b[38;5;160m".to_string(),      // 深红色
    }
}
pub fn get_ayu_dark_theme() -> HashMap<String, String> {
    hash_map! {
        // 基础颜色
        String::from("reset") => "\x1b[m\x1b[0m".to_string(),
        String::from("bold") => "\x1b[1m".to_string(),
        String::from("dim") => "\x1b[2m".to_string(),

        // Ayu Dark 核心语法颜色 - 基于 #0A0E14 背景
        String::from("keyword") => "\x1b[38;5;173m".to_string(),           // 橙色 (#FF8F40)
        String::from("value_symbol") => "\x1b[38;5;179m".to_string(),      // 淡橙色
        String::from("operator") => "\x1b[38;5;67m".to_string(),           // 青色 (#39BAE6)
        String::from("operator_prefix") => "\x1b[38;5;74m".to_string(),    // 亮青色
        String::from("operator_infix") => "\x1b[38;5;81m".to_string(),     // 天蓝色
        String::from("operator_postfix") => "\x1b[38;5;87m".to_string(),   // 浅青色

        // 字符串相关颜色
        String::from("string_raw") => "\x1b[38;5;107m".to_string(),        // 绿色 (#AAD94C)
        String::from("string_template") => "\x1b[38;5;113m".to_string(),   // 亮绿色
        String::from("string_literal") => "\x1b[38;5;114m".to_string(),    // 草绿色
        String::from("string_error") => "\x1b[38;5;203m".to_string(),      // 红色 (#F07178)

        // 数字和字面量
        String::from("number_literal") => "\x1b[38;5;215m".to_string(),    // 黄色 (#FFEE99)
        String::from("number_error") => "\x1b[38;5;196m".to_string(),      // 亮红色
        String::from("integer_literal") => "\x1b[38;5;221m".to_string(),   // 金黄色
        String::from("float_literal") => "\x1b[38;5;228m".to_string(),     // 淡黄色

        // 符号和标识符
        String::from("symbol_none") => "\x1b[38;5;204m".to_string(),       // 粉红色
        String::from("builtin_cmd") => "\x1b[38;5;111m".to_string(),       // 蓝色 (#59C2FF)
        String::from("symbol") => "\x1b[38;5;117m".to_string(),            // 淡蓝色

        // 注释和标点
        String::from("comment") => "\x1b[38;5;102m".to_string(),           // 灰色 (#5C6773)
        // String::from("punctuation_special") => "\x1b[38;5;180m".to_string(), // 金色
        String::from("punctuation") => "\x1b[38;5;145m".to_string(),       // 中灰色

        // REPL 和交互相关
        String::from("command_valid") => "\x1b[38;5;107m\x1b[1m".to_string(), // 绿色加粗
        String::from("hint") => "\x1b[38;5;59m".to_string(),               // 深灰色
        String::from("completion_cmd") => "\x1b[38;5;244m".to_string(),    // 浅灰色
        String::from("completion_ai") => "\x1b[38;5;75m".to_string(),      // 浅蓝色

        // Time token 颜色
        // String::from("time_literal") => "\x1b[38;5;186m".to_string(),      // 淡黄色
        // String::from("time_format") => "\x1b[38;5;192m".to_string(),       // 浅黄绿色
        String::from("time") => "\x1b[38;5;179m".to_string(),     // 棕橙色

        // Regex token 颜色
        // String::from("regex_pattern") => "\x1b[38;5;176m".to_string(),     // 紫红色
        // String::from("regex_flags") => "\x1b[38;5;183m".to_string(),       // 淡紫色
        String::from("regex") => "\x1b[38;5;139m".to_string(),      // 紫灰色
        // String::from("regex_group") => "\x1b[38;5;146m".to_string(),       // 灰紫色

        // 错误和警告
        // String::from("error") => "\x1b[38;5;196m".to_string(),             // 亮红色
        // String::from("warning") => "\x1b[38;5;214m".to_string(),           // 橙黄色
        // String::from("info") => "\x1b[38;5;117m".to_string(),              // 信息蓝

        // 特殊语法元素
        // String::from("whitespace") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("line_break") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("illegal_char") => "\x1b[38;5;160m".to_string(),      // 深红色
    }
}

pub fn get_light_theme() -> HashMap<String, String> {
    hash_map! {
        // 基础颜色
        String::from("reset") => "\x1b[m\x1b[0m".to_string(),
        String::from("bold") => "\x1b[1m".to_string(),
        String::from("dim") => "\x1b[2m".to_string(),

        // Ayu Light 核心语法颜色 - 基于 #FAFAFA 背景
        String::from("keyword") => "\x1b[38;5;166m".to_string(),           // 深橙色 (#FA8D3E)
        String::from("value_symbol") => "\x1b[38;5;172m".to_string(),      // 棕橙色
        String::from("operator") => "\x1b[38;5;25m".to_string(),           // 深蓝色 (#4CBF99)
        String::from("operator_prefix") => "\x1b[38;5;31m".to_string(),    // 深青色
        String::from("operator_infix") => "\x1b[38;5;37m".to_string(),     // 青绿色
        String::from("operator_postfix") => "\x1b[38;5;43m".to_string(),   // 绿青色

        // 字符串相关颜色
        String::from("string_raw") => "\x1b[38;5;64m".to_string(),         // 深绿色 (#86B300)
        String::from("string_template") => "\x1b[38;5;70m".to_string(),    // 草绿色
        String::from("string_literal") => "\x1b[38;5;76m".to_string(),     // 亮绿色
        String::from("string_error") => "\x1b[38;5;124m".to_string(),      // 深红色 (#F51818)

        // 数字和字面量
        String::from("number_literal") => "\x1b[38;5;130m".to_string(),    // 深黄色 (#A37ACC)
        String::from("number_error") => "\x1b[38;5;160m".to_string(),      // 深红色
        String::from("integer_literal") => "\x1b[38;5;136m".to_string(),   // 棕黄色
        String::from("float_literal") => "\x1b[38;5;142m".to_string(),     // 橄榄色

        // 符号和标识符
        String::from("symbol_none") => "\x1b[38;5;125m".to_string(),       // 深粉色
        String::from("builtin_cmd") => "\x1b[38;5;26m".to_string(),        // 深蓝色 (#399EE6)
        String::from("symbol") => "\x1b[38;5;32m".to_string(),             // 深青色

        // 注释和标点
        String::from("comment") => "\x1b[38;5;244m".to_string(),           // 中灰色 (#ABB0B6)
        // String::from("punctuation_special") => "\x1b[38;5;130m".to_string(), // 深黄色
        String::from("punctuation") => "\x1b[38;5;240m".to_string(),       // 深灰色

        // REPL 和交互相关
        String::from("command_valid") => "\x1b[38;5;64m\x1b[1m".to_string(), // 深绿色加粗
        String::from("hint") => "\x1b[38;5;244m".to_string(),              // 中灰色
        String::from("completion_cmd") => "\x1b[38;5;240m".to_string(),    // 深灰色
        String::from("completion_ai") => "\x1b[38;5;26m".to_string(),      // 深蓝色

        // Time token 颜色
        // String::from("time_literal") => "\x1b[38;5;136m".to_string(),      // 棕黄色
        // String::from("time_format") => "\x1b[38;5;142m".to_string(),       // 橄榄色
        String::from("time") => "\x1b[38;5;148m".to_string(),     // 浅橄榄色

        // Regex token 颜色
        // String::from("regex_pattern") => "\x1b[38;5;125m".to_string(),     // 深紫色
        // String::from("regex_flags") => "\x1b[38;5;131m".to_string(),       // 深紫红色
        String::from("regex") => "\x1b[38;5;137m".to_string(),      // 棕紫色
        // String::from("regex_group") => "\x1b[38;5;143m".to_string(),       // 浅棕色

        // 错误和警告
        // String::from("error") => "\x1b[38;5;160m".to_string(),             // 深红色
        // String::from("warning") => "\x1b[38;5;166m".to_string(),           // 深橙色
        // String::from("info") => "\x1b[38;5;32m".to_string(),               // 深青色

        // 特殊语法元素
        // String::from("whitespace") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("line_break") => "\x1b[0m".to_string(),               // 无颜色
        // String::from("illegal_char") => "\x1b[38;5;124m".to_string(),      // 深红色
    }
}
