use crate::Expression;
use std::fmt;

/// Table
#[derive(Debug, Clone, PartialEq)]
pub struct TableData {
    headers: Vec<String>,
    rows: Vec<Vec<Expression>>,
}

impl TableData {
    /// 创建新的表格
    pub fn new(headers: Vec<String>, rows: Vec<Vec<Expression>>) -> Self {
        Self { headers, rows }
    }
    pub fn with_header(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    /// 添加新行
    pub fn push_row(&mut self, row: Vec<Expression>) {
        // 确保行的列数与表头一致，不足则填充 None
        let mut padded_row = row;
        if padded_row.len() < self.headers.len() {
            padded_row.resize(self.headers.len(), Expression::None);
        } else if padded_row.len() > self.headers.len() {
            // 如果行太长，截断到表头长度
            padded_row.truncate(self.headers.len());
        }
        self.rows.push(padded_row);
    }

    /// 获取列数据
    pub fn get_column(&self, index: usize) -> Option<Vec<Expression>> {
        if index >= self.headers.len() {
            return None;
        }
        Some(
            self.rows
                .iter()
                .map(|row| row.get(index).cloned().unwrap_or(Expression::None))
                .collect(),
        )
    }
    pub fn get_header_indexes(&self, col_names: &Vec<String>) -> Vec<usize> {
        col_names
            .iter()
            .filter_map(|x| self.headers.iter().position(|h| h == x))
            .collect()
    }
    pub fn get_columns(&self, indexes: &[usize]) -> Option<Vec<Vec<Expression>>> {
        if indexes.is_empty() {
            return None;
        }
        Some(
            self.rows
                .iter()
                .map(|row| {
                    indexes
                        .iter()
                        .map(|i| row.iter().nth(*i).map_or(Expression::None, |x| x.clone()))
                        .collect()
                })
                .collect::<Vec<_>>(),
        )
    }

    /// 获取行数据
    pub fn get_row(&self, index: usize) -> Option<&[Expression]> {
        self.rows.get(index).map(|row| row.as_slice())
    }

    /// 过滤行
    pub fn filter_rows<F>(&self, mut predicate: F) -> TableData
    where
        F: FnMut(usize, &[Expression]) -> bool,
    {
        let filtered_rows = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                if predicate(i, row.as_slice()) {
                    Some(row.clone())
                } else {
                    None
                }
            })
            .collect();
        TableData {
            headers: self.headers.clone(),
            rows: filtered_rows,
        }
    }

    /// 按列排序
    pub fn sort_by_column(&self, column: usize) -> TableData {
        let mut rows = self.rows.clone();
        rows.sort_by(|a, b| match (a.get(column), b.get(column)) {
            (Some(a_val), Some(b_val)) => a_val
                .partial_cmp(b_val)
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        });
        TableData {
            headers: self.headers.clone(),
            rows,
        }
    }

    /// 获取表头
    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    pub fn rows(&self) -> &Vec<Vec<Expression>> {
        &self.rows
    }

    /// 获取行数
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// 获取列数
    pub fn column_count(&self) -> usize {
        self.headers.len()
    }

    /// 转换为 List<Map> 格式（向后兼容）
    pub fn to_list_map(&self) -> Expression {
        use std::collections::BTreeMap;

        let rows: Vec<Expression> = self
            .rows
            .iter()
            .map(|row| {
                let map: BTreeMap<String, Expression> = self
                    .headers
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

        Expression::from(rows)
    }
}

impl fmt::Display for TableData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            // 美化格式输出
            if self.headers.is_empty() {
                return write!(f, "[]");
            }

            // 计算每列的最大宽度
            let mut col_widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();

            for row in &self.rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < col_widths.len() {
                        let cell_str = cell.to_string();
                        col_widths[i] = col_widths[i].max(cell_str.len());
                    }
                }
            }

            // 输出表头
            writeln!(
                f,
                "{}",
                "─".repeat(col_widths.iter().sum::<usize>() + col_widths.len() * 3 - 1)
            )?;
            for (i, header) in self.headers.iter().enumerate() {
                if i > 0 {
                    write!(f, " │ ")?;
                }
                write!(f, "{:width$}", header, width = col_widths[i])?;
            }
            writeln!(f)?;
            writeln!(
                f,
                "{}",
                "─".repeat(col_widths.iter().sum::<usize>() + col_widths.len() * 3 - 1)
            )?;

            // 输出数据行
            for row in &self.rows {
                for (i, cell) in row.iter().enumerate() {
                    if i > 0 {
                        write!(f, " │ ")?;
                    }
                    write!(f, "{:width$}", cell.to_string(), width = col_widths[i])?;
                }
                writeln!(f)?;
            }

            if !self.rows.is_empty() {
                writeln!(
                    f,
                    "{}",
                    "─".repeat(col_widths.iter().sum::<usize>() + col_widths.len() * 3 - 1)
                )?;
            }
        } else {
            // 紧凑格式输出
            write!(f, "[\n")?;
            if !self.headers.is_empty() {
                write!(f, "  [{}]", self.headers.join(", "))?;
            }
            for row in &self.rows {
                write!(
                    f,
                    ",\n  [{}]",
                    row.iter()
                        .map(|cell| cell.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
            write!(f, "\n]")?;
        }
        Ok(())
    }
}
