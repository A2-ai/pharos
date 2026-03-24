use std::fmt::Write;

use crate::lexer::SpannedToken;
use crate::nmtran::NmtranSpannedToken;

type TokenIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    // Top-level
    Root,

    // Control record sections
    Problem,
    Input,
    Data,
    Subroutines,
    Theta,
    Omega,
    Sigma,
    Estimation,
    Table,
    Simulation,
    Covariance,
    Abbreviated,
    Pk,
    ErrorBlock,
    Des,
    Pred,
    UnknownRecord, // any $RECORD we don't specifically handle

    // Sub-nodes
    // single column def: NAME or NAME=ALIAS or NAME=DROP
    InputColumn,
    // a key value flag, eg ACCEPT=(...) for $DATA
    KeyValue,
    // Something inside (...)
    Parens,
    // Something like AGE.GE.20 or AGE 20
    Filter,
    // A list of names for theta/omega parameters
    ParamNames,
    // A VALUES(..) spec
    ParamValues,
    // A Theta/Omega/Sigma parameter
    Param,
    // A xN construct on params
    Repeat,
    // A REPLACE directive in $ABBREVIATED
    Replace,
    // A BLOCK(..) spec
    Block,
    // A SAME/SAME(..) spec
    Same,
    // standalone keyword: FIX, SAME, CORR, etc.
    Flag,
}

#[derive(Debug, Clone)]
pub struct CstNode {
    pub kind: NodeKind,
    pub children: Vec<CstChild>,
}

impl Default for CstNode {
    fn default() -> Self {
        Self {
            kind: NodeKind::Root,
            children: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum CstChild {
    /// A token leaf
    Token(TokenIdx),
    /// A nested CST node
    Node(CstNode),
    /// An embedded NMTRAN code block ($PK, $ERROR, $DES, $PRED)
    CodeBlock(NmtranCodeBlock),
}

impl CstNode {
    pub(crate) fn new(kind: NodeKind) -> Self {
        CstNode {
            kind,
            children: vec![],
        }
    }

    fn collect_text(&self, tokens: &[SpannedToken], out: &mut String) {
        for child in &self.children {
            match child {
                CstChild::Token(idx) => out.push_str(&tokens[*idx].text),
                CstChild::Node(node) => node.collect_text(tokens, out),
                CstChild::CodeBlock(cb) => {
                    for child in &cb.children {
                        match child {
                            NmtranChild::Token(idx) => out.push_str(&cb.tokens[*idx].text),
                            NmtranChild::Node(node) => node.collect_text(&cb.tokens, out),
                        }
                    }
                }
            }
        }
    }

    pub fn text(&self, tokens: &[SpannedToken]) -> String {
        let mut out = String::new();
        self.collect_text(tokens, &mut out);
        out
    }

    pub(crate) fn debug_tree(&self, tokens: &[SpannedToken]) -> String {
        let mut out = String::new();
        self.fmt_tree(tokens, 0, &mut out);
        out
    }

    fn fmt_tree(&self, tokens: &[SpannedToken], indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        writeln!(out, "{pad}{:?}", self.kind).unwrap();
        for child in &self.children {
            match child {
                CstChild::Token(idx) => {
                    let tok = &tokens[*idx];
                    writeln!(out, "{pad}  {:?} {:?}", tok.token, tok.text).unwrap();
                }
                CstChild::Node(node) => {
                    node.fmt_tree(tokens, indent + 1, out);
                }
                CstChild::CodeBlock(cb) => {
                    for child in &cb.children {
                        match child {
                            NmtranChild::Token(idx) => {
                                let tok = &cb.tokens[*idx];
                                writeln!(out, "{pad}  {:?} {:?}", tok.token, tok.text).unwrap();
                            }
                            NmtranChild::Node(node) => {
                                node.fmt_tree(&cb.tokens, indent + 1, out);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmtranNodeKind {
    /// `VAR = expr`
    Assignment,
    /// `IF (cond) stmt` or `IF (cond) THEN ... ENDIF`
    If,
    /// `CALL SUB(args)`
    Call,
    /// `EXIT n n`
    Exit,
    /// `a + b`, `a .GT. b`, etc.
    BinaryExpr,
    /// `-a`
    UnaryExpr,
    /// `(expr)`
    ParenExpr,
    /// `LOG(x)`, `THETA(1)`, etc.
    FunctionCall,
    /// Comma-separated argument list inside parens
    ArgList,
    /// `DO WHILE (cond) ... ENDDO`
    DoWhile,
    /// Unrecognized statement (fallback)
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NmtranNode {
    pub kind: NmtranNodeKind,
    pub children: Vec<NmtranChild>,
}

#[derive(Debug, Clone)]
pub enum NmtranChild {
    /// Index into `NmtranCodeBlock.tokens`
    Token(usize),
    /// A nested NMTRAN CST node
    Node(NmtranNode),
}

#[derive(Debug, Clone)]
pub struct NmtranCodeBlock {
    pub tokens: Vec<NmtranSpannedToken>,
    pub children: Vec<NmtranChild>,
}

impl NmtranNode {
    fn collect_text(&self, tokens: &[NmtranSpannedToken], out: &mut String) {
        for child in &self.children {
            match child {
                NmtranChild::Token(idx) => out.push_str(&tokens[*idx].text),
                NmtranChild::Node(node) => node.collect_text(tokens, out),
            }
        }
    }

    pub fn text(&self, tokens: &[NmtranSpannedToken]) -> String {
        let mut out = String::new();
        self.collect_text(tokens, &mut out);
        out
    }

    pub(crate) fn debug_tree(&self, tokens: &[NmtranSpannedToken]) -> String {
        let mut out = String::new();
        self.fmt_tree(tokens, 0, &mut out);
        out
    }

    fn fmt_tree(&self, tokens: &[NmtranSpannedToken], indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        writeln!(out, "{pad}{:?}", self.kind).unwrap();
        for child in &self.children {
            match child {
                NmtranChild::Token(idx) => {
                    let tok = &tokens[*idx];
                    writeln!(out, "{pad}  {:?} {:?}", tok.token, tok.text).unwrap();
                }
                NmtranChild::Node(node) => {
                    node.fmt_tree(tokens, indent + 1, out);
                }
            }
        }
    }
}
