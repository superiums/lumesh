#[cfg(test)]
mod tests;

pub type Int = i64;

mod expr;
pub use expr::*;

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
pub use repl::{new_editor, readline};

pub mod runtime;
pub use runtime::parse;

// mod binary;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
