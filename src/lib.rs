pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;

pub use ir::Document;
pub use lexer::lex;
pub use parser::parse;
