use crate::{Environment, Int};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::rc::Rc;
pub mod alias;
pub mod basic;
pub mod builtin;
pub mod catcher;
pub mod cmd_excutor;
pub mod eval;
pub mod eval2;
pub mod from;
pub mod overop;
pub mod pty;
pub mod terminal;

use builtin::Builtin;
use chrono::NaiveDateTime;
#[derive(Clone, PartialEq)]
pub enum Expression {
    // 所有嵌套节点改为Rc包裹
    Group(Rc<Self>),
    BinaryOp(String, Rc<Self>, Rc<Self>),
    Pipe(String, Rc<Self>, Rc<Self>),
    UnaryOp(String, Rc<Self>, bool),

    // 基础类型保持原样
    Symbol(String),
    Variable(String),
    Integer(Int),
    Float(f64),
    Bytes(Vec<u8>), // 这个保持值类型，因为Rc<Vec>反而增加复杂度
    String(String), // 同上
    Boolean(bool),
    None,

    // 集合类型使用Rc
    List(Rc<Vec<Self>>),
    HMap(Rc<HashMap<String, Self>>),
    Map(Rc<BTreeMap<String, Self>>),

    // 索引和切片优化
    Index(Rc<Self>, Rc<Self>),
    Slice(Rc<Self>, SliceParams),

    // 其他变体保持不变
    Del(String),
    Declare(String, Rc<Self>),
    Assign(String, Rc<Self>),
    For(String, Rc<Self>, Rc<Self>),
    While(Rc<Self>, Rc<Self>),
    Loop(Rc<Self>),
    Match(Rc<Self>, Vec<(Pattern, Rc<Self>)>),
    If(Rc<Self>, Rc<Self>, Rc<Self>),
    Apply(Rc<Self>, Rc<Vec<Self>>),
    Command(Rc<Self>, Rc<Vec<Self>>),
    Alias(String, Rc<Self>),
    Lambda(Vec<String>, Rc<Self>),
    Function(
        String,
        Vec<(String, Option<Self>)>,
        Option<String>,
        Rc<Self>,
    ),
    Return(Rc<Self>),
    Break(Rc<Self>),
    Do(Rc<Vec<Self>>),
    Builtin(Builtin),
    Quote(Rc<Self>),
    Catch(Rc<Self>, CatchType, Option<Rc<Self>>),
    Range(Range<Int>),
    DateTime(NaiveDateTime),
    FileSize(FileSize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SizeUnit {
    B,
    K,
    M,
    G,
    T,
    P,
    None,
}
impl SizeUnit {
    pub fn from_str(unit: &str) -> Self {
        match unit.to_uppercase().as_str() {
            "B" | "" => SizeUnit::B,
            "K" | "KB" => SizeUnit::K,
            "M" | "MB" => SizeUnit::M,
            "G" | "GB" => SizeUnit::G,
            "T" | "TB" => SizeUnit::T,
            "P" | "PB" => SizeUnit::P,
            _ => SizeUnit::B,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileSize {
    size: u64, // 文件大小，以字节为单位
    unit: SizeUnit,
}
impl FileSize {
    pub fn new(size: u64, unit: SizeUnit) -> Self {
        Self { size, unit }
    }
    pub fn from(size: u64, unit_str: &str) -> Self {
        Self {
            size,
            unit: SizeUnit::from_str(unit_str),
        }
    }
    pub fn from_bytes(size: u64) -> Self {
        Self {
            size,
            unit: SizeUnit::B,
        }
    }

    fn to_bytes(&self) -> u64 {
        let mut size = self.size;
        // 根据单位进行转换
        size <<= match &self.unit {
            SizeUnit::None | SizeUnit::B => 0,
            SizeUnit::K => 10,
            SizeUnit::M => 20,
            SizeUnit::G => 30,
            SizeUnit::T => 40,
            SizeUnit::P => 50,
        };
        size
    }
    pub fn to_human_readable(&self) -> String {
        let size = self.to_bytes();
        // 定义单位基数
        const KB: u64 = 1 << 10;
        const MB: u64 = 1 << 20;
        const GB: u64 = 1 << 30;
        const TB: u64 = 1 << 40;
        const PB: u64 = 1 << 50;
        let units = [
            ("P", PB, 50),
            ("T", TB, 40),
            ("G", GB, 30),
            ("M", MB, 20),
            ("K", KB, 10),
            ("B", 0, 0),
        ];

        // 查找合适的单位
        let (unit_str, _, shift) = units
            .iter()
            .find(|(_, base, _)| size >= *base)
            .unwrap_or(units.last().unwrap());
        let scaled_size = size >> shift;

        // 格式化输出
        if *unit_str == "B" {
            format!("{}", scaled_size)
        } else if *unit_str == "K" {
            format!("{}K", scaled_size)
        } else {
            let frac_size = (size >> (shift - 10)) & 1023; // 计算小数部分
            format!(
                "{:.2}{}",
                scaled_size as f64 + frac_size as f64 * 0.0009765625,
                unit_str
            )
        }
    }
}
impl PartialOrd for FileSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_bytes().partial_cmp(&other.to_bytes())
    }
}
impl PartialEq for FileSize {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes().eq(&other.to_bytes())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Bind(String),
    Literal(Rc<Expression>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum CatchType {
    Ignore,
    PrintStd,
    PrintErr,
    PrintOver,
    Deel,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SliceParams {
    pub start: Option<Rc<Expression>>,
    pub end: Option<Rc<Expression>>,
    pub step: Option<Rc<Expression>>,
}

/// PartialOrd实现
impl PartialOrd for Expression {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.partial_cmp(b),
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::Symbol(a), Self::Symbol(b)) => a.partial_cmp(b),
            (Self::Bytes(a), Self::Bytes(b)) => a.partial_cmp(b),
            (Self::List(a), Self::List(b)) => a.partial_cmp(b),
            (Self::DateTime(a), Self::DateTime(b)) => a.partial_cmp(b),
            (Self::FileSize(a), Self::FileSize(b)) => a.partial_cmp(b),

            (Self::HMap(a), Self::HMap(b)) => a.as_ref().len().partial_cmp(&b.as_ref().len()),
            (Self::Map(a), Self::Map(b)) => a.as_ref().len().partial_cmp(&b.as_ref().len()),
            _ => None,
        }
    }
}
