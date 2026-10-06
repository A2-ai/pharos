//! Removing `$THETA`, `$OMEGA` and `$SIGMA` rows. See `edit.rs` for the
//! approach: change token text on a copy, render, parse again.
//!
//! A removal takes the parameter out of every statement that uses it by
//! setting it to zero and simplifying with fixed rules: a term that becomes 0
//! leaves its sum, a factor that becomes 1 leaves its product (`EXP(0)`,
//! `x**0`), and a statement whose whole right-hand side becomes 0 is deleted,
//! along with its `$TABLE` columns. Anything else (a sign flip, a division by
//! zero, a use in an `IF` condition) is refused. Later rows are then
//! renumbered. For an OMEGA, the `MU_n` paired with the removed ETA has
//! nothing left to pair with, so its definition replaces it where it is used
//! and later `MU_k` follow their ETAs down.

use anyhow::{Result, bail};

use crate::cst::{CstChild, NmtranChild, NmtranNode, NmtranNodeKind, NodeKind};
use crate::lexer::Token;
use crate::nmtran::{NmtranSpannedToken, NmtranToken};

use super::Model;
use super::edit::{
    assignment_parts, base_name, first_token_in, indexed_references, is_nm_trivia, line_end,
    line_start, nm_first_token, nm_last_token, nm_tokens, node_tokens,
};
use super::edit_params::{
    RandomKind, Shift, locate_row, mu_number, param_node, renumber_indexed, renumber_mu,
};

/// Indexed names that are model parameters.
const PARAM_NAMES: &[&str] = &["THETA", "ETA", "EPS", "ERR", "OMEGA", "SIGMA"];

/// The references a removal takes out: `NAME(n)` or `NAME(n,n)`.
struct Target {
    refs: Vec<(&'static [&'static str], Vec<usize>)>,
    /// How messages name the parameter, e.g. `ETA(1)`.
    label: String,
}

impl Target {
    fn theta(n: usize) -> Self {
        Target {
            refs: vec![(&["THETA"], vec![n])],
            label: format!("THETA({n})"),
        }
    }

    fn random(kind: RandomKind, n: usize) -> Self {
        match kind {
            RandomKind::Omega => Target {
                refs: vec![(&["ETA"], vec![n]), (&["OMEGA"], vec![n, n])],
                label: format!("ETA({n})"),
            },
            RandomKind::Sigma => Target {
                refs: vec![(&["EPS", "ERR"], vec![n]), (&["SIGMA"], vec![n, n])],
                label: format!("EPS({n})"),
            },
        }
    }

    fn matches(&self, name: &str, idx: &[usize]) -> bool {
        self.refs
            .iter()
            .any(|(names, i)| names.contains(&name) && i.as_slice() == idx)
    }

    fn matches_node(&self, node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> bool {
        call_ref(node, tokens).is_some_and(|(name, idx)| self.matches(&name, &idx))
    }
}

/// `NAME(INT)` or `NAME(INT,INT)` as (upper-case name, indices).
fn call_ref(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> Option<(String, Vec<usize>)> {
    if node.kind != NmtranNodeKind::FunctionCall {
        return None;
    }
    let mut name = None;
    let mut idx = vec![];
    for child in &node.children {
        match child {
            NmtranChild::Token(i) if tokens[*i].token == NmtranToken::Ident && name.is_none() => {
                name = Some(tokens[*i].text.to_uppercase());
            }
            NmtranChild::Token(_) => {}
            NmtranChild::Node(args) if args.kind == NmtranNodeKind::ArgList => {
                for a in &args.children {
                    let NmtranChild::Token(i) = a else {
                        return None;
                    };
                    match tokens[*i].token {
                        NmtranToken::Int => idx.push(tokens[*i].text.parse().ok()?),
                        NmtranToken::Comma => {}
                        _ if is_nm_trivia(&tokens[*i]) => {}
                        _ => return None,
                    }
                }
            }
            NmtranChild::Node(_) => return None,
        }
    }
    let name = name?;
    (!idx.is_empty()).then_some((name, idx))
}

/// Code text of some children, comments dropped and whitespace squashed.
fn code_text<'a>(
    children: impl IntoIterator<Item = &'a NmtranChild>,
    tokens: &[NmtranSpannedToken],
) -> String {
    let mut idx = vec![];
    children.into_iter().for_each(|c| nm_tokens(c, &mut idx));
    let text: String = idx
        .into_iter()
        .filter(|&i| {
            !matches!(
                tokens[i].token,
                NmtranToken::Comment | NmtranToken::Newline | NmtranToken::Ampersand
            )
        })
        .map(|i| tokens[i].text.as_str())
        .collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a subexpression becomes once the parameter is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Null {
    /// The parameter isn't in it.
    Same,
    /// It is 0.
    Zero,
    /// It is 1.
    One,
    /// It keeps a value; token ranges were recorded for deletion.
    Edited,
}

/// One statement being simplified, for walking its right-hand side.
struct Simplify<'a> {
    tokens: &'a [NmtranSpannedToken],
    target: &'a Target,
    statement: String,
}

impl Simplify<'_> {
    fn refuse<T>(&self, why: &str) -> Result<T> {
        bail!(
            "Can't take {} out of `{}`: {why}. Edit the statement first.",
            self.target.label,
            self.statement
        )
    }

    fn unsupported<T>(&self) -> Result<T> {
        self.refuse("it is inside an expression pharos can't simplify")
    }

    fn has_target(&self, child: &NmtranChild) -> bool {
        match child {
            NmtranChild::Token(_) => false,
            NmtranChild::Node(n) => {
                self.target.matches_node(n, self.tokens)
                    || n.children.iter().any(|c| self.has_target(c))
            }
        }
    }

    /// Whether a subexpression uses a parameter other than the target.
    fn has_other_param(&self, child: &NmtranChild) -> bool {
        match child {
            NmtranChild::Token(i) => {
                let t = &self.tokens[*i];
                t.token == NmtranToken::Ident && mu_number(&t.text).is_some()
            }
            NmtranChild::Node(n) => {
                if let Some((name, idx)) = call_ref(n, self.tokens) {
                    return PARAM_NAMES.contains(&name.as_str())
                        && !self.target.matches(&name, &idx);
                }
                n.children.iter().any(|c| self.has_other_param(c))
            }
        }
    }

    /// Children that aren't trivia, parentheses or commas.
    fn operands<'n>(&self, node: &'n NmtranNode) -> Vec<&'n NmtranChild> {
        node.children
            .iter()
            .filter(|c| match c {
                NmtranChild::Token(i) => {
                    let t = &self.tokens[*i];
                    !is_nm_trivia(t)
                        && !matches!(
                            t.token,
                            NmtranToken::LeftParen | NmtranToken::RightParen | NmtranToken::Comma
                        )
                }
                NmtranChild::Node(_) => true,
            })
            .collect()
    }

    fn eval(&self, child: &NmtranChild, dels: &mut Vec<(usize, usize)>) -> Result<Null> {
        let NmtranChild::Node(node) = child else {
            return Ok(Null::Same);
        };
        if !self.has_target(child) {
            return Ok(Null::Same);
        }
        match node.kind {
            NmtranNodeKind::FunctionCall => {
                if self.target.matches_node(node, self.tokens) {
                    return Ok(Null::Zero);
                }
                let func = node
                    .children
                    .iter()
                    .find_map(nm_first_token)
                    .map(|i| self.tokens[i].text.to_uppercase())
                    .unwrap_or_default();
                let args: Vec<&NmtranChild> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        NmtranChild::Node(n) if n.kind == NmtranNodeKind::ArgList => Some(n),
                        _ => None,
                    })
                    .flat_map(|a| self.operands(a))
                    .collect();
                let [arg] = args.as_slice() else {
                    return self.refuse(&format!("it is one of several arguments of {func}()"));
                };
                match (func.as_str(), self.eval(arg, dels)?) {
                    (_, v @ (Null::Same | Null::Edited)) => Ok(v),
                    ("EXP", Null::Zero) => Ok(Null::One),
                    ("SQRT", Null::Zero) => Ok(Null::Zero),
                    ("LOG", Null::One) => Ok(Null::Zero),
                    _ => self.refuse(&format!("{func}() of it has no fixed value")),
                }
            }
            NmtranNodeKind::ParenExpr => match self.operands(node).as_slice() {
                [one] => self.eval(one, dels),
                _ => self.unsupported(),
            },
            NmtranNodeKind::UnaryExpr => {
                let parts = self.operands(node);
                let [NmtranChild::Token(op), operand] = parts.as_slice() else {
                    return self.unsupported();
                };
                match (&self.tokens[*op].token, self.eval(operand, dels)?) {
                    (_, v @ (Null::Same | Null::Edited)) => Ok(v),
                    (NmtranToken::Minus | NmtranToken::Plus, Null::Zero) => Ok(Null::Zero),
                    _ => self.refuse("removing it would leave a negated constant"),
                }
            }
            NmtranNodeKind::BinaryExpr => self.binary(node, dels),
            _ => self.unsupported(),
        }
    }

    fn binary(&self, node: &NmtranNode, dels: &mut Vec<(usize, usize)>) -> Result<Null> {
        use Null::{Edited, One, Same, Zero};
        let parts = self.operands(node);
        let [left, NmtranChild::Token(op), right] = parts.as_slice() else {
            return self.unsupported();
        };
        let (left, right) = (*left, *right);
        let mut dl = vec![];
        let mut dr = vec![];
        let a = self.eval(left, &mut dl)?;
        let b = self.eval(right, &mut dr)?;
        let (Some(lf), Some(ll), Some(rf), Some(rl)) = (
            nm_first_token(left),
            nm_last_token(left),
            nm_first_token(right),
            nm_last_token(right),
        ) else {
            return self.unsupported();
        };
        // Dropping an operand takes the operator and the spaces around it.
        let drop_left = (lf, rf - 1);
        let drop_right = (ll + 1, rl);
        let kept = |v: Null| matches!(v, Same | Edited);
        // Record the deletions behind a result.
        let mut take = |range: Option<(usize, usize)>, rest: Vec<(usize, usize)>, v: Null| {
            dels.extend(range);
            dels.extend(rest);
            Ok(v)
        };
        let op = &self.tokens[*op].token;
        match op {
            NmtranToken::Plus | NmtranToken::Minus => match (a, b) {
                (Zero, Zero) => Ok(Zero),
                (Zero, x) if *op == NmtranToken::Plus && kept(x) => {
                    take(Some(drop_left), dr, Edited)
                }
                (x, Zero) if kept(x) => take(Some(drop_right), dl, Edited),
                (Zero, _) => self.refuse("it is subtracted from, so removing it would flip a sign"),
                (One, _) | (_, One) => self.refuse("removing it would leave a constant in a sum"),
                _ => take(None, [dl, dr].concat(), merged(a, b)),
            },
            NmtranToken::Star => match (a, b) {
                (Zero, _) | (_, Zero) => {
                    let other = if a == Zero { right } else { left };
                    if self.has_other_param(other) {
                        return self.refuse("its term also uses other parameters");
                    }
                    Ok(Zero)
                }
                (One, One) => Ok(One),
                (One, _) => take(Some(drop_left), dr, Edited),
                (_, One) => take(Some(drop_right), dl, Edited),
                _ => take(None, [dl, dr].concat(), merged(a, b)),
            },
            NmtranToken::Slash => match (a, b) {
                (_, Zero) => self.refuse("removing it would divide by zero"),
                (Zero, _) => {
                    if self.has_other_param(right) {
                        return self.refuse("its term also uses other parameters");
                    }
                    Ok(Zero)
                }
                (_, One) => take(Some(drop_right), dl, Edited),
                (One, _) => self.refuse("removing it would leave 1 divided by something"),
                _ => take(None, [dl, dr].concat(), merged(a, b)),
            },
            NmtranToken::StarStar => match (a, b) {
                (_, Zero) => {
                    if self.has_other_param(left) {
                        return self.refuse("its factor also uses other parameters");
                    }
                    Ok(One)
                }
                (One, _) => {
                    if self.has_other_param(right) {
                        return self.refuse("its factor also uses other parameters");
                    }
                    Ok(One)
                }
                (Zero, _) => self.refuse("removing it would raise 0 to a power"),
                (_, One) => self.refuse("removing it would leave a power of 1"),
                _ => take(None, [dl, dr].concat(), merged(a, b)),
            },
            _ => self.refuse("it is part of a comparison"),
        }
    }
}

/// Both operands kept: the result is edited when either side was.
fn merged(a: Null, b: Null) -> Null {
    if a == Null::Edited || b == Null::Edited {
        Null::Edited
    } else {
        Null::Same
    }
}

/// Token range of a statement's whole line or lines: indentation, the
/// statement, its comment and the newline. Refused when a line holds
/// anything else.
fn statement_lines(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> Result<(usize, usize)> {
    let mut idx = vec![];
    node.children.iter().for_each(|c| nm_tokens(c, &mut idx));
    idx.retain(|&i| !is_nm_trivia(&tokens[i]));
    let (Some(&first), Some(&last)) = (idx.first(), idx.last()) else {
        bail!("Could not locate a statement.");
    };
    let shared = || {
        bail!(
            "`{}` shares its line with other code; edit it by hand.",
            code_text(&node.children, tokens)
        )
    };
    let mut start = first;
    while start > 0 && tokens[start - 1].token != NmtranToken::Newline {
        if tokens[start - 1].token != NmtranToken::Whitespace {
            return shared();
        }
        start -= 1;
    }
    let mut end = last;
    while end + 1 < tokens.len() && tokens[end + 1].token != NmtranToken::Newline {
        if !matches!(
            tokens[end + 1].token,
            NmtranToken::Whitespace | NmtranToken::Comment
        ) {
            return shared();
        }
        end += 1;
    }
    if end + 1 < tokens.len() {
        end += 1;
    }
    Ok((start, end))
}

/// Assignment statements in a code block, including those inside IF/DO bodies.
fn assignment_nodes<'a>(children: &'a [NmtranChild], out: &mut Vec<&'a NmtranNode>) {
    for child in children {
        if let NmtranChild::Node(node) = child {
            if node.kind == NmtranNodeKind::Assignment {
                out.push(node);
            } else {
                assignment_nodes(&node.children, out);
            }
        }
    }
}

/// Whether a code block's tokens use `name` as a variable.
fn uses_name(tokens: &[NmtranSpannedToken], name: &str) -> bool {
    tokens
        .iter()
        .any(|t| t.token == NmtranToken::Ident && t.text.eq_ignore_ascii_case(name))
}

/// `n` for a `$TABLE` column `ETAn`.
fn eta_column(word: &str) -> Option<usize> {
    word.strip_prefix("ETA")
        .filter(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))?
        .parse()
        .ok()
}

impl Model {
    fn code_record_indices(&self) -> Vec<usize> {
        [&self.pk, &self.error, &self.des, &self.pred]
            .into_iter()
            .flatten()
            .map(|b| b.record_idx)
            .collect()
    }

    /// Clear token ranges in a code block: `parts` of statements and whole
    /// statement `lines`. A line ending the block also takes the rest of its
    /// line, whose trailing comment the parser keeps outside the block.
    fn clear_code(
        &mut self,
        record_idx: usize,
        parts: &[(usize, usize)],
        lines: &[(usize, usize)],
    ) {
        let Some(cb) = self.code_block_at_mut(record_idx) else {
            return;
        };
        let last = cb.tokens.len().saturating_sub(1);
        for &(start, end) in parts.iter().chain(lines) {
            for i in start..=end {
                cb.tokens[i].text.clear();
            }
        }
        let to_end = lines
            .iter()
            .any(|&(_, end)| end == last && cb.tokens[last].token != NmtranToken::Newline);
        if !to_end {
            return;
        }
        let after: Vec<usize> = self.cst.children[record_idx + 1..]
            .iter()
            .map_while(|c| match c {
                CstChild::Token(i) => Some(*i),
                _ => None,
            })
            .collect();
        for i in after {
            let token = self.tokens[i].token.clone();
            if !matches!(token, Token::Whitespace | Token::Comment | Token::Newline) {
                break;
            }
            self.tokens[i].text.clear();
            if token == Token::Newline {
                break;
            }
        }
    }

    /// Set the target to zero in every statement that uses it (see the module
    /// docs). Deleted statements take their `$TABLE` columns with them.
    fn take_out(&self, target: &Target) -> Result<Model> {
        let mut edited = self.clone();
        let mut deleted: Vec<String> = vec![];
        for record_idx in self.code_record_indices() {
            let Some(cb) = self.code_block_at(record_idx) else {
                continue;
            };
            let mut parts = vec![];
            let mut lines = vec![];
            take_out_of(&cb.children, &cb.tokens, target, &mut parts, &mut lines)?;
            edited.clear_code(
                record_idx,
                &parts,
                &lines.iter().map(|l| l.1).collect::<Vec<_>>(),
            );
            deleted.extend(lines.into_iter().map(|l| l.0));
        }
        let edited = edited.reparse()?;

        for cb in edited.code_blocks() {
            for (name, idx) in indexed_references(&cb.tokens) {
                if target.matches(&name, &idx) {
                    bail!(
                        "{} is still used in the code after taking it out.",
                        target.label
                    );
                }
            }
        }
        for name in &deleted {
            if edited
                .code_blocks()
                .iter()
                .any(|cb| uses_name(&cb.tokens, name))
            {
                bail!(
                    "Taking out {} deletes the statement assigning `{name}`, but `{name}` is \
                     used or assigned elsewhere in the code. Edit those statements first.",
                    target.label
                );
            }
        }
        let mut columns = vec![];
        for table in &edited.tables {
            let CstChild::Node(record) = &edited.cst.children[table.record_idx] else {
                continue;
            };
            for (word, node) in edited.table_columns(record) {
                if deleted.contains(&word) {
                    columns.push(node.clone());
                }
            }
        }
        if columns.is_empty() {
            return Ok(edited);
        }
        let mut out = edited.clone();
        for node in &columns {
            out.blank_node(node);
        }
        out.reparse()
    }

    /// Replace `MU_n` with its definition wherever it's used and delete the
    /// statement defining it. No-op when the model has no `MU_n`.
    fn inline_mu(&self, n: usize) -> Result<Model> {
        let mu = format!("MU_{n}");
        let mut defs = vec![];
        for record_idx in self.code_record_indices() {
            let Some(cb) = self.code_block_at(record_idx) else {
                continue;
            };
            let mut nodes = vec![];
            assignment_nodes(&cb.children, &mut nodes);
            for node in nodes {
                if assignment_parts(node, &cb.tokens).is_some_and(|a| a.lhs == mu) {
                    defs.push((record_idx, node));
                }
            }
        }
        let (def_record, def) = match defs.as_slice() {
            [] => return Ok(self.clone()),
            [one] => *one,
            many => bail!(
                "`{mu}` is assigned in {} statements; edit them by hand.",
                many.len()
            ),
        };
        let cb = self.code_block_at(def_record).unwrap();
        let expr = assignment_parts(def, &cb.tokens).unwrap().expr;
        let rhs = code_text([expr], &cb.tokens);
        let atomic = match expr {
            NmtranChild::Token(_) => true,
            NmtranChild::Node(node) => node.kind == NmtranNodeKind::FunctionCall,
        };
        let line = statement_lines(def, &cb.tokens)?;

        let mut edited = self.clone();
        for record_idx in self.code_record_indices() {
            let Some(cb) = self.code_block_at(record_idx) else {
                continue;
            };
            let in_def = |i: usize| record_idx == def_record && i >= line.0 && i <= line.1;
            let sig: Vec<usize> = (0..cb.tokens.len())
                .filter(|&i| !is_nm_trivia(&cb.tokens[i]))
                .collect();
            let mut replace = vec![];
            for (k, &i) in sig.iter().enumerate() {
                let t = &cb.tokens[i];
                if t.token != NmtranToken::Ident || !t.text.eq_ignore_ascii_case(&mu) || in_def(i) {
                    continue;
                }
                // Bare when it's a whole argument or a whole right-hand side.
                let before = k.checked_sub(1).map(|p| &cb.tokens[sig[p]].token);
                let after = sig.get(k + 1).map(|&p| &cb.tokens[p].token);
                let alone = match before {
                    Some(NmtranToken::LeftParen | NmtranToken::Comma) => {
                        matches!(after, Some(NmtranToken::RightParen | NmtranToken::Comma))
                    }
                    Some(NmtranToken::Equals) => cb.tokens[i + 1..]
                        .iter()
                        .take_while(|t| t.token != NmtranToken::Newline)
                        .all(|t| t.token.is_trivia()),
                    _ => false,
                };
                let text = if atomic || alone {
                    rhs.clone()
                } else {
                    format!("({rhs})")
                };
                replace.push((i, text));
            }
            let ecb = edited.code_block_at_mut(record_idx).unwrap();
            for (i, text) in replace {
                ecb.tokens[i].text = text;
            }
        }
        edited.clear_code(def_record, &[], &[line]);
        edited.reparse()
    }

    /// Delete one row from a `$THETA`, `$OMEGA` or `$SIGMA` record: its whole
    /// line when nothing else is on it, the whole record when it's the only row.
    fn delete_row(&self, record_idx: usize, param_child_idx: usize, only: bool) -> Result<Model> {
        if only {
            return self.remove_record(record_idx);
        }
        let record = self.record_node(record_idx)?;
        let param = param_node(record, param_child_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate the row."))?
            .clone();
        let mut toks = vec![];
        node_tokens(&param, &mut toks);
        let (Some(&first), Some(&last)) = (toks.first(), toks.last()) else {
            bail!("Could not locate the row.");
        };
        let start = line_start(&self.tokens, first);
        let end = line_end(&self.tokens, last);
        let shared = record.children.iter().enumerate().any(|(k, c)| {
            k != param_child_idx
                && matches!(c, CstChild::Node(n) if n.kind == NodeKind::Param
                    && first_token_in(n).is_some_and(|t| t >= start && t <= end))
        });
        let mut edited = self.clone();
        if shared {
            edited.blank_node(&param);
        } else if self.tokens[start].token == Token::ControlRecord {
            // The row is on the record's first line: keep the record name.
            let stop = if self.tokens[end].token == Token::Newline {
                end - 1
            } else {
                end
            };
            for i in start + 1..=stop {
                edited.tokens[i].text.clear();
            }
        } else {
            for i in start..=end {
                edited.tokens[i].text.clear();
            }
        }
        edited.reparse()
    }

    /// Remove THETA(`index`): take it out of the code, delete its row and
    /// renumber the later THETAs.
    pub fn remove_theta(&self, index: usize) -> Result<Model> {
        let n = self.thetas.len();
        if index == 0 || index > n {
            bail!("There is no THETA({index}): the model has {n} THETAs.");
        }
        let row = &self.thetas[index - 1];
        let repeated = self.thetas.iter().enumerate().any(|(k, t)| {
            k != index - 1
                && t.record_idx == row.record_idx
                && t.param_child_idx == row.param_child_idx
        });
        if repeated {
            bail!("THETA({index}) is part of a repeated row; edit it by hand.");
        }

        let mut edited = self.take_out(&Target::theta(index))?;
        for record in edited.code_record_indices() {
            if let Some(cb) = edited.code_block_at_mut(record) {
                renumber_indexed(&mut cb.tokens, "THETA", index + 1, Shift::Down);
            }
        }
        let edited = edited.reparse()?;
        let row = &edited.thetas[index - 1];
        let only = edited
            .thetas
            .iter()
            .filter(|t| t.record_idx == row.record_idx)
            .count()
            == 1;
        let edited = edited.delete_row(row.record_idx, row.param_child_idx, only)?;
        if edited.thetas.len() != n - 1 {
            bail!("Removing THETA({index}) did not remove exactly one row.");
        }
        self.check_new_references(&edited)?;
        Ok(edited)
    }

    /// Remove the diagonal OMEGA or SIGMA row `index` (its ETA or EPS number):
    /// take it out of the code, delete its row and renumber the later ones.
    pub fn remove_random(&self, kind: RandomKind, index: usize) -> Result<Model> {
        let element = kind.element(index, index);
        let loc = locate_row(self.blocks(kind), kind, index)?;
        if loc.in_block {
            bail!("{element} is in a BLOCK; remove the block's rows by hand.");
        }
        let repeated = loc
            .block
            .parameters
            .iter()
            .filter(|p| p.value_idx == loc.param.value_idx)
            .count()
            > 1;
        if repeated {
            bail!("{element} is part of a repeated value; edit it by hand.");
        }
        let before = self.row_count(kind);

        let mut edited = self.take_out(&Target::random(kind, index))?;
        if kind == RandomKind::Omega {
            edited = edited.inline_mu(index)?;
        }
        let names: &[&str] = match kind {
            RandomKind::Omega => &["ETA", "OMEGA"],
            RandomKind::Sigma => &["EPS", "ERR", "SIGMA"],
        };
        for record in edited.code_record_indices() {
            if let Some(cb) = edited.code_block_at_mut(record) {
                for name in names {
                    renumber_indexed(&mut cb.tokens, name, index + 1, Shift::Down);
                }
                if kind == RandomKind::Omega {
                    renumber_mu(&mut cb.tokens, index + 1, Shift::Down);
                }
            }
        }
        let mut edited = edited.reparse()?;
        if kind == RandomKind::Omega {
            edited = edited.renumber_eta_columns(index)?;
        }

        let loc = locate_row(edited.blocks(kind), kind, index)?;
        let (record_idx, child_idx) = (loc.block.record_idx, loc.param.param_child_idx);
        let rows_in_record: usize = edited
            .blocks(kind)
            .iter()
            .filter(|b| b.record_idx == record_idx)
            .map(|b| b.parameters.len())
            .sum();
        let edited = edited.delete_row(record_idx, child_idx, rows_in_record == 1)?;
        if edited.row_count(kind) != before - 1 {
            bail!("Removing {element} did not remove exactly one row.");
        }
        self.check_new_references(&edited)?;
        Ok(edited)
    }

    /// Drop `ETAn` columns from every `$TABLE` and renumber later `ETAk`.
    fn renumber_eta_columns(&self, n: usize) -> Result<Model> {
        let mut drop = vec![];
        let mut rename = vec![];
        for table in &self.tables {
            let CstChild::Node(record) = &self.cst.children[table.record_idx] else {
                continue;
            };
            for (word, node) in self.table_columns(record) {
                match eta_column(&word) {
                    Some(k) if k == n => drop.push(node.clone()),
                    Some(k) if k > n => {
                        if let Some(i) = first_token_in(node) {
                            rename.push((i, format!("ETA{}", k - 1)));
                        }
                    }
                    _ => {}
                }
            }
        }
        if drop.is_empty() && rename.is_empty() {
            return Ok(self.clone());
        }
        let mut edited = self.clone();
        for (i, text) in rename {
            edited.tokens[i].text = text;
        }
        for node in &drop {
            edited.blank_node(node);
        }
        edited.reparse()
    }
}

/// Collect the deletions that take `target` out of a list of statements:
/// parts of statements in `dels`, and whole statements in `deleted` with the
/// names they assign.
fn take_out_of(
    children: &[NmtranChild],
    tokens: &[NmtranSpannedToken],
    target: &Target,
    dels: &mut Vec<(usize, usize)>,
    deleted: &mut Vec<(String, (usize, usize))>,
) -> Result<()> {
    for child in children {
        let NmtranChild::Node(node) = child else {
            continue;
        };
        let simplify = Simplify {
            tokens,
            target,
            statement: code_text(&node.children, tokens),
        };
        match node.kind {
            NmtranNodeKind::Assignment => {
                let Some(a) = assignment_parts(node, tokens) else {
                    continue;
                };
                let mut d = vec![];
                match simplify.eval(a.expr, &mut d)? {
                    Null::Same => {}
                    Null::Edited => dels.extend(d),
                    Null::Zero => {
                        deleted.push((base_name(&a.lhs), statement_lines(node, tokens)?));
                    }
                    Null::One => {
                        return simplify.refuse("the right-hand side would become 1");
                    }
                }
            }
            NmtranNodeKind::If | NmtranNodeKind::DoWhile => {
                for part in &node.children {
                    match part {
                        NmtranChild::Node(n)
                            if matches!(
                                n.kind,
                                NmtranNodeKind::Assignment
                                    | NmtranNodeKind::If
                                    | NmtranNodeKind::DoWhile
                            ) =>
                        {
                            take_out_of(std::slice::from_ref(part), tokens, target, dels, deleted)?;
                        }
                        other if simplify.has_target(other) => {
                            return simplify.refuse("it is used in a condition");
                        }
                        _ => {}
                    }
                }
            }
            _ if simplify.has_target(child) => {
                return simplify.refuse("it isn't in an assignment");
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "\
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
 S2 = V

$ERROR
 IPRED = F
 W = SQRT(SIGMA(1,1))
 Y = IPRED + EPS(1)
 IWRES = (DV - IPRED) / W

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

$TABLE ID KA V CL ETA1 ETA2 ETA3 NOPRINT FILE=sdtab
";

    const THETAS: &str = " -1.90936    ;CL [L/hr]  ;lognormal\n";

    /// MODEL with each `(old, new)` replaced.
    fn with(edits: &[(&str, &str)]) -> Model {
        let mut text = MODEL.to_string();
        for (old, new) in edits {
            assert!(text.contains(old), "{old}");
            text = text.replacen(old, new, 1);
        }
        Model::inner_parse(&text).unwrap()
    }

    fn model() -> Model {
        with(&[])
    }

    fn err(r: Result<Model>) -> String {
        match r {
            Ok(m) => panic!("expected an error, got:\n{}", m.model_content()),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn remove_omega_inlines_its_mu_and_renumbers() {
        let m = model().remove_random(RandomKind::Omega, 1).unwrap();
        let c = m.model_content();
        assert!(
            c.contains(
                "$PK\n MU_1 = THETA(2)\n MU_2 = THETA(3)\n\n; Individual Parameters\n \
                 KA = EXP(THETA(1))\n V  = EXP(MU_1 + ETA(1))\n CL = EXP(MU_2 + ETA(2))\n"
            ),
            "{c}"
        );
        assert!(
            c.contains("$OMEGA\n 0.0177     ;IIV V  ;lognormal\n 0.0796     ;IIV CL ;lognormal\n"),
            "{c}"
        );
        assert!(c.contains("$TABLE ID KA V CL ETA1 ETA2 NOPRINT"), "{c}");
        assert_eq!(m.eta_count(), 2);
    }

    #[test]
    fn remove_omega_inlines_a_compound_mu() {
        let m = with(&[(" MU_1 = THETA(1)\n", " MU_1 = THETA(1) + THETA(4) * WT\n")])
            .remove_random(RandomKind::Omega, 1)
            .unwrap();
        assert!(
            m.model_content()
                .contains(" KA = EXP(THETA(1) + THETA(4) * WT)\n"),
            "{}",
            m.model_content()
        );
    }

    #[test]
    fn remove_omega_drops_an_exp_factor() {
        let m = with(&[(
            " CL = EXP(MU_3 + ETA(3))\n",
            " TVCL = EXP(MU_3)\n CL = TVCL * EXP(ETA(3))\n",
        )])
        .remove_random(RandomKind::Omega, 3)
        .unwrap();
        let c = m.model_content();
        assert!(c.contains(" TVCL = EXP(THETA(3))\n CL = TVCL\n"), "{c}");
    }

    #[test]
    fn remove_theta_drops_terms_and_factors() {
        let m = with(&[
            (
                " MU_2 = THETA(2)\n",
                " MU_2 = THETA(2) + THETA(4) * LOG(WT/70)\n",
            ),
            (
                " MU_3 = THETA(3)\n",
                " MU_3 = LOG(THETA(3) * (WT/70)**THETA(5))\n",
            ),
            (THETAS, " -1.90936\n 0.75\n 0.75\n"),
        ]);
        let m = m.remove_theta(4).unwrap();
        let c = m.model_content();
        assert!(
            c.contains(" MU_2 = THETA(2)\n MU_3 = LOG(THETA(3) * (WT/70)**THETA(4))\n"),
            "{c}"
        );
        assert!(c.contains(" -1.90936\n 0.75\n\n$OMEGA"), "{c}");

        let c = m.remove_theta(4).unwrap().model_content();
        assert!(c.contains(" MU_3 = LOG(THETA(3))\n"), "{c}");
    }

    #[test]
    fn remove_theta_deletes_a_statement_and_its_table_column() {
        let m = with(&[
            (" S2 = V\n", " S2 = V\n ALAG1 = THETA(4) ; lag\n"),
            (THETAS, " -1.90936\n 0.5 ;ALAG1\n"),
            ("$TABLE ID KA", "$TABLE ID ALAG1 KA"),
        ])
        .remove_theta(4)
        .unwrap();
        let c = m.model_content();
        assert!(!c.contains("ALAG1"), "{c}");
        assert!(c.contains(" S2 = V\n\n$ERROR"), "{c}");
        assert!(c.contains("$TABLE ID KA V"), "{c}");
        assert_eq!(m.thetas.len(), 3);
    }

    #[test]
    fn edit_at_end_of_block_keeps_its_comment_and_spacing() {
        let m = with(&[(" S2 = V\n", " S2 = V - ETA(2) ; scale\n")])
            .remove_random(RandomKind::Omega, 2)
            .unwrap();
        let c = m.model_content();
        assert!(c.contains(" S2 = V ; scale\n\n$ERROR"), "{c}");
    }

    #[test]
    fn remove_theta_renumbers_later_thetas() {
        let m = with(&[
            (
                " MU_1 = THETA(1)\n MU_2 = THETA(2)\n MU_3 = THETA(3)\n",
                " MU_1 = THETA(2)\n MU_2 = THETA(3)\n MU_3 = THETA(4)\n",
            ),
            ("$THETA\n", "$THETA\n 0.1 ;unused\n"),
        ])
        .remove_theta(1)
        .unwrap();
        let c = m.model_content();
        assert!(
            c.contains(" MU_1 = THETA(1)\n MU_2 = THETA(2)\n MU_3 = THETA(3)\n"),
            "{c}"
        );
        assert!(c.contains("$THETA\n -0.62508"), "{c}");
    }

    #[test]
    fn remove_theta_refuses_a_used_mu() {
        let e = err(model().remove_theta(2));
        assert!(e.contains("`MU_2`"), "{e}");
    }

    #[test]
    fn remove_sigma_after_switching_error_model() {
        let m = with(&[
            (
                " W = SQRT(SIGMA(1,1))\n Y = IPRED + EPS(1)\n",
                " W = SQRT(SIGMA(1,1) + IPRED**2 * SIGMA(2,2))\n Y = IPRED + EPS(1) + IPRED * EPS(2)\n",
            ),
            (";AddErr\n", ";AddErr\n 0.04 ;Prop\n"),
        ]);
        let out = m.remove_random(RandomKind::Sigma, 1).unwrap();
        let c = out.model_content();
        assert!(
            c.contains(" W = SQRT(IPRED**2 * SIGMA(1,1))\n Y = IPRED + IPRED * EPS(1)\n"),
            "{c}"
        );
        assert!(c.contains("$SIGMA\n 0.04 ;Prop\n"), "{c}");
        assert_eq!(out.eps_count(), 1);
    }

    #[test]
    fn refusals() {
        let e = err(model().remove_random(RandomKind::Sigma, 1));
        assert!(e.contains("`W`"), "{e}");

        let e = err(with(&[
            (" S2 = V\n", " S2 = V / THETA(4)\n"),
            (THETAS, " -1.90936\n 2\n"),
        ])
        .remove_theta(4));
        assert!(e.contains("divide by zero"), "{e}");

        let e =
            err(with(&[(" S2 = V\n", " S2 = ETA(2) - V\n")]).remove_random(RandomKind::Omega, 2));
        assert!(e.contains("flip a sign"), "{e}");

        let e = err(
            with(&[(" S2 = V\n", " S2 = V\n IF (ETA(2).GT.0) S2 = V * 2\n")])
                .remove_random(RandomKind::Omega, 2),
        );
        assert!(e.contains("condition"), "{e}");

        let e = err(with(&[(
            "$OMEGA\n 0.4194     ;IIV KA ;lognormal\n 0.0177     ;IIV V  ;lognormal\n",
            "$OMEGA BLOCK(2)\n 0.4194\n 0.01 0.0177\n$OMEGA\n",
        )])
        .remove_random(RandomKind::Omega, 1));
        assert!(e.contains("BLOCK"), "{e}");

        let e = err(with(&[(
            " CL = EXP(MU_3 + ETA(3))\n",
            " CL = EXP(MU_3 + ETA(3) * THETA(1))\n",
        )])
        .remove_random(RandomKind::Omega, 3));
        assert!(e.contains("other parameters"), "{e}");
    }

    #[test]
    fn remove_only_row_removes_record() {
        let m = with(&[(
            " 0.0796     ;IIV CL ;lognormal\n",
            "$OMEGA 0.0796 FIX ;IIV CL\n",
        )]);
        let out = m.remove_random(RandomKind::Omega, 3).unwrap();
        let c = out.model_content();
        assert!(!c.contains("IIV CL"), "{c}");
        assert!(c.contains(" CL = EXP(THETA(3))\n"), "{c}");
        assert!(c.contains("$TABLE ID KA V CL ETA1 ETA2 NOPRINT"), "{c}");
        assert_eq!(out.eta_count(), 2);
    }

    #[test]
    fn remove_row_from_one_line_record() {
        let m = with(&[(
            "$OMEGA\n 0.4194     ;IIV KA ;lognormal\n 0.0177     ;IIV V  ;lognormal\n 0.0796     ;IIV CL ;lognormal\n",
            "$OMEGA 0.4194 0.0177 0.0796\n",
        )]);
        let c = m
            .remove_random(RandomKind::Omega, 2)
            .unwrap()
            .model_content();
        assert!(c.contains("$OMEGA 0.4194 0.0796\n"), "{c}");
    }
}
