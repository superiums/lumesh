#[cfg(test)]
mod tests;

pub type Int = i64;

mod expr2;
pub use expr2::*;

mod env;
pub use env::*;

mod error;
pub use error::*;

mod parser3;
pub use parser3::*;
mod parser_err;
pub use parser_err::*;

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

pub static mut STRICT: bool = false;
// pub static mut ENV: Environment = Environment::new();
