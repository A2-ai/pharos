use crate::cst::CstNode;
use crate::lexer::SpannedToken;

mod ast;
mod cst;
mod errors;
pub mod lexer;
mod parser;

#[derive(Debug)]
pub struct Model {
    pub problem: String,
    tokens: Vec<SpannedToken>,
    cst: CstNode,
}
