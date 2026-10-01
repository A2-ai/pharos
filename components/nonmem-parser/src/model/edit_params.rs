//! Edits to `$THETA`, `$OMEGA` and `$SIGMA` rows. See `edit.rs` for the
//! approach: change token text on a copy, render, parse again.

use anyhow::{Result, bail};

use crate::ast::{BlockStructure, OmegaSigmaBlock, OmegaSigmaParam};
use crate::comments::{CommentType, parse_omega_param, parse_sigma_param, parse_theta_param};
use crate::cst::{CstChild, CstNode, NodeKind};
use crate::lexer::{SpannedToken, Token};
use crate::nmtran::{NmtranSpannedToken, NmtranToken};

use super::Model;
use super::edit::{
    NewTheta, first_token_in, format_number, is_nm_trivia, last_token_in, line_comment, line_end,
    line_last_token, line_start, node_tokens,
};

/// OMEGA or SIGMA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomKind {
    Omega,
    Sigma,
}

impl RandomKind {
    fn record(self) -> &'static str {
        match self {
            RandomKind::Omega => "$OMEGA",
            RandomKind::Sigma => "$SIGMA",
        }
    }

    fn element(self, i: usize, j: usize) -> String {
        match self {
            RandomKind::Omega => format!("OMEGA({i},{j})"),
            RandomKind::Sigma => format!("SIGMA({i},{j})"),
        }
    }

    fn row(self) -> &'static str {
        match self {
            RandomKind::Omega => "ETA",
            RandomKind::Sigma => "EPS",
        }
    }
}

/// A diagonal OMEGA or SIGMA row to append.
#[derive(Debug, Clone, Default)]
pub struct NewRow {
    pub init: f64,
    pub fix: bool,
    pub comment: Option<String>,
}

/// One field of an update: leave it, set it, or remove it.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Change<T> {
    #[default]
    Keep,
    Set(T),
    Remove,
}

/// Changes to one THETA. Fields left at their default keep the current value.
#[derive(Debug, Clone, Default)]
pub struct ThetaUpdate {
    pub init: Option<f64>,
    pub lower: Change<f64>,
    pub upper: Change<f64>,
    pub fix: Option<bool>,
    pub comment: Change<String>,
}

/// Changes to one diagonal OMEGA or SIGMA element.
#[derive(Debug, Clone, Default)]
pub struct RowUpdate {
    pub init: Option<f64>,
    pub fix: Option<bool>,
    pub comment: Change<String>,
}

fn check_comment_line(comment: &str) -> Result<()> {
    if comment.contains('\n') {
        bail!("comment must be a single line.");
    }
    Ok(())
}

fn validate_theta_comment(comment: &str, validate: Option<CommentType>) -> Result<()> {
    check_comment_line(comment)?;
    if let Some(ct) = validate
        && parse_theta_param(comment, ct).is_none()
    {
        bail!(
            "comment `{comment}` doesn't parse as a {ct:?} THETA comment, and the project \
             sets `error_on_invalid = true`."
        );
    }
    Ok(())
}

fn validate_random_comment(
    kind: RandomKind,
    comment: &str,
    validate: Option<CommentType>,
) -> Result<()> {
    check_comment_line(comment)?;
    if let Some(ct) = validate {
        let ok = match kind {
            RandomKind::Omega => parse_omega_param(comment, ct).is_some(),
            RandomKind::Sigma => parse_sigma_param(comment, ct).is_some(),
        };
        if !ok {
            bail!(
                "comment `{comment}` doesn't parse as a {ct:?} {} comment, and the project \
                 sets `error_on_invalid = true`.",
                &kind.record()[1..]
            );
        }
    }
    Ok(())
}

fn check_variance(value: f64, fix: bool, what: &str) -> Result<()> {
    format_number(value, what)?;
    if value < 0.0 {
        bail!("{what} is a variance and can't be negative, got {value}.");
    }
    if value == 0.0 && !fix {
        bail!("{what} is 0, which NONMEM only allows when the row is FIX.");
    }
    Ok(())
}

/// Whether a lower-triangle matrix (row-major) is positive definite.
fn is_positive_definite(lower: &[f64], size: usize) -> bool {
    let at = |i: usize, j: usize| lower[i * (i + 1) / 2 + j];
    let mut l = vec![0.0; size * size];
    for i in 0..size {
        for j in 0..=i {
            let mut sum = at(i, j);
            for k in 0..j {
                sum -= l[i * size + k] * l[j * size + k];
            }
            if i == j {
                if sum <= 0.0 {
                    return false;
                }
                l[i * size + i] = sum.sqrt();
            } else {
                l[i * size + j] = sum / l[j * size + j];
            }
        }
    }
    true
}

/// Add 1 to every `NAME(k)` with `k >= from` in a code block's tokens.
fn renumber_indexed(tokens: &mut [NmtranSpannedToken], name: &str, from: usize) {
    let sig: Vec<usize> = (0..tokens.len())
        .filter(|&i| !is_nm_trivia(&tokens[i]))
        .collect();
    for w in sig.windows(4) {
        let [a, b, c, d] = [w[0], w[1], w[2], w[3]];
        if tokens[a].token == NmtranToken::Ident
            && tokens[a].text.eq_ignore_ascii_case(name)
            && tokens[b].token == NmtranToken::LeftParen
            && tokens[c].token == NmtranToken::Int
            && tokens[d].token == NmtranToken::RightParen
            && let Ok(k) = tokens[c].text.parse::<usize>()
            && k >= from
        {
            tokens[c].text = (k + 1).to_string();
        }
    }
}

/// The CST node of a parameter row.
fn param_node(record: &CstNode, child_idx: usize) -> Option<&CstNode> {
    match record.children.get(child_idx)? {
        CstChild::Node(n) if n.kind == NodeKind::Param => Some(n),
        _ => None,
    }
}

/// The FIX flag node inside a parameter row, if any.
fn fix_flag(param: &CstNode, tokens: &[SpannedToken]) -> Option<usize> {
    param.children.iter().position(|c| match c {
        CstChild::Node(n) if n.kind == NodeKind::Flag => first_token_in(n).is_some_and(|i| {
            let t = tokens[i].text.to_uppercase();
            t == "FIX" || t == "FIXED"
        }),
        _ => false,
    })
}

/// Indentation to use for a new line placed after the line holding token `i`.
fn line_indent(tokens: &[SpannedToken], i: usize) -> String {
    let start = line_start(tokens, i);
    match tokens[start].token {
        Token::Whitespace => tokens[start].text.clone(),
        _ => " ".to_string(),
    }
}

/// Set a line's comment: replace it, add it, or remove it.
fn set_line_comment(tokens: &mut [SpannedToken], i: usize, comment: &Change<String>) {
    match comment {
        Change::Keep => {}
        Change::Set(c) => match line_comment(tokens, i) {
            Some(ci) => tokens[ci].text = format!(";{c}"),
            None => {
                let last = line_last_token(tokens, i);
                tokens[last].text.push_str(&format!(" ;{c}"));
            }
        },
        Change::Remove => {
            if let Some(ci) = line_comment(tokens, i) {
                tokens[ci].text.clear();
                if ci > 0 && tokens[ci - 1].token == Token::Whitespace {
                    tokens[ci - 1].text.clear();
                }
            }
        }
    }
}

/// Where a diagonal row sits: its block and its parameter within the block.
struct RowLocation<'a> {
    block: &'a OmegaSigmaBlock,
    param: &'a OmegaSigmaParam,
    in_block: bool,
}

fn locate_row(
    blocks: &[OmegaSigmaBlock],
    kind: RandomKind,
    index: usize,
) -> Result<RowLocation<'_>> {
    let mut offset = 0;
    for block in blocks {
        let rows = match block.structure {
            BlockStructure::Diagonal => block.parameters.len(),
            BlockStructure::Block { size } => size,
            BlockStructure::BlockSame { size, repeats } => size * repeats,
        };
        if index > offset && index <= offset + rows {
            let r = index - offset - 1;
            return match block.structure {
                BlockStructure::Diagonal => Ok(RowLocation {
                    block,
                    param: &block.parameters[r],
                    in_block: false,
                }),
                BlockStructure::Block { .. } => Ok(RowLocation {
                    block,
                    param: &block.parameters[r * (r + 1) / 2 + r],
                    in_block: true,
                }),
                BlockStructure::BlockSame { .. } => bail!(
                    "{} is in a BLOCK SAME; edit the block it repeats.",
                    kind.element(index, index)
                ),
            };
        }
        offset += rows;
    }
    bail!(
        "There is no {}: the model has {} {} rows.",
        kind.element(index, index),
        offset,
        kind.row()
    )
}

impl Model {
    fn blocks(&self, kind: RandomKind) -> &[OmegaSigmaBlock] {
        match kind {
            RandomKind::Omega => &self.omega_blocks,
            RandomKind::Sigma => &self.sigma_blocks,
        }
    }

    fn row_count(&self, kind: RandomKind) -> usize {
        match kind {
            RandomKind::Omega => self.eta_count(),
            RandomKind::Sigma => self.eps_count(),
        }
    }

    fn record_node(&self, record_idx: usize) -> Result<&CstNode> {
        match self.cst.children.get(record_idx) {
            Some(CstChild::Node(n)) => Ok(n),
            _ => bail!("Could not locate a record in the model."),
        }
    }

    /// Text to put after the last record of `kinds` (searched in order), as
    /// the start of a new record.
    fn append_record_after(&mut self, kinds: &[NodeKind], text: &str) -> Result<()> {
        for kind in kinds {
            let last = self.cst.children.iter().rev().find_map(|c| match c {
                CstChild::Node(n) if n.kind == *kind => last_token_in(n, &self.tokens),
                _ => None,
            });
            if let Some(i) = last {
                let anchor = line_last_token(&self.tokens, i);
                self.tokens[anchor].text.push_str(&format!("\n{text}"));
                return Ok(());
            }
        }
        bail!("The model has no record to place the new one after.");
    }

    /// Add a THETA row. With `index`, it becomes THETA(index) and every later
    /// THETA reference in code is renumbered; without, it is appended.
    /// Returns the edited model and the new THETA's number.
    ///
    /// When `validate` is given, a comment must parse under that comment type.
    pub fn add_theta(
        &self,
        theta: &NewTheta,
        index: Option<usize>,
        validate: Option<CommentType>,
    ) -> Result<(Model, usize)> {
        if let Some(c) = &theta.comment {
            validate_theta_comment(c, validate)?;
        }
        let row = super::edit::format_theta_row(theta)?;
        let n = self.thetas.len();
        let Some(last) = self.thetas.last() else {
            bail!("The model has no $THETA record to add a row to.");
        };
        let index = index.unwrap_or(n + 1);
        if index == 0 || index > n + 1 {
            bail!(
                "`index` must be between 1 and {} (the model has {n} THETAs).",
                n + 1
            );
        }

        let mut edited = self.clone();
        if index == n + 1 {
            let record = self.record_node(last.record_idx)?;
            let Some(anchor) =
                super::edit::line_end_from(record, last.param_child_idx, &self.tokens)
            else {
                bail!("Could not locate the last $THETA row.");
            };
            let indent = line_indent(&self.tokens, anchor);
            edited.tokens[anchor]
                .text
                .push_str(&format!("\n{indent}{row}"));
        } else {
            let target = &self.thetas[index - 1];
            if index > 1 {
                let prev = &self.thetas[index - 2];
                if prev.record_idx == target.record_idx
                    && prev.param_child_idx == target.param_child_idx
                {
                    bail!("THETA({index}) is part of a repeated row; can't insert inside it.");
                }
            }
            let record = self.record_node(target.record_idx)?;
            let first = param_node(record, target.param_child_idx)
                .and_then(first_token_in)
                .ok_or_else(|| anyhow::anyhow!("Could not locate THETA({index})."))?;
            let indent = line_indent(&self.tokens, first);
            let original = self.tokens[first].text.clone();
            edited.tokens[first].text = format!("{row}\n{indent}{original}");
            for b in [&self.pk, &self.error, &self.des, &self.pred]
                .into_iter()
                .flatten()
            {
                if let Some(cb) = edited.code_block_at_mut(b.record_idx) {
                    renumber_indexed(&mut cb.tokens, "THETA", index);
                }
            }
        }

        let edited = edited.reparse()?;
        if edited.thetas.len() != n + 1 || edited.thetas[index - 1].init != theta.init {
            bail!("Adding the THETA row did not produce THETA({index}) as expected.");
        }
        Ok((edited, index))
    }

    /// Change one THETA. Values change in place; adding or removing a bound
    /// or FIX rewrites the row.
    pub fn update_theta(
        &self,
        index: usize,
        update: &ThetaUpdate,
        validate: Option<CommentType>,
    ) -> Result<Model> {
        let n = self.thetas.len();
        if index == 0 || index > n {
            bail!("There is no THETA({index}): the model has {n} THETAs.");
        }
        let t = &self.thetas[index - 1];
        let shared = self.thetas.iter().enumerate().any(|(i, o)| {
            i != index - 1 && o.record_idx == t.record_idx && o.param_child_idx == t.param_child_idx
        });
        if shared {
            bail!("THETA({index}) is part of a repeated row; edit it by hand.");
        }

        let pick = |c: &Change<f64>, current: Option<f64>| match c {
            Change::Keep => current,
            Change::Set(v) => Some(*v),
            Change::Remove => None,
        };
        let new = NewTheta {
            init: update.init.unwrap_or(t.init),
            lower: pick(&update.lower, t.lower),
            upper: pick(&update.upper, t.upper),
            fix: update.fix.unwrap_or(t.fixed),
            comment: None,
        };
        let param_text = super::edit::format_theta_row(&new)?;
        if let Change::Set(c) = &update.comment {
            validate_theta_comment(c, validate)?;
        }
        if update.comment != Change::Keep {
            let line = line_start(&self.tokens, t.init_idx);
            let on_line = self
                .thetas
                .iter()
                .filter(|o| line_start(&self.tokens, o.init_idx) == line)
                .count();
            if on_line > 1 {
                bail!("THETA({index}) shares its line and comment with another THETA.");
            }
        }

        let structural = new.lower.is_some() != t.lower.is_some()
            || new.upper.is_some() != t.upper.is_some()
            || new.fix != t.fixed
            || (t.upper.is_some() && t.lower_idx.is_none());

        let mut edited = self.clone();
        if structural {
            let record = self.record_node(t.record_idx)?;
            let param = param_node(record, t.param_child_idx)
                .ok_or_else(|| anyhow::anyhow!("Could not locate THETA({index})."))?;
            let mut toks = vec![];
            node_tokens(param, &mut toks);
            for (k, i) in toks.into_iter().enumerate() {
                edited.tokens[i].text = if k == 0 {
                    param_text.clone()
                } else {
                    String::new()
                };
            }
        } else {
            if let Some(v) = update.init {
                edited.tokens[t.init_idx].text = format_number(v, "init")?;
            }
            if let (Change::Set(v), Some(i)) = (&update.lower, t.lower_idx) {
                edited.tokens[i].text = format_number(*v, "lower")?;
            }
            if let (Change::Set(v), Some(i)) = (&update.upper, t.upper_idx) {
                edited.tokens[i].text = format_number(*v, "upper")?;
            }
        }
        set_line_comment(&mut edited.tokens, t.init_idx, &update.comment);

        let edited = edited.reparse()?;
        let e = edited
            .thetas
            .get(index - 1)
            .ok_or_else(|| anyhow::anyhow!("The edit lost THETA({index})."))?;
        if edited.thetas.len() != n
            || e.init != new.init
            || e.lower != new.lower
            || e.upper != new.upper
            || e.fixed != new.fix
        {
            bail!("Updating THETA({index}) did not give the expected row.");
        }
        Ok(edited)
    }

    /// Append a diagonal OMEGA or SIGMA row. It joins the last record when
    /// that record is a plain diagonal list; otherwise it starts a new record.
    /// Returns the edited model and the new row's ETA/EPS number.
    pub fn add_random(
        &self,
        kind: RandomKind,
        row: &NewRow,
        validate: Option<CommentType>,
    ) -> Result<(Model, usize)> {
        check_variance(row.init, row.fix, "init")?;
        let mut text = format_number(row.init, "init")?;
        if row.fix {
            text.push_str(" FIX");
        }
        if let Some(c) = &row.comment {
            validate_random_comment(kind, c, validate)?;
            text.push_str(&format!(" ;{c}"));
        }

        let before = self.row_count(kind);
        let mut edited = self.clone();
        let joinable = self.blocks(kind).last().filter(|b| {
            b.structure == BlockStructure::Diagonal
                && !b.fixed
                && b.parametrization.is_none()
                && !b.parameters.is_empty()
        });
        match joinable {
            Some(block) => {
                let p = block.parameters.last().unwrap();
                let anchor = line_last_token(&self.tokens, p.value_idx);
                let indent = line_indent(&self.tokens, p.value_idx);
                edited.tokens[anchor]
                    .text
                    .push_str(&format!("\n{indent}{text}"));
            }
            None => {
                let after: &[NodeKind] = match kind {
                    RandomKind::Omega => &[NodeKind::Omega, NodeKind::Theta],
                    RandomKind::Sigma => &[NodeKind::Sigma, NodeKind::Omega, NodeKind::Theta],
                };
                edited.append_record_after(after, &format!("{} {text}", kind.record()))?;
            }
        }

        let edited = edited.reparse()?;
        let n = edited.row_count(kind);
        if n != before + 1 {
            bail!(
                "Adding the {} row did not produce exactly one new row.",
                kind.record()
            );
        }
        Ok((edited, n))
    }

    /// Change one diagonal OMEGA or SIGMA element, `index` being its ETA/EPS
    /// number.
    pub fn update_random(
        &self,
        kind: RandomKind,
        index: usize,
        update: &RowUpdate,
        validate: Option<CommentType>,
    ) -> Result<Model> {
        let loc = locate_row(self.blocks(kind), kind, index)?;
        let element = kind.element(index, index);
        let shared = loc
            .block
            .parameters
            .iter()
            .filter(|p| p.value_idx == loc.param.value_idx)
            .count()
            > 1;
        if shared {
            bail!("{element} is part of a repeated value; edit it by hand.");
        }
        let record = self.record_node(loc.block.record_idx)?;
        let param = param_node(record, loc.param.param_child_idx)
            .ok_or_else(|| anyhow::anyhow!("Could not locate {element}."))?;
        let fixed_now = loc.block.fixed || fix_flag(param, &self.tokens).is_some();

        if update.fix.is_some() && loc.in_block {
            bail!("{element} is in a BLOCK; FIX applies to the whole block.");
        }
        let fix = update.fix.unwrap_or(fixed_now);
        let init = update.init.unwrap_or(loc.param.value);
        check_variance(init, fix, "init")?;
        if loc.in_block && update.init.is_some() {
            let BlockStructure::Block { size } = loc.block.structure else {
                unreachable!()
            };
            let mut values: Vec<f64> = loc.block.parameters.iter().map(|p| p.value).collect();
            let pos = values
                .iter()
                .zip(&loc.block.parameters)
                .position(|(_, p)| p.value_idx == loc.param.value_idx)
                .unwrap();
            values[pos] = init;
            if !is_positive_definite(&values, size) {
                bail!("With {element} = {init} the BLOCK is not positive definite.");
            }
        }
        if let Change::Set(c) = &update.comment {
            validate_random_comment(kind, c, validate)?;
        }
        if update.comment != Change::Keep {
            let line = line_start(&self.tokens, loc.param.value_idx);
            let on_line = self
                .blocks(kind)
                .iter()
                .flat_map(|b| &b.parameters)
                .filter(|p| line_start(&self.tokens, p.value_idx) == line)
                .count();
            if on_line > 1 {
                bail!("{element} shares its line and comment with another value.");
            }
        }

        let mut edited = self.clone();
        if let Some(v) = update.init {
            edited.tokens[loc.param.value_idx].text = format_number(v, "init")?;
        }
        match (update.fix, fixed_now) {
            (Some(true), false) => {
                let last = last_token_in(param, &self.tokens)
                    .ok_or_else(|| anyhow::anyhow!("Could not locate {element}."))?;
                edited.tokens[last].text.push_str(" FIX");
            }
            (Some(false), true) => {
                let Some(flag_pos) = fix_flag(param, &self.tokens) else {
                    bail!("{element} is fixed by its record; unfix the record by hand.");
                };
                let CstChild::Node(flag) = &param.children[flag_pos] else {
                    unreachable!()
                };
                let mut toks = vec![];
                node_tokens(flag, &mut toks);
                if flag_pos > 0
                    && let CstChild::Token(ws) = param.children[flag_pos - 1]
                    && self.tokens[ws].token == Token::Whitespace
                {
                    toks.push(ws);
                }
                for i in toks {
                    edited.tokens[i].text.clear();
                }
            }
            _ => {}
        }
        set_line_comment(&mut edited.tokens, loc.param.value_idx, &update.comment);

        let edited = edited.reparse()?;
        let check = locate_row(edited.blocks(kind), kind, index)?;
        if check.param.value != init || edited.row_count(kind) != self.row_count(kind) {
            bail!("Updating {element} did not give the expected value.");
        }
        Ok(edited)
    }

    /// Turn consecutive diagonal OMEGA rows `first..first + size` into one
    /// `$OMEGA BLOCK(size)`, written one value per line. Rows past the last
    /// existing ETA are created.
    ///
    /// `init` and `comment` hold every element in lower-triangle order.
    /// `None` keeps an existing diagonal's value or comment; a `Some("")`
    /// comment means none.
    pub fn add_omega_block(
        &self,
        first: usize,
        size: usize,
        init: &[Option<f64>],
        comment: &[Option<String>],
        fix: bool,
        validate: Option<CommentType>,
    ) -> Result<Model> {
        let kind = RandomKind::Omega;
        let n_elem = size * (size + 1) / 2;
        if size == 0 {
            bail!("`index` must name at least one row.");
        }
        if init.len() != n_elem || comment.len() != n_elem {
            bail!(
                "A BLOCK({size}) has {n_elem} values in lower-triangle order; `init` has {} \
                 and `comment` has {}.",
                init.len(),
                comment.len()
            );
        }
        let n = self.eta_count();
        if first == 0 || first > n + 1 {
            bail!(
                "The block must start at an existing ETA or at {} (the next new one).",
                n + 1
            );
        }
        let last = first + size - 1;
        let existing: Vec<RowLocation> = (first..=last.min(n))
            .map(|i| locate_row(&self.omega_blocks, kind, i))
            .collect::<Result<_>>()?;
        for (k, loc) in existing.iter().enumerate() {
            let i = first + k;
            if loc.in_block {
                bail!("ETA({i}) is already in a BLOCK; only diagonal rows can form a new block.");
            }
            if loc.block.parametrization.is_some() {
                bail!("ETA({i}) is in a record with SD/CORR/CHOLESKY; edit it by hand.");
            }
            let record = self.record_node(loc.block.record_idx)?;
            let fixed = loc.block.fixed
                || param_node(record, loc.param.param_child_idx)
                    .is_some_and(|p| fix_flag(p, &self.tokens).is_some());
            if fixed {
                bail!("OMEGA({i},{i}) is FIX; unfix it first or use `fix = TRUE` on the block.");
            }
            let shared = loc
                .block
                .parameters
                .iter()
                .filter(|p| p.value_idx == loc.param.value_idx)
                .count()
                > 1;
            if shared {
                bail!("OMEGA({i},{i}) is part of a repeated value; edit it by hand.");
            }
        }

        // Values and comments, element by element.
        let mut values: Vec<f64> = Vec::with_capacity(n_elem);
        let mut lines: Vec<String> = Vec::with_capacity(n_elem);
        for r in 0..size {
            for c in 0..=r {
                let e = r * (r + 1) / 2 + c;
                let (i, j) = (first + r, first + c);
                let old = (r == c).then(|| existing.get(r)).flatten();
                let (value, mut text) = match (init[e], old) {
                    (Some(v), _) => {
                        if r == c {
                            check_variance(v, fix, &format!("OMEGA({i},{i})"))?;
                        }
                        (v, format_number(v, &format!("OMEGA({i},{j})"))?)
                    }
                    (None, Some(loc)) => (
                        loc.param.value,
                        self.tokens[loc.param.value_idx].text.clone(),
                    ),
                    (None, None) => bail!("OMEGA({i},{j}) is new, so it needs a value."),
                };
                values.push(value);
                match (&comment[e], old) {
                    (Some(s), _) if s.is_empty() => {}
                    (Some(s), _) => {
                        if r == c {
                            validate_random_comment(kind, s, validate)?;
                        } else {
                            check_comment_line(s)?;
                        }
                        text.push_str(&format!(" ;{s}"));
                    }
                    (None, Some(loc)) => {
                        if let Some(ci) = line_comment(&self.tokens, loc.param.value_idx) {
                            text.push_str(&format!(" {}", self.tokens[ci].text));
                        }
                    }
                    (None, None) => {}
                }
                lines.push(format!(" {text}"));
            }
        }
        if !is_positive_definite(&values, size) {
            bail!("The BLOCK({size}) values are not positive definite.");
        }
        let header = format!("$OMEGA BLOCK({size}){}", if fix { " FIX" } else { "" });

        let mut edited = self.clone();
        if existing.is_empty() {
            let text = std::iter::once(header)
                .chain(lines)
                .collect::<Vec<_>>()
                .join("\n");
            edited.append_record_after(&[NodeKind::Omega, NodeKind::Theta], &text)?;
        } else {
            // Remove the rows (whole records when every row goes), then put the
            // block where the first row was.
            let mut spans: Vec<(usize, usize)> = vec![];
            let mut records: Vec<usize> = existing.iter().map(|l| l.block.record_idx).collect();
            records.dedup();
            let mut trailing_rows = false;
            for &ri in &records {
                // One record can lower to several blocks (a FIX splits it).
                let record_params: Vec<&OmegaSigmaParam> = self
                    .omega_blocks
                    .iter()
                    .filter(|b| b.record_idx == ri)
                    .flat_map(|b| &b.parameters)
                    .collect();
                let involved: Vec<&OmegaSigmaParam> = existing
                    .iter()
                    .filter(|l| l.block.record_idx == ri)
                    .map(|l| l.param)
                    .collect();
                let record = self.record_node(ri)?;
                if involved.len() == record_params.len() {
                    let start = first_token_in(record).unwrap();
                    let end = line_end(&self.tokens, last_token_in(record, &self.tokens).unwrap());
                    spans.push((start, end));
                    continue;
                }
                for p in &involved {
                    let start = line_start(&self.tokens, p.value_idx);
                    let end = line_end(&self.tokens, p.value_idx);
                    let alone = record_params
                        .iter()
                        .filter(|o| line_start(&self.tokens, o.value_idx) == start)
                        .count()
                        == 1;
                    let header_line =
                        (start..=end).any(|i| self.tokens[i].token == Token::ControlRecord);
                    if !alone || header_line {
                        bail!(
                            "Each row going into the block must be on its own line; edit the \
                             $OMEGA record by hand."
                        );
                    }
                    spans.push((start, end));
                }
                // A header line with no rows of its own goes with the first row.
                let first_involved = involved[0].value_idx;
                if record_params[0].value_idx == first_involved {
                    let header = first_token_in(record).unwrap();
                    let header_end = line_end(&self.tokens, header);
                    let header_has_rows = record_params.iter().any(|o| o.value_idx <= header_end);
                    if !header_has_rows {
                        spans.retain(|&(s, _)| s != line_start(&self.tokens, first_involved));
                        spans.push((header, line_end(&self.tokens, first_involved)));
                        spans.sort();
                    }
                }
                let last_involved = involved.last().unwrap().value_idx;
                trailing_rows |= record_params.iter().any(|o| o.value_idx > last_involved);
            }
            let mut text = std::iter::once(header)
                .chain(lines)
                .collect::<Vec<_>>()
                .join("\n");
            text.push('\n');
            if trailing_rows {
                text.push_str("$OMEGA\n");
            }
            for &(start, end) in &spans {
                for i in start..=end {
                    edited.tokens[i].text.clear();
                }
            }
            edited.tokens[spans[0].0].text = text;
        }

        let edited = edited.reparse()?;
        let expected = n.max(last);
        if edited.eta_count() != expected {
            bail!("Making the block changed the number of ETAs.");
        }
        match locate_row(&edited.omega_blocks, kind, first) {
            Ok(loc) if loc.block.structure == (BlockStructure::Block { size }) => Ok(edited),
            _ => bail!("Making the block did not produce a BLOCK({size}) at ETA({first})."),
        }
    }
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
 KA = EXP(MU_1 + ETA(1))
 V  = EXP(MU_2 + ETA(2))
 CL = EXP(MU_3 + ETA(3))
 S2 = V

$ERROR
 IPRED = F
 W = SQRT(SIGMA(1,1))
 Y = IPRED + EPS(1)

$THETA
 -0.62508    ;KA [1/hr]  ;lognormal
 (0, 1.85635)    ;V  [L]     ;lognormal
 -1.90936    ;CL [L/hr]  ;lognormal

$OMEGA
 0.4194     ;IIV KA ;lognormal
 0.0177     ;IIV V  ;lognormal
 0.0796     ;IIV CL ;lognormal

$SIGMA
 1.148      ;Add [ng/mL] ;AddErr
";

    fn model() -> Model {
        Model::inner_parse(MODEL).unwrap()
    }

    #[test]
    fn add_theta_at_index_renumbers() {
        let (m, n) = model()
            .add_theta(
                &NewTheta {
                    init: 0.5,
                    comment: Some("new".into()),
                    ..Default::default()
                },
                Some(2),
                None,
            )
            .unwrap();
        assert_eq!(n, 2);
        let c = m.model_content();
        assert!(
            c.contains(" -0.62508    ;KA [1/hr]  ;lognormal\n 0.5 ;new\n (0, 1.85635)"),
            "{c}"
        );
        assert!(
            c.contains(" MU_1 = THETA(1)\n MU_2 = THETA(3)\n MU_3 = THETA(4)\n"),
            "{c}"
        );
    }

    #[test]
    fn add_omega_appends_and_sigma_appends() {
        let (m, n) = model()
            .add_random(
                RandomKind::Omega,
                &NewRow {
                    init: 0.03,
                    comment: Some("IIV ALAG1 ;lognormal".into()),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert_eq!(n, 4);
        assert!(
            m.model_content()
                .contains(";IIV CL ;lognormal\n 0.03 ;IIV ALAG1 ;lognormal\n")
        );
        let (m, n) = m
            .add_random(
                RandomKind::Sigma,
                &NewRow {
                    init: 0.25,
                    comment: Some("Prop ;Proportional".into()),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert_eq!(n, 2);
        assert!(
            m.model_content()
                .ends_with(";AddErr\n 0.25 ;Prop ;Proportional\n")
        );
        assert!(
            model()
                .add_random(
                    RandomKind::Omega,
                    &NewRow {
                        init: -1.0,
                        ..Default::default()
                    },
                    None
                )
                .is_err()
        );
    }

    #[test]
    fn add_omega_after_block_starts_new_record() {
        let src = MODEL.replace(
            "$OMEGA\n 0.4194     ;IIV KA ;lognormal\n 0.0177     ;IIV V  ;lognormal\n 0.0796     ;IIV CL ;lognormal\n",
            "$OMEGA 0.4194\n$OMEGA BLOCK(2)\n 0.0177\n 0.001 0.0796\n",
        );
        let m = Model::inner_parse(&src).unwrap();
        let (m, n) = m
            .add_random(
                RandomKind::Omega,
                &NewRow {
                    init: 0.1,
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert_eq!(n, 4);
        assert!(
            m.model_content().contains(" 0.001 0.0796\n$OMEGA 0.1\n"),
            "{}",
            m.model_content()
        );
    }

    #[test]
    fn update_theta_values_bounds_comment() {
        let m = model()
            .update_theta(
                2,
                &ThetaUpdate {
                    init: Some(2.0),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert!(m.model_content().contains(" (0, 2)    ;V  [L]"));
        let m = m
            .update_theta(
                2,
                &ThetaUpdate {
                    lower: Change::Remove,
                    fix: Some(true),
                    comment: Change::Set("V".into()),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert!(
            m.model_content().contains("\n 2 FIX    ;V\n"),
            "{}",
            m.model_content()
        );
        let m = m
            .update_theta(
                1,
                &ThetaUpdate {
                    comment: Change::Remove,
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert!(
            m.model_content().contains("$THETA\n -0.62508\n"),
            "{}",
            m.model_content()
        );
        assert!(
            model()
                .update_theta(
                    1,
                    &ThetaUpdate {
                        lower: Change::Set(0.0),
                        ..Default::default()
                    },
                    None
                )
                .is_err()
        );
    }

    #[test]
    fn update_omega_fix_and_value() {
        let m = model()
            .update_random(
                RandomKind::Omega,
                2,
                &RowUpdate {
                    init: Some(0.02),
                    fix: Some(true),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert!(
            m.model_content().contains(" 0.02 FIX     ;IIV V"),
            "{}",
            m.model_content()
        );
        let m = m
            .update_random(
                RandomKind::Omega,
                2,
                &RowUpdate {
                    fix: Some(false),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert!(
            m.model_content().contains(" 0.02     ;IIV V"),
            "{}",
            m.model_content()
        );
    }

    #[test]
    fn omega_block_from_existing_and_new_rows() {
        let m = model()
            .add_omega_block(
                3,
                2,
                &[None, Some(0.005), Some(0.03)],
                &[None, Some(String::new()), Some("IIV ALAG1".into())],
                false,
                None,
            )
            .unwrap();
        let c = m.model_content();
        assert!(
            c.contains(
                " 0.0177     ;IIV V  ;lognormal\n$OMEGA BLOCK(2)\n 0.0796 ;IIV CL ;lognormal\n 0.005\n 0.03 ;IIV ALAG1\n"
            ),
            "{c}"
        );
        assert_eq!(m.eta_count(), 4);
    }

    #[test]
    fn omega_block_after_fixed_row() {
        let m = model()
            .update_random(
                RandomKind::Omega,
                1,
                &RowUpdate {
                    fix: Some(true),
                    ..Default::default()
                },
                None,
            )
            .unwrap()
            .add_omega_block(
                2,
                2,
                &[None, Some(0.001), None],
                &[None, None, None],
                false,
                None,
            )
            .unwrap();
        let c = m.model_content();
        assert!(
            c.contains(" 0.4194 FIX     ;IIV KA ;lognormal\n$OMEGA BLOCK(2)\n 0.0177 ;IIV V  ;lognormal\n 0.001\n 0.0796 ;IIV CL ;lognormal\n"),
            "{c}"
        );
    }

    #[test]
    fn omega_block_middle_rows_split_record() {
        let m = model()
            .add_omega_block(
                1,
                2,
                &[None, Some(0.01), None],
                &[None, None, None],
                false,
                None,
            )
            .unwrap();
        let c = m.model_content();
        assert!(
            c.contains("$OMEGA BLOCK(2)\n 0.4194 ;IIV KA ;lognormal\n 0.01\n 0.0177 ;IIV V  ;lognormal\n$OMEGA\n 0.0796"),
            "{c}"
        );
        assert!(!c.contains("$OMEGA\n$OMEGA BLOCK"), "{c}");
        assert!(
            model()
                .add_omega_block(
                    1,
                    2,
                    &[None, Some(1.0), None],
                    &[None, None, None],
                    false,
                    None
                )
                .unwrap_err()
                .to_string()
                .contains("positive definite")
        );
    }
}
