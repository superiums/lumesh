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
use smallstr::SmallString;
use smallvec::SmallVec;

#[derive(Clone, PartialEq)]
pub enum Expression {
    // 所有嵌套节点改为Rc包裹
    Group(Rc<Self>),
    BinaryOp(SmallString<[u8; 3]>, Rc<Self>, Rc<Self>),
    Pipe(SmallString<[u8; 3]>, Rc<Self>, Rc<Self>),
    UnaryOp(SmallString<[u8; 3]>, Rc<Self>, bool),
    CustomOp(SmallString<[u8; 10]>, Rc<Self>, bool),

    // 基础类型保持原样
    Symbol(SmallString<[u8; 16]>),
    Variable(SmallString<[u8; 16]>),
    Integer(Int),
    Float(f64),
    Bytes(Vec<u8>),
    String(String),
    Boolean(bool),
    None,

    // 集合类型使用Rc
    List(Rc<Vec<Self>>),
    HMap(Rc<HashMap<SmallString<[u8; 16]>, Self>>),
    Map(Rc<BTreeMap<SmallString<[u8; 16]>, Self>>),

    // 索引和切片优化
    Index(Rc<Self>, Rc<Self>),
    Slice(Rc<Self>, SliceParams),

    // 其他变体保持不变
    Del(SmallString<[u8; 16]>),
    Declare(SmallString<[u8; 16]>, Rc<Self>),
    Assign(SmallString<[u8; 16]>, Rc<Self>),
    For(SmallString<[u8; 16]>, Rc<Self>, Rc<Self>),
    While(Rc<Self>, Rc<Self>),
    Loop(Rc<Self>),
    Match(Rc<Self>, SmallVec<[(Pattern, Rc<Self>); 6]>),
    If(Rc<Self>, Rc<Self>, Rc<Self>),
    Apply(Rc<Self>, Rc<Vec<Self>>),
    Command(Rc<Self>, Rc<Vec<Self>>),
    Alias(SmallString<[u8; 16]>, Rc<Self>),
    Lambda(SmallVec<[SmallString<[u8; 16]>; 6]>, Rc<Self>),
    Function(
        SmallString<[u8; 16]>,
        Vec<(SmallString<[u8; 16]>, Option<Self>)>,
        Option<SmallString<[u8; 16]>>,
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
