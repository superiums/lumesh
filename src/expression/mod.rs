use crate::{Environment, Int};
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
