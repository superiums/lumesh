use std::collections::{BTreeMap, HashMap};
use tabled::{
    Table, Tabled,
    builder::Builder,
    settings::{
        Color, Modify, Style, Width,
        object::{Columns, Rows},
    },
};

use crate::{Expression};

use regex_lite::Regex;
pub fn strip_ansi_escapes(text: &str) -> String {
    // 更全面的正则表达式，匹配大多数常见的 ANSI 转义序列
    let ansi_escape_pattern = Regex::new(r"(?:\\x1b[@-_]|[\x80-\x9F])[0-?]*[ -/]*[@-~]").unwrap();
    ansi_escape_pattern.replace_all(text, "").into_owned()
    // (?:\\x1b[@-_]|[\x80-\x9F]):

    // (?: ... )：这是一个非捕获组，表示匹配其中的内容但不捕获它。
    // \\x1b[@-_]：匹配 \x1b 后面跟着 @ 到 _ 的字符。\x1b 是 ASCII 中的 ESC 字符（即转义字符），表示 ANSI 转义序列的开始。
    // |：逻辑或操作符，表示匹配左边或右边的内容。
    // [\x80-\x9F]：匹配从 \x80 到 \x9F 的字符范围。这些字符也是 ANSI 转义序列的一部分。
    // [0-?]*:

    // [0-?]：匹配从 0 到 ? 的字符范围。? 是 ASCII 中的一个特殊字符。
    // *：表示前面的字符范围可以出现零次或多次。
    // [ -/]*:

    // [ -/]：匹配从空格到 / 的字符范围。
    // *：表示前面的字符范围可以出现零次或多次。
    // [@-~]:

    // [@-~]：匹配从 @ 到 ~ 的字符范围。
    // 这个范围包括了常见的控制字符，如 A-Z, a-z, 0-9, 和一些符号。
}

pub fn pretty_printer(arg: &Expression) -> Result<Expression, crate::LmError> {
    match arg {
        Expression::Map(exprs) => println!("{}", pprint_map(exprs.as_ref())),
        Expression::HMap(exprs) => println!("{}", pprint_hmap(exprs.as_ref())),
        Expression::List(exprs) => pprint_list(exprs.as_ref()),
        _ => {
            println!("{arg}");
        }
    }
    Ok(Expression::None)
}

#[derive(Tabled, PartialOrd, PartialEq, Eq)]
struct KeyValueRow {
    #[tabled(rename = "KEY")]
    key: String,
    #[tabled(rename = "VALUE")]
    value: String,
}
impl Ord for KeyValueRow {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}
fn pprint_map_internal<I>(items: I, is_hmap: bool) -> Table
where
    I: Iterator<Item = (String, Expression)>,
{
    // 共同的格式化逻辑
    let specified_width = crossterm::terminal::size().unwrap_or((120, 0)).0 as usize;
    // 为表格预留边框和间距
    let table_padding = 10; // 边框 + 列间距
    let available_width = specified_width.saturating_sub(table_padding);

    // 为 KEY 列预留固定宽度，剩余给 VALUE 列
    let key_column_width = 12; // 或者动态计算最长键名
    let value_column_width = available_width.saturating_sub(key_column_width);

    let mut rows: Vec<KeyValueRow> = items
        .map(|(key, val)| {
            let value = match &val {
                Expression::HMap(_) | Expression::Map(_) => {
                    format!("{val:value_column_width$}")
                }
                Expression::List(_) => {
                    let formatted = format!("{val}");
                    textwrap::fill(&formatted, value_column_width)
                }
                // Expression::Builtin(Builtin { help, .. }) => {
                //     format!(
                //         "{}\n{}",
                //         val,
                //         textwrap::fill(help.as_str(), value_column_width)
                //     )
                // }
                _ => {
                    let formatted = format!("{val}");
                    textwrap::fill(&formatted, value_column_width)
                }
            };
            KeyValueRow {
                key: key.clone(),
                value,
            }
        })
        .collect();
    if is_hmap {
        rows.sort();
    }
    let mut table = Table::new(rows);
    if is_hmap {
        table
            .modify(Columns::first(), Color::FG_BLUE)
            .modify(Columns::first(), Width::increase(key_column_width))
            .with(Style::ascii());
    } else {
        table
            .modify(Columns::first(), Color::FG_GREEN)
            .with(Style::rounded());
    }

    table.with(Width::wrap(specified_width).keep_words(true));
    table
}

fn pprint_map(exprs: &BTreeMap<String, Expression>) -> Table {
    pprint_map_internal(exprs.iter().map(|(k, v)| (k.clone(), v.clone())), false)
}

pub fn pprint_hmap(exprs: &HashMap<String, Expression>) -> Table {
    pprint_map_internal(exprs.iter().map(|(k, v)| (k.clone(), v.clone())), true)
}

fn pprint_list(exprs: &[Expression]) {
    let specified_width = crossterm::terminal::size().unwrap_or((120, 0)).0 as usize;

    let (rows, heads_opt) = TableRow {
        rows: exprs,
        max_width: specified_width,
        col_padding: 5,
    }
    .split_into_rows();

    if rows.is_empty() {
        return;
    }
    let mut builder;

    let has_header = match heads_opt {
        Some(heads) => {
            builder = Builder::with_capacity(rows.len(), heads.len());
            builder.insert_record(0, heads);
            true
        }
        _ => {
            builder = Builder::with_capacity(rows.len(), rows[0].len());

            false
        }
    };
    for row in rows {
        builder.push_record(row);
    }

    // builder.insert_record(0, (0..Y).map(|i| i.to_string()));
    // builder.insert_column(0, once(String::new()).chain((0..X).map(|i| i.to_string())));
    let mut table = builder.build();

    table
        .modify(Rows::first(), Color::FG_BLUE)
        .with(Width::wrap(specified_width).keep_words(true));

    if has_header {
        table
            .with(
                Modify::new(Rows::first()).with(tabled::settings::format::Format::content(|s| {
                    s.to_uppercase()
                })),
            )
            .with(Style::rounded());
    } else {
        table.with(Style::modern_rounded());
    }
    println!("{table}");
}

// 保持原有的智能布局逻辑

struct TableRow<'a> {
    rows: &'a [Expression], // 原始数据
    max_width: usize,       // 单行总宽度限制
    col_padding: usize,     // 列间距（通常为3：1边框+2空格）
}

impl<'a> TableRow<'a> {
    /// 智能分Row算法
    fn split_into_rows(&self) -> (Vec<Vec<String>>, Option<Vec<String>>) {
        let mut result = Vec::with_capacity(self.rows.len());
        // let mut heads = vec![];

        // 二维表格
        let heads = match self.rows.first() {
            Some(Expression::List(a)) => {
                Some(a.iter().enumerate().map(|(i, _)| format!("C{i}")).collect())
            }

            Some(Expression::HMap(a)) => {
                Some(a.keys().map(|k| k.to_owned()).collect::<Vec<String>>())
            }

            Some(Expression::Map(a)) => {
                Some(a.keys().map(|k| k.to_owned()).collect::<Vec<String>>())
            }

            _ => None,
        };
        let mut cols = heads.as_ref().map_or(0, |h| h.len());
        let mut current_row = Vec::with_capacity(cols);
        let mut current_len = 0;

        if cols > 0 {
            for expr in self.rows.iter() {
                match expr {
                    Expression::List(a) => {
                        for c in a.iter() {
                            current_row.push(c.to_string());
                        }
                    }
                    Expression::HMap(a) => {
                        for (_, v) in a.iter() {
                            current_row.push(v.to_string());
                        }
                    }
                    Expression::Map(a) => {
                        for (_, v) in a.iter() {
                            current_row.push(v.to_string());
                        }
                    }
                    other => current_row.push(other.to_string()),
                };
                if !current_row.is_empty() {
                    result.push(current_row);
                    current_row = vec![];
                }
            }
            return (result, heads);
        }

        // 一唯表格
        for (i, expr) in self.rows.iter().enumerate() {
            let col = match expr {
                Expression::List(a) => a
                    .as_ref()
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                Expression::HMap(a) => a
                    .as_ref()
                    .values()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                Expression::Map(a) => a
                    .as_ref()
                    .values()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                other => other.to_string(),
            };
            let col_width = strip_ansi_escapes(&col).chars().count() + self.col_padding;

            // 两种情况需要换行：
            // 1. 当前行已有内容且加入新列会超限
            // 2. 单列宽度已超过总限制（需强制拆分列）
            if cols == 0 {
                if !current_row.is_empty() && current_len + col_width > self.max_width {
                    cols = i;
                    // dbg!(&cols);
                    result.push(current_row);
                    current_row = vec![];
                    current_len = 0;
                }
            } else if i % cols == 0 {
                // dbg!(&i);
                result.push(current_row);
                current_row = vec![];
                current_len = 0;
            }
            // 处理超宽列（需拆分成多段）
            if col_width > self.max_width {
                let chunks = self.split_column(&col);
                for chunk in chunks {
                    if !current_row.is_empty() {
                        result.push(current_row);
                        current_row = vec![];
                    }
                    current_row.push(chunk);
                }
                current_len = current_row.last().map(|s| s.len()).unwrap_or(0);
            } else {
                current_row.push(col);
                current_len += col_width;
            }
        }

        if !current_row.is_empty() {
            result.push(current_row);
        }
        (result, None)
    }

    /// 拆分超宽列为多段
    fn split_column(&self, text: &str) -> Vec<String> {
        let max_chunk = self.max_width.saturating_sub(self.col_padding);
        if max_chunk == 0 {
            return vec![text.to_string()];
        }

        // 使用textwrap进行智能换行，考虑单词边界
        textwrap::wrap(text, max_chunk)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}
