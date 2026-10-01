//! Edits to a parsed model.
//!
//! Every edit changes token text on a copy of the model, renders it, and
//! parses the result again. The returned model is therefore always a fresh,
//! valid parse; an edit that would produce an unparseable file is an error and
//! the original model is untouched.

use std::collections::{BTreeSet, HashMap};

use anyhow::{Result, bail};

use crate::ast::{BlockStructure, InputColumnKind, OmegaSigmaBlock};
use crate::cst::{
    CstChild, CstNode, NmtranChild, NmtranCodeBlock, NmtranNode, NmtranNodeKind, NodeKind,
};
use crate::lexer::{SpannedToken, Token};
use crate::nmtran::{NmtranSpannedToken, NmtranToken};

use super::Model;
use super::edit_params::Change;

/// A code record an edit can target. `$PRED` is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeRecord {
    Pk,
    Error,
    Des,
}

impl CodeRecord {
    pub fn name(self) -> &'static str {
        match self {
            CodeRecord::Pk => "$PK",
            CodeRecord::Error => "$ERROR",
            CodeRecord::Des => "$DES",
        }
    }
}

/// The kind of parameter a placeholder ref points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Theta,
    Omega,
    Sigma,
}

impl RefKind {
    fn name(self) -> &'static str {
        match self {
            RefKind::Theta => "THETA",
            RefKind::Omega => "OMEGA",
            RefKind::Sigma => "SIGMA",
        }
    }

    fn suffixes(self) -> &'static [&'static str] {
        match self {
            RefKind::Theta => &["theta"],
            RefKind::Omega => &["eta", "mu", "omega"],
            RefKind::Sigma => &["eps", "sigma"],
        }
    }
}

/// A named handle on a parameter row, used by `{name.suffix}` placeholders.
/// `index` is the 1-based NONMEM number (THETA(n), ETA(n), EPS(n)). `col` is
/// set only for an off-diagonal OMEGA element, which is `OMEGA(index,col)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamRef {
    pub kind: RefKind,
    pub index: usize,
    pub col: Option<usize>,
}

/// A THETA row to append.
#[derive(Debug, Clone, Default)]
pub struct NewTheta {
    pub init: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub fix: bool,
    pub comment: Option<String>,
}

/// Replace every `{name.suffix}` in `text` with the NONMEM reference it names.
///
/// Returns the resolved text and the ref names it used.
pub fn resolve_placeholders(
    text: &str,
    refs: &HashMap<String, ParamRef>,
) -> Result<(String, Vec<String>)> {
    let mut out = String::with_capacity(text.len());
    let mut used = Vec::new();
    let mut rest = text;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            bail!("Unclosed `{{` in `{text}`.");
        };
        let inner = after[..close].trim();
        let Some((name, suffix)) = inner.rsplit_once('.') else {
            bail!("Placeholder `{{{inner}}}` needs a suffix, e.g. `{{{inner}.theta}}`.");
        };
        let Some(param) = refs.get(name) else {
            bail!(
                "Placeholder `{{{inner}}}` refers to `{name}`, which is not declared. \
                 Declare it with `ref = \"{name}\"` when adding the row."
            );
        };
        let suffix_lc = suffix.to_lowercase();
        let n = param.index;
        if let Some(j) = param.col {
            if param.kind != RefKind::Omega || suffix_lc != "omega" {
                bail!("`{name}` is an off-diagonal OMEGA element, so only `.omega` applies.");
            }
            out.push_str(&format!("OMEGA({n},{j})"));
            if !used.iter().any(|u| u == name) {
                used.push(name.to_string());
            }
            rest = &after[close + 1..];
            continue;
        }
        let resolved = match (param.kind, suffix_lc.as_str()) {
            (RefKind::Theta, "theta") => format!("THETA({n})"),
            (RefKind::Omega, "eta") => format!("ETA({n})"),
            (RefKind::Omega, "mu") => format!("MU_{n}"),
            (RefKind::Omega, "omega") => format!("OMEGA({n},{n})"),
            (RefKind::Sigma, "eps") => format!("EPS({n})"),
            (RefKind::Sigma, "sigma") => format!("SIGMA({n},{n})"),
            (kind, _) => {
                let valid: Vec<String> =
                    kind.suffixes().iter().map(|s| format!("`.{s}`")).collect();
                bail!(
                    "`{name}` is a {}, so `.{suffix}` doesn't apply. Use {}.",
                    kind.name(),
                    valid.join(", ")
                );
            }
        };
        out.push_str(&resolved);
        if !used.iter().any(|u| u == name) {
            used.push(name.to_string());
        }
        rest = &after[close + 1..];
    }
    if rest.contains('}') {
        bail!("Unmatched `}}` in `{text}`.");
    }
    out.push_str(rest);
    Ok((out, used))
}

/// Number of rows (ETAs or EPSs) the blocks declare.
pub(super) fn block_dimension(blocks: &[OmegaSigmaBlock]) -> usize {
    blocks
        .iter()
        .map(|b| match b.structure {
            BlockStructure::Diagonal => b.parameters.len(),
            BlockStructure::Block { size } => size,
            BlockStructure::BlockSame { size, repeats } => size * repeats,
        })
        .sum()
}

pub(super) fn format_number(value: f64, what: &str) -> Result<String> {
    if !value.is_finite() {
        bail!("{what} must be a finite number, got {value}.");
    }
    Ok(format!("{value}"))
}

pub(super) fn format_theta_row(theta: &NewTheta) -> Result<String> {
    let init = format_number(theta.init, "init")?;
    let lower = theta.lower.map(|v| format_number(v, "lower")).transpose()?;
    let upper = theta.upper.map(|v| format_number(v, "upper")).transpose()?;

    if let Some(l) = theta.lower
        && l > theta.init
    {
        bail!("lower ({l}) is above init ({}).", theta.init);
    }
    if let Some(u) = theta.upper
        && u < theta.init
    {
        bail!("upper ({u}) is below init ({}).", theta.init);
    }

    let mut row = match (lower, upper) {
        (None, None) => init,
        (Some(l), None) => format!("({l}, {init})"),
        (None, Some(u)) => format!("(-INF, {init}, {u})"),
        (Some(l), Some(u)) => format!("({l}, {init}, {u})"),
    };
    if theta.fix {
        row.push_str(" FIX");
    }
    if let Some(comment) = &theta.comment {
        if comment.contains('\n') {
            bail!("comment must be a single line.");
        }
        row.push_str(&format!(" ;{comment}"));
    }
    Ok(row)
}

/// Last token in a main-lexer CST subtree that isn't whitespace or a newline.
pub(super) fn last_token_in(node: &CstNode, tokens: &[SpannedToken]) -> Option<usize> {
    for child in node.children.iter().rev() {
        match child {
            CstChild::Token(i) => {
                if !matches!(tokens[*i].token, Token::Newline | Token::Whitespace) {
                    return Some(*i);
                }
            }
            CstChild::Node(n) => {
                if let Some(i) = last_token_in(n, tokens) {
                    return Some(i);
                }
            }
            CstChild::CodeBlock(_) => {}
        }
    }
    None
}

/// The last token on the line that holds `record.children[child_idx]`.
pub(super) fn line_end_from(
    record: &CstNode,
    child_idx: usize,
    tokens: &[SpannedToken],
) -> Option<usize> {
    let mut anchor = None;
    for child in &record.children[child_idx..] {
        match child {
            CstChild::Token(i) => match tokens[*i].token {
                Token::Newline => break,
                Token::Whitespace => {}
                _ => anchor = Some(*i),
            },
            CstChild::Node(n) => {
                if let Some(i) = last_token_in(n, tokens) {
                    anchor = Some(i);
                }
            }
            CstChild::CodeBlock(_) => {}
        }
    }
    anchor
}

/// Last token in an NMTRAN subtree, ignoring nothing (comments included).
pub(super) fn nm_last_token(child: &NmtranChild) -> Option<usize> {
    match child {
        NmtranChild::Token(i) => Some(*i),
        NmtranChild::Node(n) => n.children.iter().rev().find_map(nm_last_token),
    }
}

/// First token in an NMTRAN subtree.
pub(super) fn nm_first_token(child: &NmtranChild) -> Option<usize> {
    match child {
        NmtranChild::Token(i) => Some(*i),
        NmtranChild::Node(n) => n.children.iter().find_map(nm_first_token),
    }
}

pub(super) fn is_nm_trivia(tok: &NmtranSpannedToken) -> bool {
    tok.token.is_trivia() || matches!(tok.token, NmtranToken::Newline | NmtranToken::Ampersand)
}

/// Every token index in an NMTRAN subtree, in order.
pub(super) fn nm_tokens(child: &NmtranChild, out: &mut Vec<usize>) {
    match child {
        NmtranChild::Token(i) => out.push(*i),
        NmtranChild::Node(n) => n.children.iter().for_each(|c| nm_tokens(c, out)),
    }
}

/// An assignment statement found in a code block.
pub(super) struct Assignment<'a> {
    /// Normalized left-hand side: upper case, no whitespace (e.g. `DADT(2)`).
    pub(super) lhs: String,
    /// The right-hand-side expression.
    pub(super) expr: &'a NmtranChild,
}

pub(super) fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

pub(super) fn assignment_parts<'a>(
    node: &'a NmtranNode,
    tokens: &[NmtranSpannedToken],
) -> Option<Assignment<'a>> {
    let mut lhs = String::new();
    let mut seen_equals = false;
    for child in &node.children {
        if seen_equals {
            match child {
                NmtranChild::Token(i) if is_nm_trivia(&tokens[*i]) => continue,
                _ => {
                    return Some(Assignment {
                        lhs: normalize_name(&lhs),
                        expr: child,
                    });
                }
            }
        }
        match child {
            NmtranChild::Token(i) if tokens[*i].token == NmtranToken::Equals => seen_equals = true,
            NmtranChild::Token(i) if is_nm_trivia(&tokens[*i]) => {}
            other => {
                let mut idx = vec![];
                nm_tokens(other, &mut idx);
                for i in idx {
                    if !is_nm_trivia(&tokens[i]) {
                        lhs.push_str(&tokens[i].text);
                    }
                }
            }
        }
    }
    None
}

/// All assignments in a code block, including those nested inside IF/DO bodies.
pub(super) fn collect_assignments<'a>(
    children: &'a [NmtranChild],
    tokens: &[NmtranSpannedToken],
    out: &mut Vec<Assignment<'a>>,
) {
    for child in children {
        if let NmtranChild::Node(node) = child {
            if node.kind == NmtranNodeKind::Assignment {
                if let Some(a) = assignment_parts(node, tokens) {
                    out.push(a);
                }
            } else {
                collect_assignments(&node.children, tokens, out);
            }
        }
    }
}

/// Function-call nodes named `name` (case-insensitive) inside an expression.
fn collect_calls<'a>(
    child: &'a NmtranChild,
    name: &str,
    tokens: &[NmtranSpannedToken],
    out: &mut Vec<&'a NmtranNode>,
) {
    if let NmtranChild::Node(node) = child {
        if node.kind == NmtranNodeKind::FunctionCall
            && let Some(first) = node.children.iter().find_map(nm_first_token)
            && tokens[first].text.eq_ignore_ascii_case(name)
        {
            out.push(node);
        }
        for c in &node.children {
            collect_calls(c, name, tokens, out);
        }
    }
}

/// `NAME ( INT )` and `NAME ( INT , INT )` references in a code block, as
/// (name, indices) pairs.
pub(super) fn indexed_references(tokens: &[NmtranSpannedToken]) -> Vec<(String, Vec<usize>)> {
    let sig: Vec<&NmtranSpannedToken> = tokens.iter().filter(|t| !is_nm_trivia(t)).collect();
    let mut out = vec![];
    let mut i = 0;
    while i < sig.len() {
        if sig[i].token == NmtranToken::Ident
            && sig
                .get(i + 1)
                .is_some_and(|t| t.token == NmtranToken::LeftParen)
        {
            let name = sig[i].text.to_uppercase();
            let mut j = i + 2;
            let mut nums = vec![];
            let mut ok = true;
            loop {
                match sig.get(j) {
                    Some(t) if t.token == NmtranToken::Int => {
                        nums.push(t.text.parse::<usize>().unwrap_or(0));
                        j += 1;
                    }
                    _ => {
                        ok = false;
                        break;
                    }
                }
                match sig.get(j) {
                    Some(t) if t.token == NmtranToken::Comma => j += 1,
                    Some(t) if t.token == NmtranToken::RightParen => break,
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                out.push((name, nums));
                i = j;
            }
        }
        i += 1;
    }
    out
}

/// First token in a main-lexer CST subtree.
pub(super) fn first_token_in(node: &CstNode) -> Option<usize> {
    node.children.iter().find_map(|c| match c {
        CstChild::Token(i) => Some(*i),
        CstChild::Node(n) => first_token_in(n),
        CstChild::CodeBlock(_) => None,
    })
}

/// Every main-lexer token index in a CST subtree, in order.
pub(super) fn node_tokens(node: &CstNode, out: &mut Vec<usize>) {
    for child in &node.children {
        match child {
            CstChild::Token(i) => out.push(*i),
            CstChild::Node(n) => node_tokens(n, out),
            CstChild::CodeBlock(_) => {}
        }
    }
}

/// Last token in a CST subtree that isn't whitespace, a newline or a comment.
/// Text appended after it lands before any trailing comment.
pub(super) fn last_code_token_in(node: &CstNode, tokens: &[SpannedToken]) -> Option<usize> {
    let mut idx = vec![];
    node_tokens(node, &mut idx);
    idx.into_iter().rev().find(|&i| {
        !matches!(
            tokens[i].token,
            Token::Newline | Token::Whitespace | Token::Comment
        )
    })
}

/// First token on the line holding token `i`. Main-lexer tokens are in
/// source order; a record header also starts a line, because the text of a
/// code record before it is not in the main token list.
pub(super) fn line_start(tokens: &[SpannedToken], i: usize) -> usize {
    let mut j = i;
    while j > 0 {
        if tokens[j].token == Token::ControlRecord {
            return j;
        }
        if tokens[j - 1].token == Token::Newline {
            return j;
        }
        j -= 1;
    }
    0
}

/// The newline ending the line that holds token `i`, or the line's last
/// token when no newline follows (end of file or a record header).
pub(super) fn line_end(tokens: &[SpannedToken], i: usize) -> usize {
    let mut j = i;
    while j + 1 < tokens.len() {
        if tokens[j].token == Token::Newline {
            return j;
        }
        if tokens[j + 1].token == Token::ControlRecord {
            return j;
        }
        j += 1;
    }
    j
}

/// Last token on the line holding token `i` that isn't whitespace or the newline.
pub(super) fn line_last_token(tokens: &[SpannedToken], i: usize) -> usize {
    let end = line_end(tokens, i);
    (line_start(tokens, i)..=end)
        .rev()
        .find(|&j| !matches!(tokens[j].token, Token::Newline | Token::Whitespace))
        .unwrap_or(i)
}

/// The comment token on the line holding token `i`, if any.
pub(super) fn line_comment(tokens: &[SpannedToken], i: usize) -> Option<usize> {
    (line_start(tokens, i)..=line_end(tokens, i)).find(|&j| tokens[j].token == Token::Comment)
}

/// NONMEM names that are defined without an assignment or an `$INPUT` column.
pub(super) const RESERVED_NAMES: &[&str] = &[
    "F",
    "T",
    "ICALL",
    "NEWIND",
    "IREP",
    "MIXNUM",
    "MIXEST",
    "NIREC",
    "NDREC",
    "IPROB",
    "NPROB",
    "S1NUM",
    "S2NUM",
    "S1NIT",
    "S2NIT",
    "S1IT",
    "S2IT",
    "IERPRD",
    "IERPRDU",
    "COMACT",
    "COMSAV",
    "MDVRES",
    "ETASXI",
    "NOFIRSTDERCODE",
    "PRED_",
    "RES_",
    "LIREC",
    "TSTATE",
    "ISFINL",
    "NPDE_MODE",
    "DV_LOQ",
    "CDF_L",
    "DV_LAQ",
    "CDF_LA",
];

/// Model-defined names that are visible across the input: data items and
/// left-hand sides in any code record.
pub(super) fn defined_names(model: &Model) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = RESERVED_NAMES.iter().map(|s| s.to_string()).collect();
    names.extend(input_names(model));
    for cb in model.code_blocks() {
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        for a in assignments {
            let base = a.lhs.split('(').next().unwrap_or(&a.lhs).to_string();
            names.insert(base);
        }
    }
    names
}

/// `$INPUT` names, upper case, excluding dropped columns. Both sides of an
/// alias count.
pub(super) fn input_names(model: &Model) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for col in &model.input_columns {
        match &col.kind {
            InputColumnKind::Included(n) => {
                names.insert(n.to_uppercase());
            }
            InputColumnKind::Aliased { from, to } => {
                names.insert(from.to_uppercase());
                names.insert(to.to_uppercase());
            }
            InputColumnKind::Dropped(_) => {}
        }
    }
    names
}

/// Bare names (not followed by `(`) in a piece of NMTRAN code that the model
/// doesn't define.
pub(super) fn undefined_names(model: &Model, code: &str) -> Vec<String> {
    let defined = defined_names(model);
    let toks = crate::nmtran::lex_nmtran(code, 0);
    let sig: Vec<&NmtranSpannedToken> = toks.iter().filter(|t| !is_nm_trivia(t)).collect();
    let mut bad: Vec<String> = vec![];
    for (k, t) in sig.iter().enumerate() {
        if t.token != NmtranToken::Ident {
            continue;
        }
        let name = t.text.to_uppercase();
        let called = sig
            .get(k + 1)
            .is_some_and(|n| n.token == NmtranToken::LeftParen);
        if called {
            continue;
        }
        if !defined.contains(&name) && !bad.contains(&name) {
            bad.push(name);
        }
    }
    bad
}

/// Refuse code that uses a name the model doesn't define.
pub(super) fn check_names(edited: &Model, code: &str) -> Result<()> {
    let bad = undefined_names(edited, code);
    if !bad.is_empty() {
        bail!(
            "{} {} not an $INPUT column, assigned in the model's code, or a NONMEM \
             reserved name.",
            bad.iter()
                .map(|b| format!("`{b}`"))
                .collect::<Vec<_>>()
                .join(", "),
            if bad.len() == 1 { "is" } else { "are" }
        );
    }
    Ok(())
}

/// A compartment declared in `$MODEL`, with its attributes upper-cased.
#[derive(Debug, Clone, PartialEq)]
pub struct Compartment {
    pub name: String,
    pub attributes: Vec<String>,
}

impl Model {
    pub(super) fn reparse(&self) -> Result<Model> {
        Model::parse("edited model", &self.model_content())
    }

    /// Number of ETAs the model declares.
    pub fn eta_count(&self) -> usize {
        block_dimension(&self.omega_blocks)
    }

    /// Number of EPSs the model declares.
    pub fn eps_count(&self) -> usize {
        block_dimension(&self.sigma_blocks)
    }

    pub(super) fn code_record_idx(&self, record: CodeRecord) -> Result<usize> {
        let block = match record {
            CodeRecord::Pk => &self.pk,
            CodeRecord::Error => &self.error,
            CodeRecord::Des => &self.des,
        };
        match block {
            Some(b) => Ok(b.record_idx),
            None => bail!("The model has no {} record.", record.name()),
        }
    }

    pub(super) fn code_block_at(&self, record_idx: usize) -> Option<&NmtranCodeBlock> {
        let CstChild::Node(node) = self.cst.children.get(record_idx)? else {
            return None;
        };
        node.children.iter().find_map(|c| match c {
            CstChild::CodeBlock(cb) => Some(cb),
            _ => None,
        })
    }

    pub(super) fn code_block_at_mut(&mut self, record_idx: usize) -> Option<&mut NmtranCodeBlock> {
        let CstChild::Node(node) = self.cst.children.get_mut(record_idx)? else {
            return None;
        };
        node.children.iter_mut().find_map(|c| match c {
            CstChild::CodeBlock(cb) => Some(cb),
            _ => None,
        })
    }

    pub(super) fn code_blocks(&self) -> Vec<&NmtranCodeBlock> {
        [&self.pk, &self.error, &self.des, &self.pred]
            .into_iter()
            .flatten()
            .filter_map(|b| self.code_block_at(b.record_idx))
            .collect()
    }

    /// Index of the `$MODEL` record in the CST, if any.
    pub(super) fn model_record_idx(&self) -> Option<usize> {
        self.cst.children.iter().position(|c| match c {
            CstChild::Node(n) if n.kind == NodeKind::UnknownRecord => first_token_in(n)
                .is_some_and(|i| {
                    self.tokens[i].token == Token::ControlRecord
                        && self.tokens[i].text.to_uppercase().starts_with("$MOD")
                }),
            _ => false,
        })
    }

    /// Compartments declared by `COMP=` entries in `$MODEL`, in order.
    /// `None` when the model has no `$MODEL` record.
    pub fn compartments(&self) -> Option<Vec<Compartment>> {
        let idx = self.model_record_idx()?;
        let CstChild::Node(node) = &self.cst.children[idx] else {
            return None;
        };
        let mut toks = vec![];
        node_tokens(node, &mut toks);
        let sig: Vec<&SpannedToken> = toks
            .into_iter()
            .map(|i| &self.tokens[i])
            .filter(|t| {
                !matches!(
                    t.token,
                    Token::Whitespace | Token::Newline | Token::Comment | Token::Comma
                )
            })
            .collect();
        let mut out = vec![];
        let mut k = 0;
        while k < sig.len() {
            let key = sig[k].text.to_uppercase();
            let is_comp = sig[k].token == Token::Symbol
                && (key == "COMP" || key == "COMPARTMENT")
                && sig.get(k + 1).is_some_and(|t| t.token == Token::Equals);
            if !is_comp {
                k += 1;
                continue;
            }
            k += 2;
            let mut words = vec![];
            if sig.get(k).is_some_and(|t| t.token == Token::LeftParen) {
                k += 1;
                while let Some(t) = sig.get(k) {
                    k += 1;
                    if t.token == Token::RightParen {
                        break;
                    }
                    words.push(t.text.trim_matches(|c| c == '"' || c == '\'').to_string());
                }
            } else if let Some(t) = sig.get(k) {
                words.push(t.text.clone());
                k += 1;
            }
            if let Some((name, attrs)) = words.split_first() {
                out.push(Compartment {
                    name: name.clone(),
                    attributes: attrs.iter().map(|a| a.to_uppercase()).collect(),
                });
            }
        }
        Some(out)
    }

    /// THETA/ETA/EPS/OMEGA/SIGMA references in code that point past the
    /// declared parameters, e.g. `ETA(9)` with three OMEGAs. With a `$MODEL`
    /// record, `A(n)`, `A_0(n)` and `DADT(n)` are checked against its
    /// compartments.
    pub fn invalid_references(&self) -> BTreeSet<String> {
        let n_theta = self.thetas.len();
        let n_eta = self.eta_count();
        let n_eps = self.eps_count();
        let n_comp = self.compartments().map(|c| c.len());
        let mut bad = BTreeSet::new();
        for cb in self.code_blocks() {
            for (name, nums) in indexed_references(&cb.tokens) {
                let limit = match (name.as_str(), nums.len()) {
                    ("THETA", 1) => n_theta,
                    ("ETA", 1) => n_eta,
                    ("EPS" | "ERR", 1) => n_eps,
                    ("OMEGA", 2) => n_eta,
                    ("SIGMA", 2) => n_eps,
                    ("A" | "A_0" | "DADT", 1) => match n_comp {
                        Some(n) => n,
                        None => continue,
                    },
                    _ => continue,
                };
                if nums.iter().any(|&n| n == 0 || n > limit) {
                    let args: Vec<String> = nums.iter().map(|n| n.to_string()).collect();
                    bad.insert(format!("{name}({})", args.join(",")));
                }
            }
        }
        bad
    }

    /// Refuse an edit that introduced references to parameters that don't exist.
    pub(super) fn check_new_references(&self, edited: &Model) -> Result<()> {
        let before = self.invalid_references();
        let new: Vec<String> = edited
            .invalid_references()
            .difference(&before)
            .cloned()
            .collect();
        if !new.is_empty() {
            bail!(
                "The edit refers to parameters the model doesn't declare: {} \
                 (THETAs: {}, ETAs: {}, EPSs: {}).",
                new.join(", "),
                edited.thetas.len(),
                edited.eta_count(),
                edited.eps_count()
            );
        }
        Ok(())
    }

    /// Set, replace or remove the comment on the one statement in `record`
    /// whose left-hand side is `lhs`. The comment is the one at the end of the
    /// statement's last line; a statement with comments inside it is refused.
    pub fn set_statement_comment(
        &self,
        record: CodeRecord,
        lhs: &str,
        comment: &Change<String>,
    ) -> Result<Model> {
        if *comment == Change::Keep {
            return Ok(self.clone());
        }
        if let Change::Set(c) = comment
            && c.contains('\n')
        {
            bail!("comment must be a single line.");
        }
        let record_idx = self.code_record_idx(record)?;
        let cb = self
            .code_block_at(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;

        let target = normalize_name(lhs);
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        let matches: Vec<&Assignment> = assignments.iter().filter(|a| a.lhs == target).collect();
        let stmt = match matches.as_slice() {
            [one] => one,
            [] => bail!("No statement in {} assigns `{lhs}`.", record.name()),
            many => bail!(
                "`{lhs}` is assigned in {} statements in {}; expected exactly 1.",
                many.len(),
                record.name()
            ),
        };
        let mut inside = vec![];
        nm_tokens(stmt.expr, &mut inside);
        if inside
            .iter()
            .any(|&i| cb.tokens[i].token == NmtranToken::Comment)
        {
            bail!("`{lhs}` has comments inside the statement; edit it by hand.");
        }
        let last = *inside
            .last()
            .ok_or_else(|| anyhow::anyhow!("`{lhs}` has an empty right-hand side."))?;
        let existing = cb.tokens[last + 1..]
            .iter()
            .take_while(|t| t.token != NmtranToken::Newline)
            .position(|t| t.token == NmtranToken::Comment)
            .map(|k| last + 1 + k);

        let mut edited = self.clone();
        let cb = edited
            .code_block_at_mut(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;
        match (comment, existing) {
            (Change::Set(c), Some(i)) => {
                let spaced = cb.tokens[i].text.starts_with("; ");
                cb.tokens[i].text = if spaced {
                    format!("; {c}")
                } else {
                    format!(";{c}")
                };
            }
            (Change::Set(c), None) => cb.tokens[last].text.push_str(&format!(" ; {c}")),
            (Change::Remove, Some(i)) => {
                cb.tokens[i].text.clear();
                if cb.tokens[i - 1].token == NmtranToken::Whitespace {
                    cb.tokens[i - 1].text.clear();
                }
            }
            (Change::Remove, None) => bail!("`{lhs}` has no comment to remove."),
            (Change::Keep, _) => unreachable!(),
        }
        edited.reparse()
    }

    /// Append `text` to the right-hand side of the one statement in `record`
    /// whose left-hand side is `lhs`. With `within`, the text goes at the end
    /// of the one call to that function inside the right-hand side instead.
    pub fn append_to_statement(
        &self,
        record: CodeRecord,
        lhs: &str,
        within: Option<&str>,
        text: &str,
    ) -> Result<Model> {
        if text.contains('\n') {
            bail!("`append` must be a single line.");
        }
        let record_idx = self.code_record_idx(record)?;
        let cb = self
            .code_block_at(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;

        let target = normalize_name(lhs);
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        let matches: Vec<&Assignment> = assignments.iter().filter(|a| a.lhs == target).collect();
        let stmt = match matches.as_slice() {
            [one] => one,
            [] => bail!("No statement in {} assigns `{lhs}`.", record.name()),
            many => bail!(
                "`{lhs}` is assigned in {} statements in {}; expected exactly 1.",
                many.len(),
                record.name()
            ),
        };

        let (token_idx, new_text) = match within {
            None => {
                let i = nm_last_token(stmt.expr)
                    .ok_or_else(|| anyhow::anyhow!("`{lhs}` has an empty right-hand side."))?;
                (i, format!("{} {text}", cb.tokens[i].text))
            }
            Some(func) => {
                let mut calls = vec![];
                collect_calls(stmt.expr, func, &cb.tokens, &mut calls);
                let call = match calls.as_slice() {
                    [one] => *one,
                    [] => bail!("`{lhs}` has no call to `{func}()`."),
                    many => bail!(
                        "`{lhs}` calls `{func}()` {} times; expected exactly 1.",
                        many.len()
                    ),
                };
                // The closing paren is the call's last token; the text goes just before it.
                let rparen = call
                    .children
                    .iter()
                    .rev()
                    .find_map(nm_last_token)
                    .filter(|&i| cb.tokens[i].token == NmtranToken::RightParen)
                    .ok_or_else(|| anyhow::anyhow!("Could not locate the end of `{func}()`."))?;
                (rparen, format!(" {text}{}", cb.tokens[rparen].text))
            }
        };

        let mut edited = self.clone();
        let cb = edited
            .code_block_at_mut(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;
        cb.tokens[token_idx].text = new_text;
        let edited = edited.reparse()?;
        self.check_new_references(&edited)?;
        check_names(&edited, text)?;
        Ok(edited)
    }

    /// Left-hand-side names assigned in `record`, upper case, base names only
    /// (`DADT(2)` gives `DADT`).
    pub fn assigned_names(&self, record: CodeRecord) -> Result<Vec<String>> {
        let record_idx = self.code_record_idx(record)?;
        let cb = self
            .code_block_at(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        let mut names: Vec<String> = vec![];
        for a in assignments {
            let base = a.lhs.split('(').next().unwrap_or(&a.lhs).to_string();
            if !names.contains(&base) {
                names.push(base);
            }
        }
        Ok(names)
    }

    /// The `MU_n` that the statement assigning `param` in `$PK` uses. The
    /// right-hand side must reference exactly one MU.
    pub fn mu_of(&self, param: &str) -> Result<String> {
        let record_idx = self.code_record_idx(CodeRecord::Pk)?;
        let cb = self
            .code_block_at(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate $PK."))?;

        let target = normalize_name(param);
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        let matches: Vec<&Assignment> = assignments.iter().filter(|a| a.lhs == target).collect();
        let stmt = match matches.as_slice() {
            [one] => one,
            [] => bail!("No statement in $PK assigns `{param}`."),
            many => bail!(
                "`{param}` is assigned in {} statements in $PK; expected exactly 1.",
                many.len()
            ),
        };

        let mut idx = vec![];
        nm_tokens(stmt.expr, &mut idx);
        let mus: BTreeSet<String> = idx
            .into_iter()
            .map(|i| &cb.tokens[i])
            .filter(|t| t.token == NmtranToken::Ident)
            .map(|t| t.text.to_uppercase())
            .filter(|name| {
                name.strip_prefix("MU_")
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            })
            .collect();
        match mus.len() {
            1 => Ok(mus.into_iter().next().unwrap()),
            0 => bail!("`{param}` doesn't use a MU_ variable, so it isn't MU-referenced."),
            _ => bail!(
                "`{param}` uses {} MU_ variables ({}); MU referencing allows exactly 1.",
                mus.len(),
                mus.into_iter().collect::<Vec<_>>().join(", ")
            ),
        }
    }

    /// Add `line` as a new statement in `record`. Placement:
    /// a `MU_n =` line goes after the last MU assignment, a line with an ETA
    /// goes after the last statement with an ETA, anything else goes after
    /// the last statement. Only top-level statements are anchors.
    pub fn add_statement(&self, record: CodeRecord, line: &str) -> Result<Model> {
        if line.contains('\n') {
            bail!("Each line must be a single statement; got a line break in `{line}`.");
        }
        let record_idx = self.code_record_idx(record)?;
        let cb = self
            .code_block_at(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;

        let line_trim = line.trim();
        let upper = line_trim.to_uppercase();
        let is_mu = upper
            .split('=')
            .next()
            .map(|lhs| lhs.trim())
            .and_then(|lhs| lhs.strip_prefix("MU_"))
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let has_eta = |toks: &[usize]| {
            toks.iter().any(|&i| {
                cb.tokens[i].token == NmtranToken::Ident
                    && cb.tokens[i].text.eq_ignore_ascii_case("ETA")
            })
        };
        let line_has_eta = {
            let lexed = crate::nmtran::lex_nmtran(line_trim, 0);
            lexed
                .iter()
                .any(|t| t.token == NmtranToken::Ident && t.text.eq_ignore_ascii_case("ETA"))
        };

        // Top-level statements with their position in cb.children.
        let statements: Vec<(usize, &NmtranNode)> = cb
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                NmtranChild::Node(n) => Some((i, n)),
                _ => None,
            })
            .collect();
        if statements.is_empty() {
            bail!(
                "{} has no statements to place the new line after.",
                record.name()
            );
        }

        let is_mu_stmt = |n: &NmtranNode| {
            n.kind == NmtranNodeKind::Assignment
                && assignment_parts(n, &cb.tokens).is_some_and(|a| {
                    a.lhs
                        .strip_prefix("MU_")
                        .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
                })
        };
        let stmt_has_eta = |n: &NmtranNode| {
            let mut idx = vec![];
            n.children.iter().for_each(|c| nm_tokens(c, &mut idx));
            has_eta(&idx)
        };

        let anchor = if is_mu {
            statements.iter().rev().find(|(_, n)| is_mu_stmt(n))
        } else if line_has_eta {
            statements.iter().rev().find(|(_, n)| stmt_has_eta(n))
        } else {
            None
        }
        .or_else(|| statements.last())
        .copied()
        .unwrap();

        let (child_pos, node) = anchor;
        let last = node
            .children
            .iter()
            .rev()
            .find_map(nm_last_token)
            .ok_or_else(|| anyhow::anyhow!("Could not locate the anchor statement's end."))?;
        // Copy the anchor line's indentation.
        let indent = child_pos
            .checked_sub(1)
            .and_then(|p| match &cb.children[p] {
                NmtranChild::Token(i) if cb.tokens[*i].token == NmtranToken::Whitespace => {
                    Some(cb.tokens[*i].text.clone())
                }
                _ => None,
            })
            .unwrap_or_default();

        let mut edited = self.clone();
        let cb = edited
            .code_block_at_mut(record_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {}.", record.name()))?;
        cb.tokens[last]
            .text
            .push_str(&format!("\n{indent}{line_trim}"));
        let edited = edited.reparse()?;
        self.check_new_references(&edited)?;
        check_names(&edited, line_trim)?;

        let before = self.code_block_statement_count(record_idx);
        let after = edited.code_block_statement_count(record_idx);
        if after != before + 1 {
            bail!("`{line_trim}` is not a single statement.");
        }
        Ok(edited)
    }

    pub(super) fn code_block_statement_count(&self, record_idx: usize) -> usize {
        self.code_block_at(record_idx)
            .map(|cb| {
                cb.children
                    .iter()
                    .filter(|c| matches!(c, NmtranChild::Node(_)))
                    .count()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WARFARIN: &str = "\
$PROBLEM warfarin
$INPUT ID TIME DV WT
$DATA data.csv IGNORE=@
$SUBROUTINE ADVAN2 TRANS2

$PK
 MU_1 = THETA(1)
 MU_2 = THETA(2)
 MU_3 = THETA(3)

; Individual Parameters
 KA = EXP(MU_1 + ETA(1))
 V  = EXP(MU_2 + ETA(2))
 CL = EXP(MU_3 + ETA(3))

 K20 = CL / V ;[1 / hr]
 S2 = V

$ERROR
 IPRED = F
 W = SQRT(SIGMA(1,1)) ; Additive Error Model
 Y = IPRED + EPS(1)

$THETA
 -0.62508    ;KA [1/hr]  ;lognormal
 1.85635    ;V  [L]     ;lognormal
 -1.90936    ;CL [L/hr]  ;lognormal

$OMEGA
 0.4194     ;IIV KA ;lognormal
 0.0177     ;IIV V  ;lognormal
 0.0796     ;IIV CL ;lognormal

$SIGMA
 1.148      ;Add [ng/mL] ;AddErr
";

    fn model() -> Model {
        Model::inner_parse(WARFARIN).unwrap()
    }

    fn refs(pairs: &[(&str, RefKind, usize)]) -> HashMap<String, ParamRef> {
        pairs
            .iter()
            .map(|(n, kind, index)| {
                (
                    n.to_string(),
                    ParamRef {
                        kind: *kind,
                        index: *index,
                        col: None,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn add_theta_appends_row() {
        let (edited, n) = model()
            .add_theta(
                &NewTheta {
                    init: 1.2,
                    comment: Some("WT-on-Vc".into()),
                    ..Default::default()
                },
                None,
                None,
            )
            .unwrap();
        assert_eq!(n, 4);
        assert_eq!(edited.thetas[3].init, 1.2);
        assert!(
            edited
                .model_content()
                .contains(";CL [L/hr]  ;lognormal\n 1.2 ;WT-on-Vc\n")
        );
    }

    #[test]
    fn add_theta_bounds_and_fix() {
        let (edited, _) = model()
            .add_theta(
                &NewTheta {
                    init: 4.0,
                    lower: Some(0.0),
                    fix: true,
                    ..Default::default()
                },
                None,
                None,
            )
            .unwrap();
        let t = &edited.thetas[3];
        assert_eq!((t.lower, t.init, t.fixed), (Some(0.0), 4.0, true));
        assert!(edited.model_content().contains("\n (0, 4) FIX\n"));
    }

    #[test]
    fn append_to_statement_adds_covariate() {
        let (m, n) = model()
            .add_theta(
                &NewTheta {
                    init: 1.2,
                    ..Default::default()
                },
                None,
                None,
            )
            .unwrap();
        let r = refs(&[("wt_v", RefKind::Theta, n)]);
        let (text, used) = resolve_placeholders("+ {wt_v.theta} * LOG(WT / 70)", &r).unwrap();
        assert_eq!(used, vec!["wt_v"]);
        let mu = m.mu_of("V").unwrap();
        assert_eq!(mu, "MU_2");
        let edited = m
            .append_to_statement(CodeRecord::Pk, &mu, None, &text)
            .unwrap();
        assert!(
            edited
                .model_content()
                .contains(" MU_2 = THETA(2) + THETA(4) * LOG(WT / 70)\n")
        );
    }

    #[test]
    fn append_within_call() {
        let edited = model()
            .append_to_statement(CodeRecord::Error, "W", Some("SQRT"), "+ IPRED**2")
            .unwrap();
        assert!(
            edited
                .model_content()
                .contains(" W = SQRT(SIGMA(1,1) + IPRED**2) ; Additive Error Model\n")
        );
    }

    #[test]
    fn append_refuses_missing_and_bad_references() {
        let m = model();
        assert!(
            m.append_to_statement(CodeRecord::Pk, "NOPE", None, "+ 1")
                .unwrap_err()
                .to_string()
                .contains("No statement in $PK assigns `NOPE`")
        );
        let err = m
            .append_to_statement(CodeRecord::Pk, "MU_2", None, "+ THETA(9)")
            .unwrap_err()
            .to_string();
        assert!(err.contains("THETA(9)"), "{err}");
    }

    #[test]
    fn statement_comment_set_add_remove() {
        let m = model()
            .set_statement_comment(
                CodeRecord::Error,
                "W",
                &Change::Set("Combined Error Model".into()),
            )
            .unwrap();
        assert!(
            m.model_content()
                .contains(" W = SQRT(SIGMA(1,1)) ; Combined Error Model\n")
        );
        let m = m
            .set_statement_comment(CodeRecord::Error, "Y", &Change::Set("obs".into()))
            .unwrap()
            .set_statement_comment(CodeRecord::Pk, "K20", &Change::Remove)
            .unwrap();
        let c = m.model_content();
        assert!(
            c.contains(" Y = IPRED + EPS(1) ; obs\n") && c.contains(" K20 = CL / V\n"),
            "{c}"
        );
    }

    #[test]
    fn mu_of_refuses_two_mus() {
        let src = WARFARIN.replace("V  = EXP(MU_2 + ETA(2))", "V  = EXP(MU_2 + MU_3 + ETA(2))");
        let m = Model::inner_parse(&src).unwrap();
        let err = m.mu_of("V").unwrap_err().to_string();
        assert!(err.contains("MU_2, MU_3"), "{err}");
    }

    #[test]
    fn placeholders_check_kind_and_declaration() {
        let r = refs(&[("tv", RefKind::Theta, 5), ("iiv", RefKind::Omega, 4)]);
        assert_eq!(
            resolve_placeholders("{iiv.mu} = {tv.theta} + {iiv.eta}", &r)
                .unwrap()
                .0,
            "MU_4 = THETA(5) + ETA(4)"
        );
        assert!(resolve_placeholders("{tv.eps}", &r).is_err());
        assert!(resolve_placeholders("{nope.theta}", &r).is_err());
        assert!(resolve_placeholders("{tv}", &r).is_err());
    }

    #[test]
    fn add_statement_places_mu_and_eta_lines() {
        let m = model()
            .add_statement(CodeRecord::Pk, "MU_4 = THETA(3)")
            .unwrap()
            .add_statement(CodeRecord::Pk, "ALAG1 = EXP(MU_4 + ETA(3))")
            .unwrap();
        let content = m.model_content();
        assert!(
            content.contains(" MU_3 = THETA(3)\n MU_4 = THETA(3)\n"),
            "{content}"
        );
        assert!(
            content.contains(" CL = EXP(MU_3 + ETA(3))\n ALAG1 = EXP(MU_4 + ETA(3))\n"),
            "{content}"
        );
    }

    #[test]
    fn add_statement_without_mu_or_eta_goes_last() {
        let m = model().add_statement(CodeRecord::Pk, "Q = 1").unwrap();
        assert!(m.model_content().contains(" S2 = V\n Q = 1\n"));
    }
}
