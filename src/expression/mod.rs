use crate::{Environment, Int};
use std::collections::BTreeMap;

pub mod basic;
pub mod builtin;
pub mod eval;
pub mod eval2;
pub mod from;
pub mod overop;
pub mod pipe_excutor;

use builtin::Builtin;

#[derive(Clone, PartialEq)]
pub enum Expression {
    Group(Box<Self>),
    BinaryOp(String, Box<Self>, Box<Self>),
    UnaryOp(String, Box<Self>, bool),
    Symbol(String),
    Integer(Int),
    Float(f64),
    Bytes(Vec<u8>),
    String(String),
    Boolean(bool),
    List(Vec<Self>),
    Map(BTreeMap<String, Self>),
    Index(Box<Self>, Box<Self>),
    Slice(Box<Self>, SliceParams),
    None,
    Del(String),
    Declare(String, Box<Self>),
    Assign(String, Box<Self>),
    For(String, Box<Self>, Box<Self>),
    While(Box<Self>, Box<Self>),
    Match(Box<Self>, Vec<(Pattern, Box<Self>)>),
    If(Box<Self>, Box<Self>, Box<Self>),
    Apply(Box<Self>, Vec<Self>),
    Command(Box<Self>, Vec<Self>),
    Lambda(Vec<String>, Box<Self>, Environment),
    Macro(Vec<String>, Box<Self>),
    Function(String, Vec<(String, Option<Self>)>, Box<Self>, Environment),
    Return(Box<Self>),
    Do(Vec<Self>),
    Builtin(Builtin),
    Quote(Box<Self>),
    Catch(Box<Self>, CatchType, Option<Box<Self>>),
    Error {
        code: Int,
        msg: String,
        expr: Box<Self>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Bind(String),
    Literal(Box<Expression>),
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
    pub start: Option<Box<Expression>>,
    pub end: Option<Box<Expression>>,
    pub step: Option<Box<Expression>>,
}
