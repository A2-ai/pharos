//! Edits to whole records: `$MODEL`, `$TABLE`, `$EST`, `$SUBROUTINES`,
//! `$COV`, `$DATA`, and renaming a variable. See `edit.rs` for the approach.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use crate::cst::{CstChild, CstNode, NodeKind};
use crate::lexer::{SpannedToken, Token};
use crate::nmtran::NmtranToken;

use super::Model;
use super::edit::{
    RESERVED_NAMES, collect_assignments, defined_names, first_token_in, input_names, is_nm_trivia,
    last_code_token_in, last_token_in, line_end, line_last_token, line_start, node_tokens,
};

/// What to do with one record option.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionEdit {
    /// Set `NAME=value`, replacing an existing value.
    Value(String),
    /// Add the flag `NAME`.
    Flag,
    /// Remove the option.
    Remove,
}

/// `$TABLE` options; every other bare word in a `$TABLE` is a column.
const TABLE_OPTIONS: &[&str] = &[
    "PRINT",
    "NOPRINT",
    "FILE",
    "NOHEADER",
    "ONEHEADER",
    "ONEHEADERALL",
    "NOTITLE",
    "NOLABEL",
    "FIRSTONLY",
    "FIRSTRECORDONLY",
    "FIRSTLASTONLY",
    "LASTONLY",
    "NOFORWARD",
    "FORWARD",
    "APPEND",
    "NOAPPEND",
    "FORMAT",
    "LFORMAT",
    "RFORMAT",
    "IDFORMAT",
    "NOSUB",
    "ESAMPLE",
    "WRESCHOL",
    "SEED",
    "CLOCKSEED",
    "RANMETHOD",
    "VARCALC",
    "FIXEDETAS",
    "UNCONDITIONAL",
    "CONDITIONAL",
    "OMITTED",
    "EXCLUDE_BY",
    "EXCLUDE_TABLE",
    "PARAFILE",
    "BY",
    "NPDTYPE",
    "INTERPTYPE",
    "PRED_IGNORE_DATA",
];

/// Items NONMEM can write to a table without a definition in the model.
const TABLE_ITEMS: &[&str] = &[
    "PRED", "RES", "WRES", "CWRES", "CPRED", "CRES", "CPREDI", "CRESI", "CWRESI", "CIPRED",
    "CIPREDI", "CIRES", "CIRESI", "CIWRES", "CIWRESI", "EPRED", "ERES", "EWRES", "ECWRES", "NPDE",
    "NPD", "IPRD", "IPRDI", "IRES", "IRESI", "IWRES", "IWRESI", "OBJI", "PREDI", "RESI", "WRESI",
    "EIPRED", "EIRES", "EIWRES", "NPRED", "NRES", "NWRES",
];

fn word_of(node: &CstNode, tokens: &[SpannedToken]) -> Option<String> {
    first_token_in(node).map(|i| tokens[i].text.to_uppercase())
}

fn is_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Model {
    fn record_nodes(&self, kind: NodeKind) -> Vec<(usize, &CstNode)> {
        self.cst
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                CstChild::Node(n) if n.kind == kind => Some((i, n)),
                _ => None,
            })
            .collect()
    }

    /// Blank a node's tokens and one separating space: the whitespace after
    /// it, or before it when the line ends right after the node.
    fn blank_node(&mut self, node: &CstNode) {
        let mut toks = vec![];
        node_tokens(node, &mut toks);
        let (Some(&first), Some(&last)) = (toks.first(), toks.last()) else {
            return;
        };
        let ends_with_space = self.tokens[last].token == Token::Whitespace;
        for &i in &toks {
            self.tokens[i].text.clear();
        }
        if ends_with_space {
            return;
        }
        match self.tokens.get(last + 1).map(|t| &t.token) {
            Some(Token::Whitespace) => self.tokens[last + 1].text.clear(),
            None | Some(Token::Newline | Token::ControlRecord)
                if first > 0 && self.tokens[first - 1].token == Token::Whitespace =>
            {
                self.tokens[first - 1].text.clear();
            }
            _ => {}
        }
    }

    /// Set, add or remove options on the record at `record_idx`.
    fn edit_record_options(
        &self,
        record_idx: usize,
        record_name: &str,
        edits: &[(String, OptionEdit)],
    ) -> Result<Model> {
        let CstChild::Node(record) = &self.cst.children[record_idx] else {
            bail!("Could not locate {record_name}.");
        };
        let options: Vec<(String, &CstNode)> = record
            .children
            .iter()
            .filter_map(|c| match c {
                CstChild::Node(n) if matches!(n.kind, NodeKind::KeyValue | NodeKind::Flag) => {
                    word_of(n, &self.tokens).map(|w| (w, n))
                }
                _ => None,
            })
            .collect();
        let mut edited = self.clone();
        let mut appended = String::new();
        for (name, edit) in edits {
            let name = name.to_uppercase();
            if !is_name(&name) {
                bail!("`{name}` is not a valid option name.");
            }
            let found: Vec<&CstNode> = options
                .iter()
                .filter(|(w, _)| *w == name)
                .map(|(_, n)| *n)
                .collect();
            if found.is_empty()
                && edit != &OptionEdit::Remove
                && let Some((other, _)) = options.iter().find(|(w, _)| {
                    w.len() >= 3 && (name.starts_with(w.as_str()) || w.starts_with(&name))
                })
            {
                bail!(
                    "{record_name} already has `{other}`, which NONMEM reads as the same \
                     option as `{name}`. Use `{}` instead.",
                    other.to_lowercase()
                );
            }
            let node = match found.as_slice() {
                [] => None,
                [one] => Some(*one),
                _ => bail!("{record_name} has `{name}` more than once; edit it by hand."),
            };
            if let OptionEdit::Value(v) = edit
                && (v.is_empty() || v.chars().any(char::is_whitespace))
            {
                bail!("The value for `{name}` can't be empty or contain spaces.");
            }
            match (edit, node) {
                (OptionEdit::Value(v), Some(n)) if n.kind == NodeKind::KeyValue => {
                    let mut toks = vec![];
                    node_tokens(n, &mut toks);
                    let eq = toks
                        .iter()
                        .position(|&i| self.tokens[i].token == Token::Equals)
                        .ok_or_else(|| anyhow::anyhow!("Could not read `{name}`."))?;
                    let value: Vec<usize> = toks[eq + 1..]
                        .iter()
                        .copied()
                        .filter(|&i| {
                            !matches!(self.tokens[i].token, Token::Whitespace | Token::Newline)
                        })
                        .collect();
                    for (k, &i) in value.iter().enumerate() {
                        edited.tokens[i].text = if k == 0 { v.clone() } else { String::new() };
                    }
                }
                (OptionEdit::Value(_), Some(_)) => bail!(
                    "`{name}` is a flag in {record_name}; set it with TRUE or remove it with NULL."
                ),
                (OptionEdit::Value(v), None) => appended.push_str(&format!(" {name}={v}")),
                (OptionEdit::Flag, Some(n)) if n.kind == NodeKind::Flag => {}
                (OptionEdit::Flag, Some(_)) => {
                    bail!("`{name}` takes a value in {record_name}; give it one.")
                }
                (OptionEdit::Flag, None) => appended.push_str(&format!(" {name}")),
                (OptionEdit::Remove, Some(n)) => edited.blank_node(n),
                (OptionEdit::Remove, None) => bail!("{record_name} has no `{name}` to remove."),
            }
        }
        if !appended.is_empty() {
            let anchor = last_code_token_in(record, &self.tokens)
                .ok_or_else(|| anyhow::anyhow!("Could not locate the end of {record_name}."))?;
            edited.tokens[anchor].text.push_str(&appended);
        }
        edited.reparse()
    }

    /// Change options on the `index`-th `$EST` record (0-based).
    pub fn update_est(&self, index: usize, edits: &[(String, OptionEdit)]) -> Result<Model> {
        let Some(est) = self.estimations.get(index) else {
            bail!(
                "There is no $EST {}: the model has {}.",
                index + 1,
                self.estimations.len()
            );
        };
        self.edit_record_options(est.record_idx, "$EST", edits)
    }

    /// Change `$SUBROUTINES`: `advan`/`trans` replace the `ADVANn`/`TRANSn`
    /// flags, `tol` the `TOL=` option. `None` keeps; for `trans` and `tol`,
    /// `Some(None)` removes.
    pub fn update_subroutines(
        &self,
        advan: Option<u32>,
        trans: Option<Option<u32>>,
        tol: Option<Option<u32>>,
    ) -> Result<Model> {
        let Some(subs) = &self.subroutines else {
            bail!("The model has no $SUBROUTINES record.");
        };
        let CstChild::Node(record) = &self.cst.children[subs.record_idx] else {
            bail!("Could not locate $SUBROUTINES.");
        };
        let flags = |prefix: &str| -> Vec<&CstNode> {
            record
                .children
                .iter()
                .filter_map(|c| match c {
                    CstChild::Node(n) if n.kind == NodeKind::Flag => Some(n),
                    _ => None,
                })
                .filter(|n| {
                    word_of(n, &self.tokens).is_some_and(|w| {
                        w.strip_prefix(prefix)
                            .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
                    })
                })
                .collect()
        };
        let mut edited = self.clone();
        let mut appended = String::new();
        for (prefix, change) in [("ADVAN", advan.map(Some)), ("TRANS", trans)] {
            let Some(change) = change else { continue };
            let found = flags(prefix);
            if found.len() > 1 {
                bail!("$SUBROUTINES has more than one {prefix}; edit it by hand.");
            }
            match (change, found.first()) {
                (Some(n), Some(node)) => {
                    edited.tokens[first_token_in(node).unwrap()].text = format!("{prefix}{n}")
                }
                (Some(n), None) => appended.push_str(&format!(" {prefix}{n}")),
                (None, Some(node)) => edited.blank_node(node),
                (None, None) => bail!("$SUBROUTINES has no {prefix} to remove."),
            }
        }
        if !appended.is_empty() {
            let anchor = last_code_token_in(record, &self.tokens)
                .ok_or_else(|| anyhow::anyhow!("Could not locate the end of $SUBROUTINES."))?;
            edited.tokens[anchor].text.push_str(&appended);
        }
        let mut edited = edited.reparse()?;
        if let Some(tol) = tol {
            let edit = match tol {
                Some(n) => OptionEdit::Value(n.to_string()),
                None => OptionEdit::Remove,
            };
            let idx = edited.subroutines.as_ref().unwrap().record_idx;
            edited = edited.edit_record_options(idx, "$SUBROUTINES", &[("TOL".into(), edit)])?;
        }
        Ok(edited)
    }

    /// Remove the `$COV` record.
    pub fn remove_cov(&self) -> Result<Model> {
        let records = self.record_nodes(NodeKind::Covariance);
        let (idx, _) = match records.as_slice() {
            [one] => one,
            [] => bail!("The model has no $COV record."),
            _ => bail!("The model has more than one $COV record; edit it by hand."),
        };
        let edited = self.remove_record(*idx)?;
        if edited.covariance.is_some() {
            bail!("Removing $COV did not remove it.");
        }
        Ok(edited)
    }

    /// Set the `$DATA` path, exactly as it should appear in the file.
    pub fn update_data(&self, path: &str) -> Result<Model> {
        let quoted = self
            .data
            .path_idx
            .is_some_and(|i| self.tokens[i].token == Token::QuotedString);
        if path.is_empty() {
            bail!("`path` can't be empty.");
        }
        if !quoted && path.chars().any(char::is_whitespace) {
            bail!("`path` has a space; quote it in $DATA by hand first.");
        }
        let mut edited = self.clone();
        edited.update_data_path(path);
        let edited = edited.reparse()?;
        if edited.data.path != path {
            bail!("Setting the $DATA path gave `{}`.", edited.data.path);
        }
        Ok(edited)
    }

    /// Append lines to `$MODEL`.
    pub fn update_model_record(&self, lines: &[String]) -> Result<Model> {
        let Some(idx) = self.model_record_idx() else {
            bail!("The model has no $MODEL record.");
        };
        let CstChild::Node(record) = &self.cst.children[idx] else {
            bail!("Could not locate $MODEL.");
        };
        let last = last_token_in(record, &self.tokens)
            .ok_or_else(|| anyhow::anyhow!("Could not locate the end of $MODEL."))?;
        let anchor = line_last_token(&self.tokens, last);
        let mut text = String::new();
        for line in lines {
            if line.contains('\n') {
                bail!("Each element of `append` must be a single line.");
            }
            text.push_str(&format!("\n {}", line.trim()));
        }
        let mut edited = self.clone();
        edited.tokens[anchor].text.push_str(&text);
        let edited = edited.reparse()?;
        self.check_new_references(&edited)?;
        Ok(edited)
    }

    /// Pick a `$TABLE` (0-based) by `FILE=` name or by 1-based index. With
    /// one table, neither is needed.
    pub fn table_index(&self, file: Option<&str>, index: Option<usize>) -> Result<usize> {
        let listing = || {
            self.tables
                .iter()
                .enumerate()
                .map(|(i, t)| format!("{}: {}", i + 1, t.file.as_deref().unwrap_or("(no FILE)")))
                .collect::<Vec<_>>()
                .join(", ")
        };
        match (file, index) {
            (Some(_), Some(_)) => bail!("Give `file` or `index`, not both."),
            (Some(f), None) => {
                let hits: Vec<usize> = self
                    .tables
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.file.as_deref() == Some(f))
                    .map(|(i, _)| i)
                    .collect();
                match hits.as_slice() {
                    [one] => Ok(*one),
                    [] => bail!("No $TABLE has FILE={f}. Tables: {}.", listing()),
                    _ => bail!("More than one $TABLE has FILE={f}; use `index`."),
                }
            }
            (None, Some(i)) if i >= 1 && i <= self.tables.len() => Ok(i - 1),
            (None, Some(i)) => bail!("There is no $TABLE {i}. Tables: {}.", listing()),
            (None, None) if self.tables.len() == 1 => Ok(0),
            (None, None) if self.tables.is_empty() => bail!("The model has no $TABLE record."),
            (None, None) => bail!(
                "The model has {} tables; give `file` or `index`. Tables: {}.",
                self.tables.len(),
                listing()
            ),
        }
    }

    /// Columns of a `$TABLE` (bare words that aren't options), upper case.
    fn table_columns<'a>(&self, record: &'a CstNode) -> Vec<(String, &'a CstNode)> {
        record
            .children
            .iter()
            .filter_map(|c| match c {
                CstChild::Node(n) if n.kind == NodeKind::Flag => {
                    word_of(n, &self.tokens).map(|w| (w, n))
                }
                _ => None,
            })
            .filter(|(w, _)| !TABLE_OPTIONS.contains(&w.as_str()))
            .collect()
    }

    /// Add columns to the end of a `$TABLE`'s column list (0-based `index`).
    pub fn update_table(&self, index: usize, columns: &[String]) -> Result<Model> {
        let Some(table) = self.tables.get(index) else {
            bail!("There is no $TABLE {}.", index + 1);
        };
        let CstChild::Node(record) = &self.cst.children[table.record_idx] else {
            bail!("Could not locate the $TABLE.");
        };
        let existing = self.table_columns(record);
        let defined = defined_names(self);
        let mut seen: BTreeSet<String> = existing.iter().map(|(w, _)| w.clone()).collect();
        let mut text = String::new();
        for col in columns {
            let col = col.trim();
            let up = col.to_uppercase();
            let etas = up.starts_with("ETAS(") && up.ends_with(')');
            if !etas && !is_name(&up) {
                bail!("`{col}` is not a column name.");
            }
            if TABLE_OPTIONS.contains(&up.as_str()) {
                bail!("`{col}` is a $TABLE option, not a column.");
            }
            let numbered = |p: &str| {
                up.strip_prefix(p)
                    .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
            };
            let known = etas
                || numbered("ETA")
                || numbered("ET")
                || defined.contains(&up)
                || TABLE_ITEMS.contains(&up.as_str());
            if !known {
                bail!(
                    "`{col}` is not an $INPUT column, assigned in the model's code, or a NONMEM \
                     table item."
                );
            }
            if !seen.insert(up.clone()) {
                bail!("The $TABLE already has `{up}`.");
            }
            text.push_str(&format!(" {col}"));
        }
        let anchor = match existing.last() {
            Some((_, n)) => last_token_in(n, &self.tokens),
            None => first_token_in(record),
        }
        .ok_or_else(|| anyhow::anyhow!("Could not locate the $TABLE columns."))?;
        let mut edited = self.clone();
        edited.tokens[anchor].text.push_str(&text);
        edited.reparse()
    }

    /// Rename a variable assigned in code, across every code record and
    /// `$TABLE` columns. Comments and `$INPUT` are untouched.
    pub fn rename_variable(&self, from: &str, to: &str) -> Result<Model> {
        let f = from.to_uppercase();
        let t = to.to_uppercase();
        if !is_name(&t) {
            bail!("`{to}` is not a valid NONMEM name.");
        }
        if input_names(self).contains(&f) {
            bail!("`{from}` is an $INPUT column; rename the column in the dataset instead.");
        }
        let mut assigned = BTreeSet::new();
        let mut used = BTreeSet::new();
        let mut indexed = false;
        for cb in self.code_blocks() {
            let mut assignments = vec![];
            collect_assignments(&cb.children, &cb.tokens, &mut assignments);
            for a in assignments {
                assigned.insert(a.lhs.split('(').next().unwrap_or(&a.lhs).to_string());
            }
            for (k, tok) in cb.tokens.iter().enumerate() {
                if tok.token != NmtranToken::Ident {
                    continue;
                }
                let up = tok.text.to_uppercase();
                if up == f
                    && cb.tokens[k + 1..]
                        .iter()
                        .find(|n| !is_nm_trivia(n))
                        .is_some_and(|n| n.token == NmtranToken::LeftParen)
                {
                    indexed = true;
                }
                used.insert(up);
            }
        }
        if !assigned.contains(&f) {
            bail!("`{from}` is not assigned in the model's code.");
        }
        if indexed || RESERVED_NAMES.contains(&f.as_str()) {
            bail!("`{from}` is a NONMEM name or indexed; it can't be renamed.");
        }
        if used.contains(&t) || defined_names(self).contains(&t) {
            bail!("`{to}` already exists in the model.");
        }

        let mut edited = self.clone();
        for b in [&self.pk, &self.error, &self.des, &self.pred]
            .into_iter()
            .flatten()
        {
            if let Some(cb) = edited.code_block_at_mut(b.record_idx) {
                for tok in cb.tokens.iter_mut() {
                    if tok.token == NmtranToken::Ident && tok.text.eq_ignore_ascii_case(&f) {
                        tok.text = to.to_string();
                    }
                }
            }
        }
        for table in &self.tables {
            let CstChild::Node(record) = &self.cst.children[table.record_idx] else {
                continue;
            };
            for (w, n) in self.table_columns(record) {
                if w == f {
                    edited.tokens[first_token_in(n).unwrap()].text = to.to_string();
                }
            }
        }
        edited.reparse()
    }

    /// `$MODEL` compartments (1-based) that aren't `INITIALOFF` and have no
    /// `DADT(n)` assigned in `$DES`. Empty without `$DES` or `$MODEL`.
    pub fn missing_dadt(&self) -> Vec<usize> {
        let (Some(des), Some(comps)) = (&self.des, self.compartments()) else {
            return vec![];
        };
        let Some(cb) = self.code_block_at(des.record_idx) else {
            return vec![];
        };
        let mut assignments = vec![];
        collect_assignments(&cb.children, &cb.tokens, &mut assignments);
        let assigned: BTreeSet<String> = assignments.into_iter().map(|a| a.lhs).collect();
        comps
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.attributes.iter().any(|a| a == "INITIALOFF"))
            .map(|(i, _)| i + 1)
            .filter(|n| !assigned.contains(&format!("DADT({n})")))
            .collect()
    }
}

/// `$DATA` filter list kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Ignore,
    Accept,
}

impl FilterKind {
    fn name(self) -> &'static str {
        match self {
            FilterKind::Ignore => "IGNORE",
            FilterKind::Accept => "ACCEPT",
        }
    }
}

/// Upper case with whitespace removed, for exact filter matching.
fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

impl Model {
    fn data_record(&self) -> Result<&CstNode> {
        match self.record_nodes(NodeKind::Data).as_slice() {
            [(_, n)] => Ok(*n),
            [] => bail!("The model has no $DATA record."),
            _ => bail!("The model has more than one $DATA record."),
        }
    }

    fn filters(&self, kind: FilterKind) -> &[crate::ast::DataFilter] {
        match kind {
            FilterKind::Ignore => &self.data.ignore,
            FilterKind::Accept => &self.data.accept,
        }
    }

    /// `(KIND, filter node, parens node)` for every list-form filter in `$DATA`.
    fn data_filter_nodes(&self) -> Result<Vec<(FilterKind, &CstNode, &CstNode)>> {
        let record = self.data_record()?;
        let mut out = vec![];
        for child in &record.children {
            let CstChild::Node(kv) = child else { continue };
            if kv.kind != NodeKind::KeyValue {
                continue;
            }
            let kind = match word_of(kv, &self.tokens).as_deref() {
                Some("IGNORE") => FilterKind::Ignore,
                Some("ACCEPT") => FilterKind::Accept,
                _ => continue,
            };
            for c in &kv.children {
                let CstChild::Node(parens) = c else { continue };
                if parens.kind != NodeKind::Parens {
                    continue;
                }
                for f in &parens.children {
                    if let CstChild::Node(filter) = f
                        && filter.kind == NodeKind::Filter
                    {
                        out.push((kind, filter, parens));
                    }
                }
            }
        }
        Ok(out)
    }

    fn filter_text(&self, filter: &CstNode) -> String {
        let mut toks = vec![];
        node_tokens(filter, &mut toks);
        squash(
            &toks
                .iter()
                .map(|&i| self.tokens[i].text.as_str())
                .collect::<String>(),
        )
    }

    /// Add `condition` (e.g. `DVID.EQ.2`) as its own `IGNORE=(...)` or
    /// `ACCEPT=(...)` option at the end of `$DATA`.
    pub fn add_data_filter(&self, kind: FilterKind, condition: &str) -> Result<Model> {
        let cond = condition.trim();
        if cond.is_empty() || cond.contains(['\n', '(', ')', ',', ';']) {
            bail!("`{condition}` must be a single condition like `DVID.EQ.2`.");
        }
        let other = match kind {
            FilterKind::Ignore => FilterKind::Accept,
            FilterKind::Accept => FilterKind::Ignore,
        };
        let other_lists = self
            .data_filter_nodes()?
            .iter()
            .any(|(k, _, _)| *k == other);
        if other_lists {
            bail!(
                "$DATA has an {} list, and NONMEM doesn't allow ACCEPT and IGNORE lists together.",
                other.name()
            );
        }
        if self
            .data_filter_nodes()?
            .iter()
            .any(|(k, f, _)| *k == kind && self.filter_text(f) == squash(cond))
        {
            bail!("$DATA already has {}=({cond}).", kind.name());
        }

        let record = self.data_record()?;
        let anchor = last_code_token_in(record, &self.tokens)
            .ok_or_else(|| anyhow::anyhow!("Could not locate the end of $DATA."))?;
        let mut edited = self.clone();
        edited.tokens[anchor]
            .text
            .push_str(&format!(" {}=({cond})", kind.name()));
        let edited = edited.reparse()?;

        let before = self.filters(kind).len();
        let added = edited.filters(kind);
        let Some(crate::ast::DataFilter::ValueFilter(f)) =
            added.last().filter(|_| added.len() == before + 1)
        else {
            bail!("`{cond}` doesn't parse as a $DATA condition like `DVID.EQ.2`.");
        };
        if !input_names(self).contains(&f.field.to_uppercase())
            && !self.input_columns.iter().any(|c| {
                matches!(&c.kind, crate::ast::InputColumnKind::Dropped(n) if n.eq_ignore_ascii_case(&f.field))
            })
        {
            bail!("`{}` is not an $INPUT column.", f.field);
        }
        Ok(edited)
    }

    /// Remove the `IGNORE`/`ACCEPT` condition that matches `condition`
    /// exactly (case and spaces aside). An option left empty is removed.
    pub fn remove_data_filter(&self, kind: FilterKind, condition: &str) -> Result<Model> {
        let target = squash(condition);
        let nodes = self.data_filter_nodes()?;
        let hits: Vec<&(FilterKind, &CstNode, &CstNode)> = nodes
            .iter()
            .filter(|(k, f, _)| *k == kind && self.filter_text(f) == target)
            .collect();
        let (_, filter, parens) = match hits.as_slice() {
            [one] => **one,
            [] => {
                let have: Vec<String> = nodes
                    .iter()
                    .filter(|(k, _, _)| *k == kind)
                    .map(|(_, f, _)| self.filter_text(f))
                    .collect();
                bail!(
                    "$DATA has no {}=({condition}). It has: {}.",
                    kind.name(),
                    if have.is_empty() {
                        "none".to_string()
                    } else {
                        have.join(", ")
                    }
                )
            }
            _ => bail!(
                "$DATA has {}=({condition}) more than once; edit it by hand.",
                kind.name()
            ),
        };

        let mut edited = self.clone();
        let siblings = parens
            .children
            .iter()
            .filter(|c| matches!(c, CstChild::Node(n) if n.kind == NodeKind::Filter))
            .count();
        if siblings == 1 {
            // The whole IGNORE=(...) option goes.
            let record = self.data_record()?;
            let kv = record
                .children
                .iter()
                .find_map(|c| match c {
                    CstChild::Node(kv)
                        if kv
                            .children
                            .iter()
                            .any(|k| matches!(k, CstChild::Node(p) if std::ptr::eq(p, parens))) =>
                    {
                        Some(kv)
                    }
                    _ => None,
                })
                .ok_or_else(|| anyhow::anyhow!("Could not locate the $DATA option."))?;
            edited.blank_node(kv);
        } else {
            // Drop the condition and the comma that separates it.
            let pos = parens
                .children
                .iter()
                .position(|c| matches!(c, CstChild::Node(n) if std::ptr::eq(n, filter)))
                .unwrap();
            let is_filter =
                |c: &CstChild| matches!(c, CstChild::Node(n) if n.kind == NodeKind::Filter);
            let next = parens.children[pos + 1..]
                .iter()
                .position(is_filter)
                .map(|k| pos + 1 + k);
            let range = match next {
                Some(n) => pos..n,
                None => {
                    let prev = parens.children[..pos].iter().rposition(is_filter).unwrap();
                    prev + 1..pos + 1
                }
            };
            for c in &parens.children[range] {
                match c {
                    CstChild::Token(i) => edited.tokens[*i].text.clear(),
                    CstChild::Node(n) => {
                        let mut toks = vec![];
                        node_tokens(n, &mut toks);
                        toks.into_iter().for_each(|i| edited.tokens[i].text.clear());
                    }
                    CstChild::CodeBlock(_) => {}
                }
            }
        }
        let edited = edited.reparse()?;
        if edited.filters(kind).len() + 1 != self.filters(kind).len() {
            bail!("Removing the condition did not remove exactly one filter.");
        }
        Ok(edited)
    }

    /// Add a `$EST` record after the last one.
    pub fn add_est(&self, options: &[(String, OptionEdit)]) -> Result<Model> {
        if options.is_empty() {
            bail!("Give at least one option, e.g. `method = \"IMP\"`.");
        }
        let mut words: Vec<String> = vec![];
        let mut names: Vec<String> = vec![];
        for (name, edit) in options {
            let name = name.to_uppercase();
            if !is_name(&name) {
                bail!("`{name}` is not a valid option name.");
            }
            if let Some(other) = names.iter().find(|w| {
                **w == name
                    || (w.len() >= 3
                        && name.len() >= 3
                        && (name.starts_with(w.as_str()) || w.starts_with(&name)))
            }) {
                bail!("`{name}` and `{other}` are the same $EST option to NONMEM; give it once.");
            }
            names.push(name.clone());
            match edit {
                OptionEdit::Value(v) if v.is_empty() || v.chars().any(char::is_whitespace) => {
                    bail!("The value for `{name}` can't be empty or contain spaces.")
                }
                OptionEdit::Value(v) => words.push(format!("{name}={v}")),
                OptionEdit::Flag => words.push(name),
                OptionEdit::Remove => bail!("`{name}`: a new $EST has nothing to remove."),
            }
        }
        // Wrap long records onto indented continuation lines.
        let mut lines = vec![String::from("$EST")];
        for w in words {
            let last = lines.last_mut().unwrap();
            if last.len() + 1 + w.len() > 80 && last.trim() != "$EST" {
                lines.push(format!("     {w}"));
            } else {
                last.push(' ');
                last.push_str(&w);
            }
        }
        let text = lines.join("\n");

        let mut edited = self.clone();
        let anchor = match self.estimations.last() {
            Some(est) => {
                let CstChild::Node(record) = &self.cst.children[est.record_idx] else {
                    bail!("Could not locate $EST.");
                };
                last_token_in(record, &self.tokens)
            }
            None => [NodeKind::Sigma, NodeKind::Omega, NodeKind::Theta]
                .iter()
                .find_map(|k| {
                    self.record_nodes(*k)
                        .last()
                        .and_then(|(_, n)| last_token_in(n, &self.tokens))
                }),
        }
        .ok_or_else(|| anyhow::anyhow!("The model has no record to place $EST after."))?;
        let anchor = line_last_token(&self.tokens, anchor);
        edited.tokens[anchor].text.push_str(&format!("\n{text}"));
        let edited = edited.reparse()?;
        if edited.estimations.len() != self.estimations.len() + 1 {
            bail!("Adding $EST did not produce exactly one new $EST record.");
        }
        Ok(edited)
    }

    /// Remove the `index`-th `$EST` record (0-based).
    pub fn remove_est(&self, index: usize) -> Result<Model> {
        let n = self.estimations.len();
        let Some(est) = self.estimations.get(index) else {
            bail!("There is no $EST {}: the model has {n}.", index + 1);
        };
        if n == 1 {
            bail!("This is the model's only $EST record.");
        }
        let edited = self.remove_record(est.record_idx)?;
        if edited.estimations.len() != n - 1 {
            bail!("Removing $EST did not remove exactly one record.");
        }
        Ok(edited)
    }

    /// Blank a whole record, its line end, and one blank line when it sat
    /// between two.
    fn remove_record(&self, record_idx: usize) -> Result<Model> {
        let CstChild::Node(record) = &self.cst.children[record_idx] else {
            bail!("Could not locate the record.");
        };
        let first = first_token_in(record).unwrap();
        let end = line_end(
            &self.tokens,
            last_token_in(record, &self.tokens).unwrap_or(first),
        );
        // Comment lines directly above the record describe it; remove them too.
        let mut start = first;
        while start >= 2 && self.tokens[start - 1].token == Token::Newline {
            let above = line_start(&self.tokens, start - 2);
            let line = &self.tokens[above..start - 1];
            let comment_only = line.iter().any(|t| t.token == Token::Comment)
                && line
                    .iter()
                    .all(|t| matches!(t.token, Token::Comment | Token::Whitespace));
            if !comment_only {
                break;
            }
            start = above;
        }
        let mut edited = self.clone();
        for i in start..=end {
            edited.tokens[i].text.clear();
        }
        let blank_before = start >= 2
            && self.tokens[start - 1].token == Token::Newline
            && self.tokens[start - 2].token == Token::Newline;
        if blank_before
            && let Some(next) = self.tokens.get(end + 1)
            && next.token == Token::Newline
        {
            edited.tokens[end + 1].text.clear();
        }
        edited.reparse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edit::CodeRecord;

    const MODEL: &str = "\
$PROBLEM two
$INPUT ID TIME DV AMT WT
$DATA ../data.csv IGNORE=@
$SUBROUTINE ADVAN13 TRANS1 TOL=6
$MODEL COMP=(DEPOT, DEFDOSE)
 COMP=(CENTRAL) ; c
$PK
 KA = THETA(1)
 V = THETA(2)
 K = THETA(3)
$DES
 DADT(1) = -KA*A(1)
 DADT(2) = KA*A(1) - K*A(2)
$ERROR
 IPRED = A(2)/V
 Y = IPRED + EPS(1)
$THETA 1 2 3
$OMEGA 0.1
$SIGMA 0.1
$EST METHOD=1 INTER MAXEVAL=9999
 PRINT=5 ; c
$COV
$TABLE ID TIME KA NOPRINT
 FILE=sdtab1
$TABLE ID V NOPRINT FILE=patab1
";

    fn model() -> Model {
        Model::inner_parse(MODEL).unwrap()
    }

    #[test]
    fn est_options() {
        let m = model()
            .update_est(
                0,
                &[
                    ("maxeval".into(), OptionEdit::Value("0".into())),
                    ("inter".into(), OptionEdit::Remove),
                    ("posthoc".into(), OptionEdit::Flag),
                    ("msfo".into(), OptionEdit::Value("run.msf".into())),
                ],
            )
            .unwrap();
        let c = m.model_content();
        assert!(
            c.contains("$EST METHOD=1 MAXEVAL=0\n PRINT=5 POSTHOC MSFO=run.msf ; c\n"),
            "{c}"
        );
        let m = model()
            .update_est(0, &[("method".into(), OptionEdit::Remove)])
            .unwrap();
        assert!(m.model_content().contains("$EST INTER MAXEVAL=9999\n"));
        let err = model()
            .update_est(0, &[("max".into(), OptionEdit::Value("0".into()))])
            .unwrap_err()
            .to_string();
        assert!(err.contains("MAXEVAL"), "{err}");
    }

    #[test]
    fn subroutines_and_cov_and_data() {
        let m = model()
            .update_subroutines(Some(6), Some(None), Some(Some(9)))
            .unwrap()
            .remove_cov()
            .unwrap()
            .update_data("../other.csv")
            .unwrap();
        let c = m.model_content();
        assert!(c.contains("$SUBROUTINE ADVAN6 TOL=9\n"), "{c}");
        assert!(c.contains("; c\n$TABLE ID TIME"), "{c}");
        assert!(c.contains("$DATA ../other.csv IGNORE=@"), "{c}");
    }

    #[test]
    fn table_columns_and_selection() {
        let m = model();
        assert!(m.table_index(None, None).is_err());
        let i = m.table_index(Some("patab1"), None).unwrap();
        let m = m.update_table(i, &["K".into()]).unwrap();
        assert!(
            m.model_content()
                .contains("$TABLE ID V K NOPRINT FILE=patab1")
        );
        assert!(m.update_table(0, &["NOPE".into()]).is_err());
        assert!(m.update_table(0, &["KA".into()]).is_err());
        assert!(m.update_table(0, &["CWRES".into(), "ETA1".into()]).is_ok());
    }

    #[test]
    fn model_record_and_compartment_check() {
        let m = model()
            .update_model_record(&["COMP=(PERIPH)".into()])
            .unwrap();
        assert_eq!(m.compartments().unwrap().len(), 3);
        assert!(
            m.model_content()
                .contains(" COMP=(CENTRAL) ; c\n COMP=(PERIPH)\n$PK")
        );
        assert_eq!(m.missing_dadt(), vec![3]);
        let m = m
            .add_statement(CodeRecord::Des, "DADT(3) = K*A(2)")
            .unwrap();
        assert!(m.missing_dadt().is_empty());
        let err = m
            .add_statement(CodeRecord::Des, "DADT(4) = 0")
            .unwrap_err()
            .to_string();
        assert!(err.contains("DADT(4)"), "{err}");
    }

    #[test]
    fn rename() {
        let m = model().rename_variable("V", "V2").unwrap();
        let c = m.model_content();
        assert!(
            c.contains(" V2 = THETA(2)\n")
                && c.contains("A(2)/V2")
                && c.contains("$TABLE ID V2 NOPRINT"),
            "{c}"
        );
        assert!(model().rename_variable("WT", "W2").is_err());
        assert!(model().rename_variable("V", "KA").is_err());
        assert!(model().rename_variable("NOPE", "X").is_err());
    }

    #[test]
    fn data_filters_add_remove() {
        let src = MODEL.replace(
            "$DATA ../data.csv IGNORE=@",
            "$DATA ../data.csv IGNORE=@ IGNORE=(ID.EQ.3, TIME.GT.24)",
        );
        let m = Model::inner_parse(&src).unwrap();
        let m = m.add_data_filter(FilterKind::Ignore, "AMT.GT.100").unwrap();
        assert!(
            m.model_content()
                .contains("IGNORE=(ID.EQ.3, TIME.GT.24) IGNORE=(AMT.GT.100)\n")
        );
        let m = m.remove_data_filter(FilterKind::Ignore, "id.eq.3").unwrap();
        assert!(
            m.model_content()
                .contains("IGNORE=@ IGNORE=(TIME.GT.24) IGNORE=(AMT.GT.100)\n"),
            "{}",
            m.model_content()
        );
        let m = m
            .remove_data_filter(FilterKind::Ignore, "TIME.GT.24")
            .unwrap();
        assert!(
            m.model_content().contains("IGNORE=@ IGNORE=(AMT.GT.100)\n"),
            "{}",
            m.model_content()
        );
        let m = m
            .remove_data_filter(FilterKind::Ignore, "AMT.GT.100")
            .unwrap();
        assert!(
            m.model_content().contains("$DATA ../data.csv IGNORE=@\n"),
            "{}",
            m.model_content()
        );
        assert!(m.add_data_filter(FilterKind::Ignore, "NOPE.EQ.1").is_err());
        assert!(m.remove_data_filter(FilterKind::Ignore, "ID.EQ.9").is_err());
        let m = m.add_data_filter(FilterKind::Ignore, "ID.EQ.9").unwrap();
        assert!(m.add_data_filter(FilterKind::Accept, "ID.EQ.1").is_err());
    }

    #[test]
    fn est_add_remove() {
        let m = model()
            .add_est(&[
                ("method".into(), OptionEdit::Value("IMP".into())),
                ("interaction".into(), OptionEdit::Flag),
                ("eonly".into(), OptionEdit::Value("1".into())),
            ])
            .unwrap();
        assert_eq!(m.estimations.len(), 2);
        assert!(
            m.model_content()
                .contains(" PRINT=5 ; c\n$EST METHOD=IMP INTERACTION EONLY=1\n$COV"),
            "{}",
            m.model_content()
        );
        let m = m.remove_est(0).unwrap();
        assert!(
            m.model_content().contains("$SIGMA 0.1\n$EST METHOD=IMP"),
            "{}",
            m.model_content()
        );
        assert!(m.remove_est(0).is_err());

        let two = Model::inner_parse(
            "$PROBLEM x\n$INPUT ID DV\n$DATA d.csv\n$PRED Y = THETA(1) + ETA(1) + EPS(1)\n\
             $THETA 1\n$OMEGA 0.1\n$SIGMA 0.1\n\n; first\n$EST METHOD=1\n\n\
             ; second\n ; step\n$EST METHOD=IMP EONLY=1\n\n$COV\n",
        )
        .unwrap();
        let m = two.remove_est(1).unwrap();
        assert!(
            m.model_content()
                .contains("$SIGMA 0.1\n\n; first\n$EST METHOD=1\n\n$COV"),
            "{}",
            m.model_content()
        );
        assert!(
            model()
                .add_est(&[
                    ("max".into(), OptionEdit::Value("1".into())),
                    ("maxeval".into(), OptionEdit::Value("1".into()))
                ])
                .is_err()
        );
    }

    #[test]
    fn undefined_names_refused() {
        let err = model()
            .add_statement(CodeRecord::Pk, "CL = THETA(3) * (AGE / 40)")
            .unwrap_err()
            .to_string();
        assert!(err.contains("`AGE`"), "{err}");
    }
}
