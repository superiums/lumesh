use std::collections::{BTreeMap, HashMap};

use crate::Builtin;
use crate::Expression;
use prettytable::{
    Cell,
    Row,
    Table,
    format::consts::{FORMAT_BORDERS_ONLY, FORMAT_BOX_CHARS},
    // format::{LinePosition, LineSeparator},
    row,
};
pub fn pretty_printer(arg: &Expression) -> Result<Expression, crate::LmError> {
    match arg {
        Expression::Map(exprs) => pprint_map(exprs.as_ref()),
        Expression::HMap(exprs) => pprint_hmap(exprs.as_ref()),

        Expression::List(exprs) => {
            // Create a table with one column
            let specified_width = crossterm::terminal::size().unwrap_or((120, 0)).0 as usize;

            let (rows, heads_opt) = TableRow {
                columns: exprs.as_ref(),
                max_width: specified_width - 10,
                col_padding: 3 + 2,
            }
            .split_into_rows();

            let mut t = Table::new();
            if let Some(heads) = heads_opt {
                t.set_format(*FORMAT_BORDERS_ONLY);
                t.set_titles(Row::new(
                    heads
                        .into_iter()
                        .map(|x| Cell::new(&x.to_uppercase()))
                        .collect(),
                ));
            } else {
                t.set_format(*FORMAT_BOX_CHARS);
            }
            for row in rows {
                t.add_row(Row::new(row.into_iter().map(|x| Cell::new(&x)).collect()));
            }

            t.printstd();
        }
        _ => {
            println!("{}", arg);
        }
    }
    Ok(Expression::None)
}

fn pprint_map(exprs: &BTreeMap<String, Expression>) {
    let specified_width = crossterm::terminal::size().unwrap_or((120, 0)).0 as usize;
    // terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(120)

    let mut t = Table::new();
    t.set_format(*FORMAT_BORDERS_ONLY);
    t.set_titles(row!("KEY", "VALUE"));

    for (key, val) in exprs.iter() {
        match &val {
            Expression::Builtin(Builtin { help, .. }) => {
                t.add_row(row!(
                    key,
                    format!("{}", val),
                    textwrap::fill(help, specified_width / 2)
                ));
            }
            Expression::HMap(_) | Expression::Map(_) => {
                t.add_row(row!(key, format!("{:specified_width$}", val)));
            }
            Expression::List(_) => {
                let w = specified_width - key.len() - 3;
                let formatted = format!("{:w$}", val);
                t.add_row(row!(key, textwrap::fill(&formatted, w),));
            }
            _ => {
                // Format the value to the width of the terminal / 5
                let formatted = format!("{}", val);
                let w = specified_width / 3;
                t.add_row(row!(key, textwrap::fill(&formatted, w),));
            }
        }
    }
    // write!($f, "{}", t)
    t.printstd();
}

fn pprint_hmap(exprs: &HashMap<String, Expression>) {
    let specified_width = crossterm::terminal::size().unwrap_or((120, 0)).0 as usize;
    // terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(120)

    let mut t = Table::new();
    t.set_format(*FORMAT_BORDERS_ONLY);
    t.set_titles(row!("KEY", "VALUE"));

    for (key, val) in exprs.iter() {
        match &val {
            Expression::Builtin(Builtin { help, .. }) => {
                t.add_row(row!(
                    key,
                    format!("{}", val),
                    textwrap::fill(help, specified_width / 2)
                ));
            }
            Expression::HMap(_) | Expression::Map(_) => {
                t.add_row(row!(key, format!("{:specified_width$}", val)));
            }
            Expression::List(_) => {
                let w = specified_width - key.len() - 3;
                let formatted = format!("{:w$}", val);
                t.add_row(row!(key, textwrap::fill(&formatted, w),));
            }
            _ => {
                // Format the value to the width of the terminal / 5
                let formatted = format!("{}", val);
                let w = specified_width / 3;
                t.add_row(row!(key, textwrap::fill(&formatted, w),));
            }
        }
    }
    // write!($f, "{}", t)
    t.printstd();
}

struct TableRow<'a> {
    columns: &'a Vec<Expression>, // 原始数据
    max_width: usize,             // 单行总宽度限制
    col_padding: usize,           // 列间距（通常为3：1边框+2空格）
}

impl<'a> TableRow<'a> {
    /// 智能分Row算法
    fn split_into_rows(&self) -> (Vec<Vec<String>>, Option<Vec<String>>) {
        let mut result = vec![];
        let mut heads = vec![];
        let mut current_row = vec![];
        let mut current_len = 0;

        // 二维表格
        let mut cols = match self.columns.first() {
            Some(Expression::List(a)) => {
                heads = a
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("C{}", i))
                    .collect();
                a.len()
            }
            Some(Expression::HMap(a)) => {
                heads = a.iter().map(|(k, _)| k.to_owned()).collect::<Vec<String>>();
                a.keys().len()
            }
            Some(Expression::Map(a)) => {
                heads = a.iter().map(|(k, _)| k.to_owned()).collect::<Vec<String>>();
                a.keys().len()
            }
            _ => 0,
        };
        if cols > 0 {
            for expr in self.columns.iter() {
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
            return (result, Some(heads));
        }

        // 一唯表格
        for (i, expr) in self.columns.iter().enumerate() {
            let col = match expr {
                Expression::List(a) => a
                    .as_ref()
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                Expression::HMap(a) => a
                    .as_ref()
                    .iter()
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                Expression::Map(a) => a
                    .as_ref()
                    .iter()
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<String>>()
                    .join("\t"),
                other => other.to_string(),
            };
            let col_width = col.chars().count() + self.col_padding;

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
        let max_chunk = self.max_width - self.col_padding;
        text.chars()
            .collect::<Vec<_>>()
            .chunks(max_chunk)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }
}
