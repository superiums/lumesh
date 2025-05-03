#[cfg(test)]
mod tests;

pub type Int = i64;

mod expression;

pub use expression::builtin::Builtin;
pub use expression::eval;
pub use expression::{Expression, Pattern, SliceParams};
// mod expr2;
// pub use expr2::*;

mod env;
pub use env::*;

mod error;
pub use error::*;

mod parser;
pub use parser::*;

mod tokens;
pub use tokens::{Token, TokenKind};

mod tokenizer;
pub use tokenizer::*;

pub mod repl;

pub mod runtime;
pub use runtime::{parse, parse_and_eval, syntax_highlight};

pub mod syntax;
pub use syntax::highlight;

pub mod ai;
pub mod cmdhelper;
pub mod prompt;

pub mod binary;
pub use binary::get_builtin;

// pub mod excutor;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static mut STRICT: bool = false;
// pub static mut ENV: Environment = Environment::new();
