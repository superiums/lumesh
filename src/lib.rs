#[cfg(test)]
mod tests;

pub type Int = i64;

mod expression;

pub use expression::builtin::Builtin;
pub use expression::eval;
pub use expression::{Expression};
// mod expr2;
// pub use expr2::*;

mod env;
pub use env::*;

mod errors;
pub use errors::LmError;
pub use errors::error_runtime::*;
pub use errors::error_syntax::*;

mod parser;
pub use parser::*;

mod tokens;
pub use tokens::{Token, TokenKind};

mod tokenizer;
pub use tokenizer::*;

pub mod repl;

pub mod runtime;
pub use runtime::{parse, parse_and_eval};

pub mod syntax;
pub use syntax::highlight;

pub mod ai;
pub mod cmdhelper;
pub mod prompt;

pub mod modules;
pub use modules::get_builtin;

pub mod childman;
pub mod keyhandler;

pub mod utils;
// pub use utils::abs;
// pub use utils::canon;
pub mod completion;
pub mod modman;
// pub mod excutor;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// pub static mut STRICT: bool = false;
pub static mut PRINT_DIRECT: bool = true;
pub static mut CFM_ENABLED: bool = false;
// pub static mut ENV: Environment = Environment::new();
pub static mut MAX_RUNTIME_RECURSION: usize = 800;
pub static mut MAX_SYNTAX_RECURSION: usize = 100;
pub static MAX_USEMODE_RECURSION: usize = 100;
