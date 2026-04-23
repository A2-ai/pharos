use crate::ast::{
    Abbreviated, BlockStructure, CodeBlock, ComparisonOperator, Covariance, Data, DataFilter,
    DataValueFilter, DataValueFilterKind, DiagonalScale, Distribution, Estimation,
    EstimationMethod, InputColumn, InputColumnKind, Msfi, OffDiagonalScale, OmegaSigmaBlock,
    OmegaSigmaParam, Parametrization, Problem, Replace, SeedGroup, Simulation, Subroutine,
    Subroutines, Table, ThetaParameter, TrueKind,
};
use crate::cst::{CstChild, CstNode, NodeKind};
use crate::errors::Diagnostic;
use crate::lexer::{SpannedToken, Token};
use crate::model::Model;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Semantic classification of an omega/sigma flag keyword.
#[derive(Debug, Clone, PartialEq)]
enum OmegaSigmaFlagKind {
    Fix,
    Diagonal(DiagonalScale),
    OffDiagonal(OffDiagonalScale),
    Cholesky,
}

impl OmegaSigmaFlagKind {
    fn from_str(text: &str) -> Option<Self> {
        match text.to_uppercase().as_str() {
            "FIX" | "FIXED" => Some(OmegaSigmaFlagKind::Fix),
            "SD" | "STANDARD" => Some(OmegaSigmaFlagKind::Diagonal(
                DiagonalScale::StandardDeviation,
            )),
            "VAR" | "VARIANCE" => Some(OmegaSigmaFlagKind::Diagonal(DiagonalScale::Variance)),
            "CORR" | "CORRELATION" => Some(OmegaSigmaFlagKind::OffDiagonal(
                OffDiagonalScale::Correlation,
            )),
            "COV" | "COVAR" | "COVARIANCE" => Some(OmegaSigmaFlagKind::OffDiagonal(
                OffDiagonalScale::Covariance,
            )),
            "CHOLESKY" => Some(OmegaSigmaFlagKind::Cholesky),
            _ => None,
        }
    }

    /// Convert to a Parametrization, returning None for Fix (which is not a parametrization).
    fn to_parametrization(&self) -> Option<Parametrization> {
        match self {
            OmegaSigmaFlagKind::Fix => None,
            OmegaSigmaFlagKind::Cholesky => Some(Parametrization::Cholesky),
            OmegaSigmaFlagKind::Diagonal(d) => Some(Parametrization::Axes {
                diagonal: Some(*d),
                off_diagonal: None,
            }),
            OmegaSigmaFlagKind::OffDiagonal(od) => Some(Parametrization::Axes {
                diagonal: None,
                off_diagonal: Some(*od),
            }),
        }
    }
}

/// A `Param` node extracted from the CST, shared between `lower_diagonal`
/// and `lower_block` to avoid traversing the node twice.
struct ParsedParam {
    /// Optional `NAME=` label attached to this param.
    name: Option<String>,
    /// Token indices of the numeric values for this param, in source order.
    /// For paren-form params these are drawn from inside the `Parens` child node.
    nums: Vec<usize>,
    /// `Some(n)` if an `xN` repeat suffix was present; `None` otherwise.
    /// When `Some`, `nums[0]` is emitted `n` times. When `None`, each entry
    /// in `nums` contributes one element.
    repeat: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Lowerer<'a> {
    tokens: &'a [SpannedToken],
    errors: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    pub fn new(tokens: &'a [SpannedToken]) -> Self {
        Self {
            tokens,
            errors: Vec::new(),
        }
    }

    fn push_error(&mut self, diagnostic: Diagnostic) {
        self.errors.push(diagnostic);
    }

    pub(crate) fn non_trivia_children(&self, node: &CstNode) -> Vec<usize> {
        node.children
            .iter()
            .filter_map(|c| match c {
                CstChild::Token(idx)
                    if !matches!(
                        self.tokens[*idx].token,
                        Token::Whitespace | Token::Newline | Token::Comment
                    ) =>
                {
                    Some(*idx)
                }
                _ => None,
            })
            .collect()
    }

    fn find_first_child<'n>(&self, node: &'n CstNode, kind: NodeKind) -> Option<&'n CstNode> {
        node.children.iter().find_map(|c| match c {
            CstChild::Node(n) if n.kind == kind => Some(n),
            _ => None,
        })
    }

    fn find_all_children<'n>(&self, node: &'n CstNode, kind: NodeKind) -> Vec<&'n CstNode> {
        node.children
            .iter()
            .filter_map(|c| match c {
                CstChild::Node(n) if n.kind == kind => Some(n),
                _ => None,
            })
            .collect()
    }

    fn extract_names(&self, names_node: &CstNode) -> Vec<String> {
        self.non_trivia_children(names_node)
            .iter()
            .filter(|&&i| {
                self.tokens[i].token == Token::Symbol
                    && !self.tokens[i].text.eq_ignore_ascii_case("NAMES")
            })
            .map(|&i| self.tokens[i].text.clone())
            .collect()
    }

    fn find_repeat_number(&self, node: &CstNode) -> Option<usize> {
        let rep = self.find_first_child(node, NodeKind::Repeat)?;
        let idx = self.non_trivia_children(rep).into_iter().next()?;
        self.tokens[idx]
            .text
            .strip_prefix('x')
            .or_else(|| self.tokens[idx].text.strip_prefix('X'))?
            .parse()
            .ok()
    }

    fn find_same_repeats(&self, node: &CstNode) -> Option<usize> {
        let same = self.find_first_child(node, NodeKind::Same)?;
        Some(
            self.non_trivia_children(same)
                .into_iter()
                .find(|&i| self.tokens[i].token == Token::Int)
                .and_then(|i| self.tokens[i].text.parse::<usize>().ok())
                .unwrap_or(1),
        )
    }

    fn parse_number(&self, idx: usize) -> f64 {
        let tok = &self.tokens[idx];
        match tok.token {
            Token::Infinity if tok.text.starts_with('-') => f64::NEG_INFINITY,
            Token::Infinity => f64::INFINITY,
            _ => tok.text.parse::<f64>().unwrap(),
        }
    }

    fn token_value(&self, idx: usize) -> String {
        let tok = &self.tokens[idx];
        match tok.token {
            Token::QuotedString => tok.text.trim_matches('"').trim_matches('\'').to_string(),
            _ => tok.text.clone(),
        }
    }

    fn has_fix(&self, node: &CstNode) -> bool {
        self.find_all_children(node, NodeKind::Flag)
            .iter()
            .any(|flag| {
                self.non_trivia_children(flag).iter().any(|&i| {
                    let t = &self.tokens[i].text;
                    t.eq_ignore_ascii_case("FIX") || t.eq_ignore_ascii_case("FIXED")
                })
            })
    }

    fn collect_options(&self, node: &CstNode) -> BTreeMap<String, Option<String>> {
        let mut options = BTreeMap::new();

        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::Flag => {
                    let toks = self.non_trivia_children(n);
                    if toks
                        .iter()
                        .any(|&i| self.tokens[i].token == Token::LeftParen)
                    {
                        continue;
                    }
                    let text: String = toks
                        .iter()
                        .map(|&i| self.tokens[i].text.as_str())
                        .collect::<String>()
                        .to_uppercase();
                    options.insert(text, None);
                }
                NodeKind::KeyValue => {
                    let toks = self.non_trivia_children(n);
                    let key = self.tokens[toks[0]].text.to_uppercase();
                    let val = self.token_value(*toks.last().unwrap());
                    options.insert(key, Some(val));
                }
                _ => {}
            }
        }

        options
    }

    fn find_kv_value_token(&self, node: &CstNode, key: &str) -> Option<usize> {
        for child in &node.children {
            let CstChild::Node(kv) = child else { continue };
            if kv.kind != NodeKind::KeyValue {
                continue;
            }
            let toks = self.non_trivia_children(kv);
            if self.tokens[toks[0]].text.eq_ignore_ascii_case(key) {
                return Some(*toks.last().unwrap());
            }
        }
        None
    }

    fn parse_value(s: &str) -> DataValueFilterKind {
        match s.parse::<f64>() {
            Ok(n) => DataValueFilterKind::Number(n),
            Err(_) => DataValueFilterKind::String(s.to_string()),
        }
    }

    fn parse_filter(&mut self, filter: &CstNode) -> Option<DataFilter> {
        let toks = self.non_trivia_children(filter);
        if toks.is_empty() {
            return None;
        }

        let error_span = self.tokens[toks[0]].span.clone();

        // Concatenate all non-trivia token texts into a single string
        let joined: String = toks.iter().map(|&i| self.tokens[i].text.as_str()).collect();

        // 1. Dotted operators: FIELD.OP.VALUE via splitn(3, '.')
        let dot_parts: Vec<&str> = joined.splitn(3, '.').collect();
        if dot_parts.len() == 3
            && !dot_parts[0].is_empty()
            && let Ok(op) = dot_parts[1].to_uppercase().parse::<ComparisonOperator>()
        {
            let value = Self::parse_value(dot_parts[2]);
            return Some(DataFilter::ValueFilter(DataValueFilter {
                field: dot_parts[0].to_string(),
                op,
                value,
            }));
        }

        // 2. F90/symbolic operators (longest first)
        let f90_ops: &[(&str, ComparisonOperator)] = &[
            ("==", ComparisonOperator::Equal),
            ("/=", ComparisonOperator::NotEqual),
            (">=", ComparisonOperator::GreaterOrEqual),
            ("<=", ComparisonOperator::LowerOrEqual),
            (">", ComparisonOperator::Greater),
            ("<", ComparisonOperator::Lower),
            ("=", ComparisonOperator::Equal),
        ];
        for &(sym, op) in f90_ops {
            if let Some(pos) = joined.find(sym)
                && pos > 0
                && pos + sym.len() < joined.len()
            {
                let field = &joined[..pos];
                let val_str = &joined[pos + sym.len()..];
                let value = Self::parse_value(val_str);
                return Some(DataFilter::ValueFilter(DataValueFilter {
                    field: field.to_string(),
                    op,
                    value,
                }));
            }
        }

        // 3. Implicit equality: exactly 2 tokens → FIELD VALUE
        if toks.len() == 2 {
            let field = self.tokens[toks[0]].text.clone();
            let value = Self::parse_value(&self.tokens[toks[1]].text);
            return Some(DataFilter::ValueFilter(DataValueFilter {
                field,
                op: ComparisonOperator::Equal,
                value,
            }));
        }

        let text: String = toks
            .iter()
            .map(|&i| self.tokens[i].text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        self.push_error(Diagnostic::lowering(
            format!("invalid filter: '{text}'"),
            error_span,
        ));
        None
    }

    fn lower_problem(&self, node: &CstNode) -> String {
        let mut out = String::new();
        for child in &node.children[1..] {
            if let CstChild::Token(idx) = child {
                out.push_str(&self.tokens[*idx].text);
            }
        }

        out.trim().to_string()
    }

    fn lower_input(&self, node: &CstNode) -> Vec<InputColumn> {
        let mut out = vec![];

        for (child_idx, child) in node.children.iter().enumerate() {
            // skip trivia
            let CstChild::Node(col) = child else { continue };
            if col.kind != NodeKind::InputColumn {
                continue;
            }

            let indices = self.non_trivia_children(col);
            if indices.len() == 1 {
                out.push(InputColumn {
                    kind: InputColumnKind::Included(self.tokens[indices[0]].text.clone()),
                    child_idx,
                });
            } else if indices.len() == 3 {
                // SOMETHING=A
                let a = &self.tokens[indices[0]].text;
                let b = &self.tokens[indices[2]].text;

                let kind = if a.eq_ignore_ascii_case("DROP") || a.eq_ignore_ascii_case("SKIP") {
                    InputColumnKind::Dropped(b.to_owned())
                } else if b.eq_ignore_ascii_case("DROP") || b.eq_ignore_ascii_case("SKIP") {
                    InputColumnKind::Dropped(a.to_owned())
                } else {
                    InputColumnKind::Aliased {
                        from: a.to_string(),
                        to: b.to_string(),
                    }
                };
                out.push(InputColumn { kind, child_idx });
            }
        }

        out
    }

    fn lower_data(&mut self, node: &CstNode) -> Data {
        let mut data = Data::default();

        // First non trivia token is the path
        for child in &node.children[1..] {
            if let CstChild::Token(idx) = child {
                let tok = &self.tokens[*idx];
                data.path = match tok.token {
                    Token::QuotedString | Token::Symbol => self.token_value(*idx),
                    _ => continue,
                };
                data.path_idx = Some(*idx);
                break;
            }
        }

        let mut first_ignore_idx: Option<usize> = None;
        let mut first_accept_idx: Option<usize> = None;

        // Then the options
        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::KeyValue => {
                    let indices = self.non_trivia_children(n);
                    let keyword = self.tokens[indices[0]].text.to_uppercase();
                    let value = &self.tokens[*indices.last().unwrap()].text;

                    match keyword.as_ref() {
                        "IGNORE" | "IGN" | "ACCEPT" => {
                            let target = if keyword == "ACCEPT" {
                                first_accept_idx.get_or_insert(indices[0]);
                                &mut data.accept
                            } else {
                                first_ignore_idx.get_or_insert(indices[0]);
                                &mut data.ignore
                            };
                            if let Some(parens) = self.find_first_child(n, NodeKind::Parens) {
                                for filter in self.find_all_children(parens, NodeKind::Filter) {
                                    match self.parse_filter(filter) {
                                        Some(f) => target.push(f),
                                        None => continue,
                                    }
                                }
                            } else {
                                target.push(DataFilter::Marker(value.clone()));
                            }
                        }
                        "RECORDS" => {
                            data.num_records = match usize::from_str(value) {
                                Ok(v) => Some(v),
                                Err(_) => {
                                    let span = self.tokens[*indices.last().unwrap()].span.clone();
                                    self.push_error(Diagnostic::lowering(
                                        format!("RECORDS value '{value}' is not a valid integer"),
                                        span,
                                    ));
                                    None
                                }
                            };
                        }
                        "NULL" => {
                            data.null_value = Some(value.to_owned());
                        }
                        _ => {
                            data.other_options.push((keyword, Some(value.to_owned())));
                        }
                    }
                }
                NodeKind::Flag => {
                    let toks = self.non_trivia_children(n);
                    let text = self.tokens[toks[0]].text.to_uppercase();
                    data.other_options.push((text, None));
                }
                _ => {}
            }
        }

        if !data.ignore.is_empty() && !data.accept.is_empty() {
            let span_idx = if first_ignore_idx < first_accept_idx {
                first_accept_idx.unwrap()
            } else {
                first_ignore_idx.unwrap()
            };
            self.push_error(Diagnostic::lowering(
                "ACCEPT and IGNORE cannot both be specified in $DATA",
                self.tokens[span_idx].span.clone(),
            ));
        }

        data
    }

    fn lower_theta(&mut self, node: &CstNode, record_idx: usize) -> Vec<ThetaParameter> {
        let mut params: Vec<ThetaParameter> = vec![];

        // We can have an optional NAMES arg
        let names: Vec<String> = self
            .find_first_child(node, NodeKind::ParamNames)
            .map(|n| self.extract_names(n))
            .unwrap_or_default();

        // We want to repeat the attached comment for each param in a line or in xN repeat
        let mut batch_start = 0;

        for (child_idx, child) in node.children.iter().enumerate() {
            match child {
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Newline => {
                    batch_start = params.len();
                }
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Comment => {
                    let text = self.tokens[*idx].text.trim_start_matches(';').trim();
                    if !text.is_empty() {
                        for p in params[batch_start..].iter_mut() {
                            p.comment = Some(text.to_string());
                        }
                    }
                }
                CstChild::Node(param) if param.kind == NodeKind::Param => {
                    let indices = self.non_trivia_children(param);

                    // Check if it's a named theta, eg CL=(..) for example and grab the name
                    let is_named = indices
                        .first()
                        .map(|&i| self.tokens[i].token == Token::Symbol)
                        .unwrap_or(false)
                        && indices
                            .get(1)
                            .map(|&i| self.tokens[i].token == Token::Equals)
                            .unwrap_or(false);
                    let name = if is_named {
                        Some(self.tokens[indices[0]].text.clone())
                    } else {
                        None
                    };

                    if let Some(parens) = self.find_first_child(param, NodeKind::Parens) {
                        let repeat = self.find_repeat_number(param).unwrap_or(1);
                        let nums: Vec<usize> = self
                            .non_trivia_children(parens)
                            .into_iter()
                            .filter(|&i| {
                                matches!(
                                    self.tokens[i].token,
                                    Token::Int | Token::Float | Token::Infinity
                                )
                            })
                            .collect();
                        // can be inside or outside params
                        let fixed = self.has_fix(parens) || self.has_fix(param);

                        let (lower, lower_idx, init, init_idx, upper, upper_idx) = match nums.len()
                        {
                            1 => {
                                let v = self.parse_number(nums[0]);
                                (None, None, v, nums[0], None, None)
                            }
                            2 => {
                                let lo = self.parse_number(nums[0]);
                                let ini = self.parse_number(nums[1]);
                                (Some(lo), Some(nums[0]), ini, nums[1], None, None)
                            }
                            3 => {
                                let lo = self.parse_number(nums[0]);
                                let ini = self.parse_number(nums[1]);
                                let up = self.parse_number(nums[2]);
                                (
                                    Some(lo),
                                    Some(nums[0]),
                                    ini,
                                    nums[1],
                                    Some(up),
                                    Some(nums[2]),
                                )
                            }
                            _ => {
                                let span = self.tokens[nums[0]].span.clone();
                                self.push_error(Diagnostic::lowering(
                                    format!(
                                        "expected 1 to 3 numeric values in theta bounds, found {}",
                                        nums.len()
                                    ),
                                    span,
                                ));
                                continue;
                            }
                        };

                        // Validate: lower <= init <= upper
                        let lo = lower.unwrap_or(f64::NEG_INFINITY);
                        let up = upper.unwrap_or(f64::INFINITY);
                        if lo > init || init > up {
                            let span = self.tokens[init_idx].span.clone();
                            self.push_error(Diagnostic::lowering(
                                "theta bounds violated: requires lower <= init <= upper",
                                span,
                            ));
                        }
                        let base = ThetaParameter {
                            name,
                            lower,
                            init,
                            upper,
                            fixed,
                            comment: None,
                            parsed_comment: None,
                            record_idx,
                            param_child_idx: child_idx,
                            lower_idx,
                            init_idx,
                            upper_idx,
                        };
                        params.extend(std::iter::repeat_n(base, repeat));
                    } else {
                        // just a number
                        let num_idx = indices.iter().copied().find(|&i| {
                            matches!(
                                self.tokens[i].token,
                                Token::Int | Token::Float | Token::Infinity
                            )
                        });
                        if let Some(num_idx) = num_idx {
                            let init = self.parse_number(num_idx);
                            let fixed = self.has_fix(param);
                            params.push(ThetaParameter {
                                name,
                                lower: None,
                                init,
                                upper: None,
                                fixed,
                                comment: None,
                                parsed_comment: None,
                                record_idx,
                                param_child_idx: child_idx,
                                lower_idx: None,
                                init_idx: num_idx,
                                upper_idx: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if !names.is_empty() {
            if names.len() != params.len() {
                let names_node = self.find_first_child(node, NodeKind::ParamNames).unwrap();
                let span = self
                    .non_trivia_children(names_node)
                    .first()
                    .map(|&i| self.tokens[i].span.clone())
                    .unwrap_or_default();
                self.push_error(Diagnostic::lowering(
                    format!(
                        "NAMES count ({}) does not match parameter count ({})",
                        names.len(),
                        params.len()
                    ),
                    span,
                ));
            }
            for (i, name) in names.into_iter().enumerate() {
                if i < params.len() {
                    params[i].name = Some(name);
                }
            }
        }

        params
    }

    fn flags_from_node(&self, node: &CstNode) -> Vec<(OmegaSigmaFlagKind, usize)> {
        let mut out = Vec::new();
        // For paren-form Param nodes, flags inside the Parens child apply to this param.
        if let Some(parens) = self.find_first_child(node, NodeKind::Parens) {
            for child in &parens.children {
                let CstChild::Node(flag_node) = child else {
                    continue;
                };
                if flag_node.kind != NodeKind::Flag {
                    continue;
                }
                for &idx in &self.non_trivia_children(flag_node) {
                    if let Some(kind) = OmegaSigmaFlagKind::from_str(&self.tokens[idx].text) {
                        out.push((kind, idx));
                    }
                }
            }
        }
        for child in &node.children {
            let CstChild::Node(flag_node) = child else {
                continue;
            };
            if flag_node.kind != NodeKind::Flag {
                continue;
            }
            for &idx in &self.non_trivia_children(flag_node) {
                if let Some(kind) = OmegaSigmaFlagKind::from_str(&self.tokens[idx].text) {
                    out.push((kind, idx));
                }
            }
        }
        out
    }

    fn has_non_fix_flag_in_parens(&self, param: &CstNode) -> bool {
        let Some(parens) = self.find_first_child(param, NodeKind::Parens) else {
            return false;
        };
        self.find_all_children(parens, NodeKind::Flag)
            .iter()
            .any(|flag| {
                self.non_trivia_children(flag).iter().any(|&idx| {
                    matches!(
                        OmegaSigmaFlagKind::from_str(&self.tokens[idx].text),
                        Some(k) if !matches!(k, OmegaSigmaFlagKind::Fix)
                    )
                })
            })
    }

    fn has_split_trigger(&self, param: &CstNode) -> bool {
        self.flags_from_node(param).iter().any(|(kind, _)| {
            matches!(
                kind,
                OmegaSigmaFlagKind::Fix
                    | OmegaSigmaFlagKind::Diagonal(DiagonalScale::StandardDeviation)
            )
        })
    }

    fn build_block_parametrization(
        &mut self,
        flags: &[(OmegaSigmaFlagKind, usize)],
        size: usize,
    ) -> (Option<Parametrization>, bool) {
        let fixed = flags
            .iter()
            .any(|(k, _)| matches!(k, OmegaSigmaFlagKind::Fix));
        let cholesky = flags
            .iter()
            .find(|(k, _)| matches!(k, OmegaSigmaFlagKind::Cholesky));
        let diagonals: Vec<_> = flags
            .iter()
            .filter(|(k, _)| matches!(k, OmegaSigmaFlagKind::Diagonal(_)))
            .collect();
        let off_diagonals: Vec<_> = flags
            .iter()
            .filter(|(k, _)| matches!(k, OmegaSigmaFlagKind::OffDiagonal(_)))
            .collect();

        if diagonals.len() > 1 {
            self.push_error(
                Diagnostic::lowering(
                    "duplicate diagonal axis flag: SD and VAR cannot both be specified",
                    self.tokens[diagonals[1].1].span.clone(),
                )
                .with_note(
                    "first specified here",
                    self.tokens[diagonals[0].1].span.clone(),
                ),
            );
        }
        if off_diagonals.len() > 1 {
            self.push_error(
                Diagnostic::lowering(
                    "duplicate off-diagonal axis flag: CORR and COV cannot both be specified",
                    self.tokens[off_diagonals[1].1].span.clone(),
                )
                .with_note(
                    "first specified here",
                    self.tokens[off_diagonals[0].1].span.clone(),
                ),
            );
        }
        if let Some((_, idx)) = cholesky {
            let conflicting: Vec<_> = diagonals.iter().chain(off_diagonals.iter()).collect();
            if !conflicting.is_empty() {
                let mut diag = Diagnostic::lowering(
                    "CHOLESKY is mutually exclusive with SD, VAR, CORR, and COV",
                    self.tokens[*idx].span.clone(),
                );
                for (_, flag_idx) in &conflicting {
                    diag = diag
                        .with_note("conflicting flag here", self.tokens[*flag_idx].span.clone());
                }
                self.push_error(diag);
            }
        }
        if size == 1
            && let Some((_, idx)) = off_diagonals.first()
        {
            self.push_error(Diagnostic::lowering(
                "CORR and COV require n >= 2 — BLOCK(1) has no off-diagonal elements",
                self.tokens[*idx].span.clone(),
            ));
        }

        let parametrization = if cholesky.is_some() {
            Some(Parametrization::Cholesky)
        } else {
            let diagonal = diagonals.first().map(|(k, _)| match k {
                OmegaSigmaFlagKind::Diagonal(d) => *d,
                _ => unreachable!(),
            });
            let off_diagonal = off_diagonals.first().map(|(k, _)| match k {
                OmegaSigmaFlagKind::OffDiagonal(od) => *od,
                _ => unreachable!(),
            });
            if diagonal.is_some() || off_diagonal.is_some() {
                Some(Parametrization::Axes {
                    diagonal,
                    off_diagonal,
                })
            } else {
                None
            }
        };

        (parametrization, fixed)
    }

    fn parse_omega_sigma_param(&self, param: &CstNode) -> ParsedParam {
        // Paren form: (value [flags]) [xN] — the numeric value lives inside the Parens child.
        if let Some(parens) = self.find_first_child(param, NodeKind::Parens) {
            let nums: Vec<usize> = self
                .non_trivia_children(parens)
                .into_iter()
                .filter(|&i| {
                    matches!(
                        self.tokens[i].token,
                        Token::Int | Token::Float | Token::Infinity
                    )
                })
                .collect();
            return ParsedParam {
                name: None,
                nums,
                repeat: self.find_repeat_number(param),
            };
        }

        // Bare or named form: all tokens are direct children of Param.
        let non_trivia = self.non_trivia_children(param);

        let is_named = non_trivia
            .first()
            .map(|&i| self.tokens[i].token == Token::Symbol)
            .unwrap_or(false)
            && non_trivia
                .get(1)
                .map(|&i| self.tokens[i].token == Token::Equals)
                .unwrap_or(false);
        let name = is_named.then(|| self.tokens[non_trivia[0]].text.clone());

        let nums: Vec<usize> = non_trivia
            .iter()
            .copied()
            .filter(|&i| {
                matches!(
                    self.tokens[i].token,
                    Token::Int | Token::Float | Token::Infinity
                )
            })
            .collect();

        ParsedParam {
            name,
            nums,
            repeat: None,
        }
    }

    fn lower_omega_sigma(&mut self, node: &CstNode, record_idx: usize) -> Vec<OmegaSigmaBlock> {
        // 1. Determine structure
        let same_repeats = self.find_same_repeats(node);
        let structure = if let Some(block) = self.find_first_child(node, NodeKind::Block) {
            let size = self
                .non_trivia_children(block)
                .iter()
                .find(|&&i| self.tokens[i].token == Token::Int)
                .and_then(|&i| self.tokens[i].text.parse::<usize>().ok())
                .unwrap_or(1);
            if let Some(repeats) = same_repeats {
                BlockStructure::BlockSame { size, repeats }
            } else {
                BlockStructure::Block { size }
            }
        } else if same_repeats.is_some() {
            unreachable!("SAME without BLOCK must be rejected by the parser")
        } else {
            BlockStructure::Diagonal
        };

        match structure {
            BlockStructure::Diagonal => self.lower_diagonal(node, record_idx),
            BlockStructure::Block { size } => {
                vec![self.lower_block(node, record_idx, size)]
            }
            BlockStructure::BlockSame { size, repeats } => {
                vec![self.lower_block_same(node, record_idx, size, repeats)]
            }
        }
    }

    fn lower_diagonal(&mut self, node: &CstNode, record_idx: usize) -> Vec<OmegaSigmaBlock> {
        // Reject NAMES on diagonal (requires BLOCK)
        if let Some(names_node) = self.find_first_child(node, NodeKind::ParamNames) {
            let span = self
                .non_trivia_children(names_node)
                .first()
                .map(|&i| self.tokens[i].span.clone())
                .unwrap_or_default();
            self.push_error(Diagnostic::lowering(
                "NAMES requires BLOCK — it is not valid on a diagonal $OMEGA/$SIGMA record",
                span,
            ));
        }

        // Reject record-level flags
        for flag in self.find_all_children(node, NodeKind::Flag) {
            if let Some(&idx) = self.non_trivia_children(flag).first() {
                self.push_error(Diagnostic::lowering(
                    format!(
                        "{} must appear inline after a value, not at record level",
                        self.tokens[idx].text
                    ),
                    self.tokens[idx].span.clone(),
                ));
            }
        }

        // Validate inline flags on each param
        for child in &node.children {
            let CstChild::Node(param) = child else {
                continue;
            };
            if param.kind != NodeKind::Param {
                continue;
            }
            let mut seen_parametrization: Option<usize> = None;
            for (kind, idx) in self.flags_from_node(param) {
                if matches!(kind, OmegaSigmaFlagKind::OffDiagonal(_)) {
                    self.push_error(Diagnostic::lowering(
                        format!(
                            "off-diagonal flag {} is not valid on diagonal $OMEGA/$SIGMA values",
                            self.tokens[idx].text
                        ),
                        self.tokens[idx].span.clone(),
                    ));
                } else if kind.to_parametrization().is_some() {
                    if seen_parametrization.is_some() {
                        self.push_error(Diagnostic::lowering(
                            format!(
                                "conflicting parametrization flag {}: only one of CHOLESKY, SD, or VAR may be specified per value",
                                self.tokens[idx].text
                            ),
                            self.tokens[idx].span.clone(),
                        ));
                    } else {
                        seen_parametrization = Some(idx);
                    }
                }
            }
        }

        // Check for split-triggering flags
        let splitting = node.children.iter().any(|c| {
            matches!(c, CstChild::Node(p) if p.kind == NodeKind::Param && self.has_split_trigger(p))
        });

        // Collect params — same single-walk pattern as lower_theta
        let mut parameters: Vec<OmegaSigmaParam> = Vec::new();
        let mut batch_start = 0;

        for (child_idx, child) in node.children.iter().enumerate() {
            match child {
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Newline => {
                    batch_start = parameters.len();
                }
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Comment => {
                    let text = self.tokens[*idx].text.trim_start_matches(';').trim();
                    if !text.is_empty() {
                        for p in parameters[batch_start..].iter_mut() {
                            p.comment = Some(text.to_string());
                        }
                    }
                }
                CstChild::Node(param) if param.kind == NodeKind::Param => {
                    let parsed = self.parse_omega_sigma_param(param);
                    let Some(&num_idx) = parsed.nums.first() else {
                        continue;
                    };
                    let value = self.parse_number(num_idx);

                    for _ in 0..parsed.repeat.unwrap_or(1) {
                        parameters.push(OmegaSigmaParam {
                            value,
                            name: parsed.name.clone(),
                            comment: None,
                            parsed_comment: None,
                            param_child_idx: child_idx,
                            value_idx: num_idx,
                        });
                    }
                }
                _ => {}
            }
        }

        if splitting {
            self.lower_split_diagonal(node, record_idx, parameters)
        } else {
            let parametrization = self.uniform_diagonal_parametrization(node);
            vec![OmegaSigmaBlock {
                structure: BlockStructure::Diagonal,
                parametrization,
                fixed: false,
                names: vec![],
                parameters,
                record_idx,
            }]
        }
    }

    /// Splits a single diagonal record into one Diagonal(1) block per parameter,
    /// each carrying its own parametrization and fixed flag.
    ///
    /// NONMEM splits a diagonal record whenever any parameter carries a FIX or SD
    /// flag — each parameter becomes its own block. See `docs/nm-test-results.md`
    /// case02_diag_sd and case03_diag_fix for concrete examples.
    fn lower_split_diagonal(
        &self,
        node: &CstNode,
        record_idx: usize,
        parameters: Vec<OmegaSigmaParam>,
    ) -> Vec<OmegaSigmaBlock> {
        parameters
            .into_iter()
            .map(|p| {
                let CstChild::Node(param_node) = &node.children[p.param_child_idx] else {
                    unreachable!("param_child_idx must point to a Param node");
                };
                let flags: Vec<OmegaSigmaFlagKind> = self
                    .flags_from_node(param_node)
                    .into_iter()
                    .map(|(k, _)| k)
                    .collect();

                OmegaSigmaBlock {
                    structure: BlockStructure::Diagonal,
                    parametrization: flags.iter().find_map(|f| f.to_parametrization()),
                    fixed: flags.iter().any(|f| matches!(f, OmegaSigmaFlagKind::Fix)),
                    names: vec![],
                    parameters: vec![p],
                    record_idx,
                }
            })
            .collect()
    }

    /// Returns the shared parametrization if all params carry the same
    /// non-split-triggering flag, or `None` if coverage is partial or mixed.
    fn uniform_diagonal_parametrization(&self, node: &CstNode) -> Option<Parametrization> {
        let mut uniform: Option<OmegaSigmaFlagKind> = None;
        let mut any_flagged = false;
        let mut any_unflagged = false;

        for child in &node.children {
            let CstChild::Node(param) = child else {
                continue;
            };
            if param.kind != NodeKind::Param {
                continue;
            }

            let non_fix: Vec<OmegaSigmaFlagKind> = self
                .flags_from_node(param)
                .into_iter()
                .map(|(k, _)| k)
                .filter(|k| !matches!(k, OmegaSigmaFlagKind::Fix))
                .collect();

            if non_fix.is_empty() {
                any_unflagged = true;
            } else {
                any_flagged = true;
                for kind in non_fix {
                    match &uniform {
                        None => uniform = Some(kind),
                        Some(existing) if *existing == kind => {}
                        _ => return None,
                    }
                }
            }
        }

        if any_flagged && any_unflagged {
            return None;
        }

        uniform.and_then(|f| f.to_parametrization())
    }

    fn lower_block(&mut self, node: &CstNode, record_idx: usize, size: usize) -> OmegaSigmaBlock {
        // Collect all flags: record-level + inline on Param nodes
        let mut all_flags: Vec<(OmegaSigmaFlagKind, usize)> = Vec::new();

        all_flags.extend(self.flags_from_node(node));

        for child in &node.children {
            let CstChild::Node(param) = child else {
                continue;
            };
            if param.kind != NodeKind::Param {
                continue;
            }

            if self.has_non_fix_flag_in_parens(param) {
                let span = self
                    .find_first_child(param, NodeKind::Parens)
                    .and_then(|parens| {
                        parens.children.iter().find_map(|c| {
                            if let CstChild::Token(i) = c
                                && self.tokens[*i].token == Token::LeftParen
                            {
                                Some(self.tokens[*i].span.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default();
                self.push_error(Diagnostic::lowering(
                    "parametrization flags inside parentheses are not valid in a BLOCK record",
                    span,
                ));
                continue;
            }

            all_flags.extend(self.flags_from_node(param));
        }

        let (parametrization, fixed) = self.build_block_parametrization(&all_flags, size);

        // NAMES field
        let names: Vec<String> = self
            .find_first_child(node, NodeKind::ParamNames)
            .map(|n| self.extract_names(n))
            .unwrap_or_default();

        // VALUES field
        let values_nums: Vec<f64> = self
            .find_first_child(node, NodeKind::ParamValues)
            .map(|pv| {
                self.non_trivia_children(pv)
                    .iter()
                    .filter(|&&i| matches!(self.tokens[i].token, Token::Int | Token::Float))
                    .map(|&i| self.parse_number(i))
                    .collect()
            })
            .unwrap_or_default();

        // Collect parameters
        let mut parameters: Vec<OmegaSigmaParam> = Vec::new();
        let mut batch_start = 0;

        for (child_idx, child) in node.children.iter().enumerate() {
            match child {
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Newline => {
                    batch_start = parameters.len();
                }
                CstChild::Token(idx) if self.tokens[*idx].token == Token::Comment => {
                    let text = self.tokens[*idx].text.trim_start_matches(';').trim();
                    if !text.is_empty() {
                        for p in parameters[batch_start..].iter_mut() {
                            p.comment = Some(text.to_string());
                        }
                    }
                }
                CstChild::Node(param) if param.kind == NodeKind::Param => {
                    let parsed = self.parse_omega_sigma_param(param);

                    if let Some(repeat) = parsed.repeat {
                        let value = self.parse_number(parsed.nums[0]);
                        for _ in 0..repeat {
                            parameters.push(OmegaSigmaParam {
                                value,
                                name: parsed.name.clone(),
                                comment: None,
                                parsed_comment: None,
                                param_child_idx: child_idx,
                                value_idx: parsed.nums[0],
                            });
                        }
                    } else {
                        for (i, &num_idx) in parsed.nums.iter().enumerate() {
                            parameters.push(OmegaSigmaParam {
                                value: self.parse_number(num_idx),
                                name: if i == 0 { parsed.name.clone() } else { None },
                                comment: None,
                                parsed_comment: None,
                                param_child_idx: child_idx,
                                value_idx: num_idx,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Expand VALUES(diag, odiag) into full lower-triangle for BLOCK(n)
        if !values_nums.is_empty() && parameters.is_empty() && values_nums.len() == 2 {
            let (diag, odiag) = (values_nums[0], values_nums[1]);
            for row in 0..size {
                for col in 0..=row {
                    parameters.push(OmegaSigmaParam {
                        value: if row == col { diag } else { odiag },
                        name: None,
                        comment: None,
                        parsed_comment: None,
                        param_child_idx: 0,
                        value_idx: 0,
                    });
                }
            }
        }

        OmegaSigmaBlock {
            structure: BlockStructure::Block { size },
            parametrization,
            fixed,
            names,
            parameters,
            record_idx,
        }
    }

    fn lower_block_same(
        &mut self,
        node: &CstNode,
        record_idx: usize,
        size: usize,
        repeats: usize,
    ) -> OmegaSigmaBlock {
        // Reject any flags on a SAME block
        for child in &node.children {
            let CstChild::Node(flag_or_param) = child else {
                continue;
            };
            match flag_or_param.kind {
                NodeKind::Flag => {
                    let toks = self.non_trivia_children(flag_or_param);
                    if let Some(&idx) = toks.first() {
                        let span = self.tokens[idx].span.clone();
                        let text = &self.tokens[idx].text;
                        self.push_error(Diagnostic::lowering(
                            format!("{text} is not allowed on a SAME block"),
                            span,
                        ));
                    }
                }
                NodeKind::Param => {
                    // For paren-form params the value token is inside the Parens child.
                    let span = self
                        .non_trivia_children(flag_or_param)
                        .first()
                        .map(|&i| self.tokens[i].span.clone())
                        .or_else(|| {
                            self.find_first_child(flag_or_param, NodeKind::Parens)
                                .and_then(|parens| {
                                    self.non_trivia_children(parens)
                                        .first()
                                        .map(|&i| self.tokens[i].span.clone())
                                })
                        })
                        .unwrap_or_default();
                    self.push_error(Diagnostic::lowering(
                        "SAME block cannot contain parameter values",
                        span,
                    ));
                }
                _ => {}
            }
        }

        OmegaSigmaBlock {
            structure: BlockStructure::BlockSame { size, repeats },
            parametrization: None,
            fixed: false,
            names: vec![],
            parameters: vec![],
            record_idx,
        }
    }

    fn lower_estimation(&self, node: &CstNode, record_idx: usize) -> Estimation {
        let mut options = self.collect_options(node);
        let method = options
            .get("METHOD")
            .and_then(|v| v.as_deref())
            .map(|v| v.parse::<EstimationMethod>().unwrap_or_default())
            .unwrap_or_default();
        let msfo = options.remove("MSFO").flatten().map(PathBuf::from);
        let file = options.remove("FILE").flatten().map(PathBuf::from);
        let msfo_idx = self.find_kv_value_token(node, "MSFO");
        let file_idx = self.find_kv_value_token(node, "FILE");
        options.remove("METHOD");

        Estimation {
            method,
            msfo,
            file,
            options,
            record_idx,
            msfo_idx,
            file_idx,
        }
    }

    fn lower_table(&self, node: &CstNode, record_idx: usize) -> Table {
        let mut options = Vec::new();
        let mut file = None;

        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::Flag => {
                    let toks = self.non_trivia_children(n);
                    let text: String = toks
                        .iter()
                        .map(|&i| self.tokens[i].text.as_str())
                        .collect::<String>()
                        .to_uppercase();
                    options.push((text, None));
                }
                NodeKind::KeyValue => {
                    let toks = self.non_trivia_children(n);
                    let key = self.tokens[toks[0]].text.to_uppercase();
                    let val = self.token_value(*toks.last().unwrap());
                    if key == "FILE" {
                        file = Some(val);
                    } else {
                        options.push((key, Some(val)));
                    }
                }
                _ => {}
            }
        }

        let file_idx = self.find_kv_value_token(node, "FILE");
        Table {
            file,
            options,
            record_idx,
            file_idx,
        }
    }

    fn lower_seed_group(&mut self, flag: &CstNode, is_first: bool) -> SeedGroup {
        let toks = self.non_trivia_children(flag);
        let paren_pos = toks
            .iter()
            .position(|&i| self.tokens[i].token == Token::LeftParen);
        let seed_toks: &[usize] = match paren_pos {
            Some(pos) => &toks[pos + 1..],
            None => &toks,
        };

        let int_indices: Vec<usize> = seed_toks
            .iter()
            .filter(|&&i| self.tokens[i].token == Token::Int)
            .copied()
            .collect();

        if int_indices.len() > 2 {
            let n = int_indices.len();
            self.push_error(Diagnostic::lowering(
                format!(
                    "seed group allows at most 2 seed values (seed1 and optional seed2), found {n}"
                ),
                self.tokens[int_indices[2]].span.clone(),
            ));
        }

        if int_indices.is_empty() {
            let span = seed_toks
                .iter()
                .copied()
                .find(|&i| self.tokens[i].token != Token::RightParen)
                .or_else(|| toks.first().copied())
                .map(|i| self.tokens[i].span.clone())
                .unwrap_or_default();
            self.push_error(Diagnostic::lowering(
                "seed group requires an integer seed1",
                span,
            ));
            return SeedGroup::default();
        }

        let seed1 = self.parse_seed_value(int_indices[0]).unwrap_or(0);
        let seed2 = int_indices
            .get(1)
            .and_then(|&idx| self.parse_seed_value(idx));
        let (distribution, dist_idx, new) = self.scan_seed_group_symbols(seed_toks);

        if seed1 == -1
            && let Some(v) = seed2
            && v != 0
        {
            let seed2_idx = int_indices[1];
            self.push_error(Diagnostic::lowering(
                "seed2 must be 0 or omitted when seed1 is -1",
                self.tokens[seed2_idx].span.clone(),
            ));
        }

        if is_first
            && matches!(
                distribution,
                Some(Distribution::Uniform) | Some(Distribution::Nonparametric)
            )
            && let Some(idx) = dist_idx
        {
            let dist_name = self.tokens[idx].text.to_uppercase();
            self.push_error(Diagnostic::lowering(
                format!(
                    "first $SIMULATION seed group's distribution must be NORMAL, found {dist_name}"
                ),
                self.tokens[idx].span.clone(),
            ));
        }

        SeedGroup {
            seed1,
            seed2,
            distribution,
            new,
        }
    }

    fn parse_seed_value(&mut self, idx: usize) -> Option<i32> {
        let text = self.tokens[idx].text.clone();
        match text.parse::<i32>() {
            Ok(-1) => Some(-1),
            Ok(v) if v >= 0 => Some(v),
            _ => {
                self.push_error(Diagnostic::lowering(
                    format!(
                        "seed value '{text}' is out of range: must be -1 or an integer in [0, 2147483647]"
                    ),
                    self.tokens[idx].span.clone(),
                ));
                None
            }
        }
    }

    fn scan_seed_group_symbols(
        &mut self,
        seed_toks: &[usize],
    ) -> (Option<Distribution>, Option<usize>, bool) {
        let mut distribution = None;
        let mut dist_idx: Option<usize> = None;
        let mut new = false;
        for &i in seed_toks {
            if self.tokens[i].token != Token::Symbol {
                continue;
            }
            let upper = self.tokens[i].text.to_uppercase();
            let dist = match upper.as_str() {
                "NORMAL" => Distribution::Normal,
                "UNIFORM" => Distribution::Uniform,
                "NONPARAMETRIC" => Distribution::Nonparametric,
                "NEW" => {
                    new = true;
                    continue;
                }
                _ => continue,
            };
            if distribution.is_some() {
                self.push_error(Diagnostic::lowering(
                    format!("seed group already has a distribution; duplicate: {upper}"),
                    self.tokens[i].span.clone(),
                ));
                continue;
            }
            distribution = Some(dist);
            dist_idx = Some(i);
        }
        (distribution, dist_idx, new)
    }

    fn lower_simulation(&mut self, node: &CstNode, record_idx: usize) -> Simulation {
        let mut sim = Simulation {
            record_idx,
            ..Simulation::default()
        };
        let mut paren_indices: Vec<usize> = Vec::new();
        let mut omitted_idx: Option<usize> = None;

        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            let toks = self.non_trivia_children(n);
            if toks.is_empty() {
                continue;
            }

            match n.kind {
                NodeKind::Flag => {
                    let paren_pos = toks
                        .iter()
                        .position(|&i| self.tokens[i].token == Token::LeftParen);
                    if let Some(paren_pos) = paren_pos {
                        paren_indices.push(toks[paren_pos]);
                        let is_first = sim.seeds.is_empty();
                        sim.seeds.push(self.lower_seed_group(n, is_first));

                        if paren_pos > 0 {
                            let prefix: String = toks[..paren_pos]
                                .iter()
                                .map(|&i| self.tokens[i].text.as_str())
                                .collect::<String>()
                                .to_uppercase();
                            let prefix_idx = toks[0];
                            self.dispatch_simulation_flag(
                                &mut sim,
                                &prefix,
                                prefix_idx,
                                &mut omitted_idx,
                            );
                        }
                        continue;
                    }

                    let key_idx = toks[0];
                    let key = self.tokens[key_idx].text.to_uppercase();
                    self.dispatch_simulation_flag(&mut sim, &key, key_idx, &mut omitted_idx);
                }
                NodeKind::KeyValue => {
                    let key_idx = toks[0];
                    let val_idx = *toks.last().unwrap();
                    let key = self.tokens[key_idx].text.to_uppercase();
                    let value = self.token_value(val_idx);
                    let val_span = self.tokens[val_idx].span.clone();
                    self.dispatch_simulation_option(&mut sim, &key, key_idx, &value, val_span);
                }
                _ => {}
            }
        }

        if sim.seeds.len() > 10 {
            let span = self.tokens[paren_indices[10]].span.clone();
            let n = sim.seeds.len();
            self.push_error(Diagnostic::lowering(
                format!("$SIMULATION allows at most 10 random-number sources, found {n}"),
                span,
            ));
        }

        if sim.omitted {
            let has_other = !sim.seeds.is_empty()
                || sim.only_sim
                || sim.clockseed.is_some()
                || sim.subproblems.is_some()
                || sim.bootstrap.is_some()
                || sim.source_eps.is_some()
                || sim.ttdf.is_some()
                || sim.true_kind.is_some()
                || !sim.other_options.is_empty();
            if has_other {
                let span = omitted_idx
                    .map(|i| self.tokens[i].span.clone())
                    .unwrap_or_default();
                self.push_error(Diagnostic::lowering(
                    "OMITTED cannot be used with other $SIMULATION options",
                    span,
                ));
            }
        }

        if sim.seeds.is_empty() && !sim.omitted {
            let span = self
                .non_trivia_children(node)
                .first()
                .map(|&i| self.tokens[i].span.clone())
                .unwrap_or_default();
            self.push_error(Diagnostic::lowering(
                "$SIMULATION requires a random-number source: specify at least one (seed) group, e.g. $SIMULATION (1)",
                span,
            ));
        }

        sim
    }

    fn dispatch_simulation_flag(
        &mut self,
        sim: &mut Simulation,
        key: &str,
        key_idx: usize,
        omitted_idx: &mut Option<usize>,
    ) {
        match key {
            "ONLYSIM" | "ONLYSIMULATION" => {
                sim.only_sim = true;
            }
            "OMITTED" => {
                sim.omitted = true;
                if omitted_idx.is_none() {
                    *omitted_idx = Some(key_idx);
                }
            }
            "REQUESTFIRST" | "REQUESTSECOND" | "PREDICTION" | "NOPREDICTION" | "NOREWIND"
            | "REWIND" | "SUPRESET" | "NOSUPRESET" | "REPLACE" | "NOREPLACE" => {
                sim.other_options.push((key.to_string(), None));
            }
            "CLOCKSEED" => {
                self.push_error(Diagnostic::lowering(
                    "CLOCKSEED requires a value of 0 or 1",
                    self.tokens[key_idx].span.clone(),
                ));
            }
            "SUBPROBLEMS" | "SUBPROBS" | "BOOTSTRAP" | "SOURCE_EPS" | "TTDF" | "TRUE" | "STRAT"
            | "STRATF" | "RANMETHOD" | "PARAFILE" => {
                self.push_error(Diagnostic::lowering(
                    format!("$SIMULATION option {key} requires a value"),
                    self.tokens[key_idx].span.clone(),
                ));
            }
            _ => {
                self.push_error(Diagnostic::lowering(
                    format!("unknown $SIMULATION option: {key}"),
                    self.tokens[key_idx].span.clone(),
                ));
            }
        }
    }

    fn dispatch_simulation_option(
        &mut self,
        sim: &mut Simulation,
        key: &str,
        key_idx: usize,
        value: &str,
        val_span: std::ops::Range<usize>,
    ) {
        match key {
            "ONLYSIM" | "ONLYSIMULATION" | "OMITTED" | "REQUESTFIRST" | "REQUESTSECOND"
            | "PREDICTION" | "NOPREDICTION" | "NOREWIND" | "REWIND" | "SUPRESET" | "NOSUPRESET"
            | "REPLACE" | "NOREPLACE" => {
                self.push_error(Diagnostic::lowering(
                    format!("$SIMULATION option {key} does not take a value"),
                    val_span,
                ));
            }
            "CLOCKSEED" => match value {
                "0" => sim.clockseed = Some(false),
                "1" => sim.clockseed = Some(true),
                _ => self.push_error(Diagnostic::lowering(
                    format!("CLOCKSEED must be 0 or 1, found '{value}'"),
                    val_span,
                )),
            },
            "SUBPROBLEMS" | "SUBPROBS" => {
                self.parse_i32_option(&mut sim.subproblems, key, value, val_span);
            }
            "BOOTSTRAP" => {
                self.parse_i32_option(&mut sim.bootstrap, key, value, val_span);
            }
            "SOURCE_EPS" => {
                self.parse_i32_option(&mut sim.source_eps, key, value, val_span);
            }
            "TTDF" => {
                self.parse_i32_option(&mut sim.ttdf, key, value, val_span);
            }
            "TRUE" => match TrueKind::from_str(value) {
                Ok(k) => sim.true_kind = Some(k),
                Err(()) => self.push_error(Diagnostic::lowering(
                    format!("TRUE must be INITIAL, FINAL, or PRIOR, found '{value}'"),
                    val_span,
                )),
            },
            "STRAT" | "STRATF" | "RANMETHOD" | "PARAFILE" => {
                sim.other_options
                    .push((key.to_string(), Some(value.to_string())));
            }
            _ => self.push_error(Diagnostic::lowering(
                format!("unknown $SIMULATION option: {key}"),
                self.tokens[key_idx].span.clone(),
            )),
        }
    }

    fn parse_i32_option(
        &mut self,
        field: &mut Option<i32>,
        key: &str,
        value: &str,
        val_span: std::ops::Range<usize>,
    ) {
        match value.parse::<i32>() {
            Ok(n) => *field = Some(n),
            Err(_) => self.push_error(Diagnostic::lowering(
                format!("{key} requires an integer value, found '{value}'"),
                val_span,
            )),
        }
    }

    fn lower_covariance(&self, node: &CstNode, record_idx: usize) -> Covariance {
        let options = self.collect_options(node);
        Covariance {
            options,
            record_idx,
        }
    }

    fn lower_msfi(&self, node: &CstNode, record_idx: usize) -> Msfi {
        let options = self.collect_options(node);
        Msfi {
            options,
            record_idx,
        }
    }

    fn lower_subroutines(&self, node: &CstNode, record_idx: usize) -> Subroutines {
        let mut entries = Vec::new();

        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::Flag => {
                    let toks = self.non_trivia_children(n);
                    entries.push(Subroutine::Builtin {
                        name: self.tokens[toks[0]].text.clone(),
                        tolerance: None,
                    });
                }
                NodeKind::KeyValue => {
                    let toks = self.non_trivia_children(n);
                    let key = self.tokens[toks[0]].text.to_uppercase();
                    let val_idx = *toks.last().unwrap();

                    match key.as_str() {
                        "OTHER" => {
                            entries.push(Subroutine::Other {
                                path: self.token_value(val_idx),
                                path_idx: val_idx,
                            });
                        }
                        "TOL" => {
                            // Attach tolerance to the most recent Builtin entry
                            if let Some(Subroutine::Builtin { tolerance, .. }) = entries
                                .iter_mut()
                                .rev()
                                .find(|e| matches!(e, Subroutine::Builtin { .. }))
                            {
                                *tolerance = self.token_value(val_idx).parse().ok();
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Subroutines {
            entries,
            record_idx,
        }
    }

    fn lower_abbreviated(&self, node: &CstNode, record_idx: usize) -> Abbreviated {
        let mut replaces = Vec::new();
        let mut declares = Vec::new();

        for child in &node.children {
            let CstChild::Node(n) = child else { continue };
            if n.kind == NodeKind::Declare {
                let toks = self.non_trivia_children(n);
                // toks: [DECLARE, ...rest...]
                if toks.len() > 1 {
                    let full = n.text(self.tokens);
                    // Skip the keyword (DECLARE) and trim leading whitespace
                    let keyword_len = self.tokens[toks[0]].text.len();
                    let text = full[keyword_len..].trim().to_string();
                    declares.push(text);
                }
            } else if n.kind == NodeKind::Replace {
                let toks = self.non_trivia_children(n);
                // toks: [REPLACE, ...from_tokens..., =, ...to_tokens...]
                // Find the = separator
                if let Some(eq_pos) = toks
                    .iter()
                    .position(|&i| self.tokens[i].token == Token::Equals)
                {
                    let from: String = toks[1..eq_pos]
                        .iter()
                        .map(|&i| self.tokens[i].text.as_str())
                        .collect::<Vec<_>>()
                        .join("");
                    let to: String = toks[eq_pos + 1..]
                        .iter()
                        .map(|&i| self.tokens[i].text.as_str())
                        .collect::<Vec<_>>()
                        .join("");
                    if !from.is_empty() && !to.is_empty() {
                        replaces.push(Replace { from, to });
                    }
                }
            }
        }

        let options = self.collect_options(node);

        Abbreviated {
            replaces,
            declares,
            options,
            record_idx,
        }
    }

    fn lower_code_block(&mut self, node: &CstNode, record_idx: usize) -> Option<CodeBlock> {
        let cb = node.children.iter().find_map(|c| match c {
            CstChild::CodeBlock(cb) => Some(cb),
            _ => None,
        });
        match cb {
            Some(cb) => {
                let statements = crate::nmtran::lower::lower_stmts(&cb.children, &cb.tokens);
                Some(CodeBlock {
                    statements,
                    record_idx,
                })
            }
            None => {
                let span = self
                    .non_trivia_children(node)
                    .first()
                    .map(|&i| self.tokens[i].span.clone())
                    .unwrap_or_default();
                self.push_error(Diagnostic::lowering("missing code block", span));
                None
            }
        }
    }

    pub(crate) fn lower(mut self, cst: &CstNode) -> (Model, Vec<Diagnostic>) {
        let mut model = Model::default();

        for (record_idx, child) in cst.children.iter().enumerate() {
            if let CstChild::Node(node) = child {
                match node.kind {
                    NodeKind::Problem => {
                        model.problem = Problem {
                            text: self.lower_problem(node),
                            record_idx,
                        };
                    }
                    NodeKind::Input => {
                        model.input_columns.extend(self.lower_input(node));
                    }
                    NodeKind::Data => {
                        model.data = self.lower_data(node);
                    }
                    NodeKind::Theta => {
                        model.thetas.extend(self.lower_theta(node, record_idx));
                    }
                    NodeKind::Omega => {
                        model
                            .omega_blocks
                            .extend(self.lower_omega_sigma(node, record_idx));
                    }
                    NodeKind::Sigma => {
                        model
                            .sigma_blocks
                            .extend(self.lower_omega_sigma(node, record_idx));
                    }
                    NodeKind::Estimation => {
                        model
                            .estimations
                            .push(self.lower_estimation(node, record_idx));
                    }
                    NodeKind::Table => {
                        model.tables.push(self.lower_table(node, record_idx));
                    }
                    NodeKind::Simulation => {
                        model.simulation = Some(self.lower_simulation(node, record_idx));
                    }
                    NodeKind::Covariance => {
                        model.covariance = Some(self.lower_covariance(node, record_idx));
                    }
                    NodeKind::Msfi => {
                        model.msfi = Some(self.lower_msfi(node, record_idx));
                    }
                    NodeKind::Subroutines => {
                        model.subroutines = Some(self.lower_subroutines(node, record_idx));
                    }
                    NodeKind::Abbreviated => {
                        let new = self.lower_abbreviated(node, record_idx);
                        match &mut model.abbreviated {
                            Some(existing) => {
                                existing.replaces.extend(new.replaces);
                                existing.declares.extend(new.declares);
                                existing.options.extend(new.options);
                            }
                            None => {
                                model.abbreviated = Some(new);
                            }
                        }
                    }
                    NodeKind::Pk => {
                        model.pk = self.lower_code_block(node, record_idx);
                    }
                    NodeKind::ErrorBlock => {
                        model.error = self.lower_code_block(node, record_idx);
                    }
                    NodeKind::Des => {
                        model.des = self.lower_code_block(node, record_idx);
                    }
                    NodeKind::Pred => {
                        model.pred = self.lower_code_block(node, record_idx);
                    }
                    _ => continue,
                }
            }
        }
        self.validate_block_same_refs(&model.omega_blocks, cst);
        self.validate_block_same_refs(&model.sigma_blocks, cst);
        self.validate_simulation_nonparametric_msfi(&model, cst);
        self.validate_onlysim_with_estimation(cst);
        self.validate_first_problem_seed1(cst);

        (model, self.errors)
    }

    fn validate_simulation_nonparametric_msfi(&mut self, model: &Model, cst: &CstNode) {
        let Some(sim) = &model.simulation else { return };
        let needs_msfi = sim
            .seeds
            .iter()
            .any(|s| s.distribution == Some(Distribution::Nonparametric));
        if !needs_msfi || model.msfi.is_some() {
            return;
        }

        let span = if let CstChild::Node(sim_node) = &cst.children[sim.record_idx] {
            sim_node
                .children
                .iter()
                .find_map(|c| {
                    let CstChild::Node(n) = c else { return None };
                    if n.kind != NodeKind::Flag {
                        return None;
                    }
                    self.non_trivia_children(n).iter().find_map(|&i| {
                        (self.tokens[i].token == Token::Symbol
                            && self.tokens[i].text.eq_ignore_ascii_case("NONPARAMETRIC"))
                        .then(|| self.tokens[i].span.clone())
                    })
                })
                .unwrap_or_default()
        } else {
            Default::default()
        };

        self.push_error(Diagnostic::lowering(
            "NONPARAMETRIC distribution requires a $MSFI record",
            span,
        ));
    }

    // Walks the CST partitioned by $PROBLEM because `model.simulation` is a
    // single `Option<Simulation>` — it retains only the last $SIM per model,
    // so iterating the lowered model would miss ONLYSIM on any earlier $SIM.
    fn validate_onlysim_with_estimation(&mut self, cst: &CstNode) {
        let mut onlysim_span: Option<std::ops::Range<usize>> = None;
        let mut est_span: Option<std::ops::Range<usize>> = None;
        let mut conflicts: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> = Vec::new();

        for child in &cst.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::Problem => {
                    if let (Some(os), Some(es)) = (onlysim_span.take(), est_span.take()) {
                        conflicts.push((os, es));
                    }
                }
                NodeKind::Simulation => {
                    if onlysim_span.is_none() {
                        onlysim_span = self.find_onlysim_span(n);
                    }
                }
                NodeKind::Estimation => {
                    if est_span.is_none() {
                        est_span = self
                            .non_trivia_children(n)
                            .first()
                            .map(|&i| self.tokens[i].span.clone());
                    }
                }
                _ => {}
            }
        }
        if let (Some(os), Some(es)) = (onlysim_span, est_span) {
            conflicts.push((os, es));
        }

        for (os, es) in conflicts {
            self.push_error(
                Diagnostic::lowering("$SIMULATION ONLYSIM is incompatible with $ESTIMATION", os)
                    .with_note("$ESTIMATION record here", es),
            );
        }
    }

    fn find_onlysim_span(&self, sim_node: &CstNode) -> Option<std::ops::Range<usize>> {
        for child in &sim_node.children {
            let CstChild::Node(n) = child else { continue };
            if n.kind != NodeKind::Flag {
                continue;
            }
            for &i in self.non_trivia_children(n).iter() {
                if self.tokens[i].token == Token::Symbol {
                    let upper = self.tokens[i].text.to_uppercase();
                    if upper == "ONLYSIM" || upper == "ONLYSIMULATION" {
                        return Some(self.tokens[i].span.clone());
                    }
                }
            }
        }
        None
    }

    // Walks the CST directly so each $SIM record is visible: `model.simulation`
    // is a single Option that only retains the last $SIM, and the seed1=-1 rule
    // depends on which $PROBLEM the record sits under, not just the final one.
    fn validate_first_problem_seed1(&mut self, cst: &CstNode) {
        let mut problem_count = 0;
        for child in &cst.children {
            let CstChild::Node(n) = child else { continue };
            match n.kind {
                NodeKind::Problem => problem_count += 1,
                NodeKind::Simulation if problem_count <= 1 => {
                    self.flag_first_problem_neg_one_seeds(n);
                }
                _ => {}
            }
        }
    }

    fn flag_first_problem_neg_one_seeds(&mut self, sim_node: &CstNode) {
        for child in &sim_node.children {
            let CstChild::Node(n) = child else { continue };
            if n.kind != NodeKind::Flag {
                continue;
            }
            let toks = self.non_trivia_children(n);
            let Some(paren_pos) = toks
                .iter()
                .position(|&i| self.tokens[i].token == Token::LeftParen)
            else {
                continue;
            };
            for &i in &toks[paren_pos + 1..] {
                if self.tokens[i].token == Token::Int {
                    if self.tokens[i].text == "-1" {
                        self.push_error(Diagnostic::lowering(
                            "seed1 may not be -1 on the first $PROBLEM",
                            self.tokens[i].span.clone(),
                        ));
                    }
                    break;
                }
            }
        }
    }

    fn validate_block_same_refs(&mut self, blocks: &[OmegaSigmaBlock], cst: &CstNode) {
        for (i, block) in blocks.iter().enumerate() {
            let BlockStructure::BlockSame { size, .. } = &block.structure else {
                continue;
            };
            let valid = i > 0
                && match &blocks[i - 1].structure {
                    BlockStructure::Block { size: s }
                    | BlockStructure::BlockSame { size: s, .. } => s == size,
                    _ => false,
                };
            if !valid {
                let span = if let CstChild::Node(node) = &cst.children[block.record_idx] {
                    self.find_first_child(node, NodeKind::Same)
                        .and_then(|same| self.non_trivia_children(same).first().copied())
                        .map(|idx| self.tokens[idx].span.clone())
                        .unwrap_or_default()
                } else {
                    0..0
                };
                let prev_span = if i > 0 {
                    if let CstChild::Node(node) = &cst.children[blocks[i - 1].record_idx] {
                        self.non_trivia_children(node)
                            .first()
                            .copied()
                            .map(|idx| self.tokens[idx].span.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                let same_span = span.clone();
                let mut diag = Diagnostic::lowering(
                    format!("SAME must immediately follow a BLOCK({size}) record"),
                    span,
                );
                if let Some(prev_span) = prev_span {
                    diag = diag.with_note(
                        format!("the preceding record is not a BLOCK({size})"),
                        prev_span,
                    );
                } else {
                    diag = diag.with_note(
                        format!("no BLOCK({size}) record precedes this SAME"),
                        same_span,
                    );
                }
                self.push_error(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BlockStructure, DiagonalScale, OffDiagonalScale, Parametrization};
    use crate::model::parameters::ParameterOrdering;
    use crate::parser::Parser;
    use insta::{assert_snapshot, glob};

    #[test]
    fn multiple_input_records_are_merged() {
        let src = "$PROB test\n$INPUT ID TIME\n$INPUT DV AMT\n$DATA foo.csv\n";
        let parser = Parser::new(src);
        let (cst, tokens, _source) = parser.parse().unwrap();
        let lowerer = Lowerer::new(tokens.as_slice());
        let (model, diagnostics) = lowerer.lower(&cst);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        let names: Vec<_> = model
            .input_columns
            .iter()
            .map(|c| format!("{:?}", c.kind))
            .collect();
        assert_eq!(
            names,
            vec![
                "Included(ID)",
                "Included(TIME)",
                "Included(DV)",
                "Included(AMT)"
            ]
        );
    }

    #[test]
    fn can_lower() {
        glob!("../test_data/", "*.mod", |path| {
            let input = fs_err::read_to_string(path).unwrap();
            let parser = Parser::new(&input);
            let (cst, tokens, _source) = parser.parse().unwrap();
            let lowerer = Lowerer::new(tokens.as_slice());
            let (model, diagnostics) = lowerer.lower(&cst);
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert_snapshot!(model.debug_ast());
        });
    }

    fn parse_ok(input: &str) -> crate::model::Model {
        crate::model::Model::parse(input).unwrap_or_else(|errs| {
            panic!("parse failed: {errs:?}");
        })
    }

    fn minimal(body: &str) -> String {
        format!("$PROBLEM test\n$INPUT ID\n$DATA data.csv\n{body}")
    }

    fn omega_blocks(body: &str) -> Vec<crate::ast::OmegaSigmaBlock> {
        parse_ok(&minimal(body)).omega_blocks
    }

    fn sigma_blocks(body: &str) -> Vec<crate::ast::OmegaSigmaBlock> {
        parse_ok(&minimal(body)).sigma_blocks
    }

    const SD: Option<Parametrization> = Some(Parametrization::Axes {
        diagonal: Some(DiagonalScale::StandardDeviation),
        off_diagonal: None,
    });
    const VAR: Option<Parametrization> = Some(Parametrization::Axes {
        diagonal: Some(DiagonalScale::Variance),
        off_diagonal: None,
    });
    const CORR: Option<Parametrization> = Some(Parametrization::Axes {
        diagonal: None,
        off_diagonal: Some(OffDiagonalScale::Correlation),
    });
    const COV: Option<Parametrization> = Some(Parametrization::Axes {
        diagonal: None,
        off_diagonal: Some(OffDiagonalScale::Covariance),
    });
    const SD_CORR: Option<Parametrization> = Some(Parametrization::Axes {
        diagonal: Some(DiagonalScale::StandardDeviation),
        off_diagonal: Some(OffDiagonalScale::Correlation),
    });
    const CHOL: Option<Parametrization> = Some(Parametrization::Cholesky);

    #[test]
    fn parametrization_block_flags() {
        // (input, expected_parametrization, expected_fixed, expected_param_count)
        let cases: Vec<(&str, Option<Parametrization>, bool, usize)> = vec![
            // case 13: plain block
            (
                "$OMEGA BLOCK(3)\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                None,
                false,
                6,
            ),
            // case 14: CHOLESKY
            (
                "$OMEGA BLOCK(3) CHOLESKY\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                CHOL,
                false,
                6,
            ),
            // case 15: SD CORR
            (
                "$OMEGA BLOCK(3) SD CORR\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                SD_CORR,
                false,
                6,
            ),
            // case 16: CORR SD (reversed, same result)
            (
                "$OMEGA BLOCK(3) CORR SD\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                SD_CORR,
                false,
                6,
            ),
            // case 17: inline flags accumulate
            (
                "$OMEGA BLOCK(3)\n0.1\n0.01\n0.1\n0.01\n0.01 SD\n0.1 CORR\n",
                SD_CORR,
                false,
                6,
            ),
            // case 18: mixed record-level + inline
            (
                "$OMEGA BLOCK(3) SD\n0.1\n0.01 0.1\n0.01 0.01 CORR 0.1\n",
                SD_CORR,
                false,
                6,
            ),
            // case 19: FIX on any value
            ("$OMEGA BLOCK(2)\n0.3\n0.01 FIX 0.35\n", None, true, 3),
            // case 20: record-level FIX
            ("$OMEGA BLOCK(2) FIX\n0.1\n0.01 0.1\n", None, true, 3),
            // case 22: COV explicit
            ("$OMEGA BLOCK(2) COV\n0.1\n0.01 0.1\n", COV, false, 3),
            // case 23: VAR explicit
            ("$OMEGA BLOCK(2) VAR\n0.1\n0.01 0.1\n", VAR, false, 3),
            // case 24: COVARIANCE long form
            ("$OMEGA BLOCK(2) COVARIANCE\n0.1\n0.01 0.1\n", COV, false, 3),
            // case 25: CORRELATION long form
            (
                "$OMEGA BLOCK(3) CORRELATION\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                CORR,
                false,
                6,
            ),
            // case 26: CHOLESKY on BLOCK(1)
            ("$OMEGA BLOCK(1) CHOLESKY\n0.04\n", CHOL, false, 1),
            // case 27: SD on BLOCK(1)
            ("$OMEGA BLOCK(1) SD\n0.2\n", SD, false, 1),
        ];

        for (input, expected_param, expected_fixed, expected_count) in cases {
            let blocks = omega_blocks(input);
            assert_eq!(blocks.len(), 1, "input: {input}");
            assert_eq!(
                blocks[0].parametrization, expected_param,
                "parametrization mismatch: {input}"
            );
            assert_eq!(blocks[0].fixed, expected_fixed, "fixed mismatch: {input}");
            assert_eq!(
                blocks[0].parameters.len(),
                expected_count,
                "param count mismatch: {input}"
            );
        }
    }

    #[test]
    fn parametrization_diagonal_no_split() {
        // (input, expected_param_count, expected_parametrization)
        let cases: Vec<(&str, usize, Option<Parametrization>)> = vec![
            // case 1: plain
            ("$OMEGA\n0.04\n0.09\n", 2, None),
            // case 6: VAR partial → None
            ("$OMEGA\n0.04\n0.05 VAR\n0.03\n", 3, None),
            // case 9: named VAR partial → None
            ("$OMEGA\n0.04\nEV=0.05 VAR\n0.03\n", 3, None),
            // case 10: VARIANCE long form partial → None
            ("$OMEGA\n0.04\n0.05 VARIANCE\n0.03\n", 3, None),
            // case 12: CHOLESKY partial → None
            ("$OMEGA\n0.04\n0.05 CHOLESKY\n0.03\n", 3, None),
            // all VAR uniform → stored
            ("$OMEGA\n0.04 VAR\n0.05 VAR\n", 2, VAR),
        ];

        for (input, expected_count, expected_param) in cases {
            let blocks = omega_blocks(input);
            assert_eq!(blocks.len(), 1, "expected 1 block: {input}");
            assert!(!blocks[0].fixed, "should not be fixed: {input}");
            assert_eq!(
                blocks[0].parameters.len(),
                expected_count,
                "param count: {input}"
            );
            assert_eq!(
                blocks[0].parametrization, expected_param,
                "parametrization: {input}"
            );
        }
    }

    #[test]
    fn parametrization_diagonal_split() {
        // (input, expected_block_count, per-block: (parametrization, fixed))
        let cases: Vec<(&str, Vec<(Option<Parametrization>, bool)>)> = vec![
            // case 2: inline SD
            (
                "$OMEGA\n0.04\n0.01 SD\n0.09\n",
                vec![(None, false), (SD, false), (None, false)],
            ),
            // case 3: inline FIX
            (
                "$OMEGA\n0.25 FIXED\n0.25\n(0.49 FIXED)\n",
                vec![(None, true), (None, false), (None, true)],
            ),
            // case 4: SD + FIX
            (
                "$OMEGA\n0.04\n0.01 SD FIX\n0.09\n",
                vec![(None, false), (SD, true), (None, false)],
            ),
            // case 5: repeat with SD
            (
                "$OMEGA\n(0.1 SD)x3\n",
                vec![(SD, false), (SD, false), (SD, false)],
            ),
            // case 7: named mixed
            (
                "$OMEGA\nECL=0.04 FIX\nEV=0.09\nEKA=0.16 SD\nEF=1\n",
                vec![(None, true), (None, false), (SD, false), (None, false)],
            ),
            // case 11: STANDARD long form
            (
                "$OMEGA\n0.04\n0.05 STANDARD\n0.03\n",
                vec![(None, false), (SD, false), (None, false)],
            ),
        ];

        for (input, expected) in cases {
            let blocks = omega_blocks(input);
            assert_eq!(blocks.len(), expected.len(), "block count: {input}");
            for (i, (exp_param, exp_fixed)) in expected.iter().enumerate() {
                assert_eq!(
                    blocks[i].parametrization, *exp_param,
                    "block[{i}] parametrization: {input}"
                );
                assert_eq!(blocks[i].fixed, *exp_fixed, "block[{i}] fixed: {input}");
                assert_eq!(
                    blocks[i].parameters.len(),
                    1,
                    "block[{i}] should have 1 param: {input}"
                );
            }
        }
    }

    #[test]
    fn parametrization_block_same() {
        let blocks = omega_blocks("$OMEGA BLOCK(2) CORR\n0.2\n0.3 0.15\n$OMEGA BLOCK(2) SAME\n");
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[1].structure,
            BlockStructure::BlockSame {
                size: 2,
                repeats: 1
            }
        );
        assert_eq!(blocks[1].parametrization, None);

        let blocks = omega_blocks("$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n$OMEGA BLOCK(2) SAME(3)\n");
        assert_eq!(
            blocks[1].structure,
            BlockStructure::BlockSame {
                size: 2,
                repeats: 3
            }
        );

        // Consecutive SAMEs — both refer back to the original Block
        let model = parse_ok(&minimal(
            "$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n$OMEGA BLOCK(2) SAME\n$OMEGA BLOCK(2) SAME\n",
        ));
        let params = model
            .get_omega_parameters(ParameterOrdering::RowMajor)
            .unwrap();
        assert_eq!(params.len(), 9); // 3 params × 3 blocks (original + 2 SAMEs), each BLOCK(2) = 3 params

        // SAME with intervening diagonal — rejected during lowering
        let input =
            minimal("$OMEGA BLOCK(2) SD CORR\n0.2\n0.3 0.15\n$OMEGA 0.04\n$OMEGA BLOCK(2) SAME\n");
        let errs = crate::model::Model::parse(&input)
            .expect_err("SAME with intervening diagonal should fail to parse");
        assert!(
            errs.iter()
                .any(|e| e.to_string().contains("SAME must immediately follow")),
            "unexpected errors: {errs:?}"
        );
    }

    #[test]
    fn parametrization_sigma() {
        let blocks = sigma_blocks("$SIGMA\n0.1\n2\n0.04 SD\n");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[2].parametrization, SD);

        let blocks = sigma_blocks("$SIGMA BLOCK(2) CORR\n0.1\n0.3 0.2\n");
        assert_eq!(blocks[0].parametrization, CORR);
    }

    #[test]
    fn parametrization_values_syntax() {
        let blocks = omega_blocks("$OMEGA BLOCK(4) NAMES(ECL,EV,EQ,EVP) VALUES(0.03,0.01)\n");
        assert_eq!(blocks[0].names, vec!["ECL", "EV", "EQ", "EVP"]);
        assert_eq!(blocks[0].parameters.len(), 10);
    }

    #[test]
    fn parametrization_rejection_cases() {
        let cases: Vec<(&str, &str)> = vec![
            // case 8: NAMES on diagonal
            ("$OMEGA NAMES(CL,V)\n0.04\n0.09\n", "NAMES requires BLOCK"),
            // case 33: record-level FIX on diagonal
            (
                "$OMEGA FIX\n0.04\n0.01\n",
                "must appear inline after a value",
            ),
            // case 34: record-level SD on diagonal
            (
                "$OMEGA SD\n0.04\n0.01\n",
                "must appear inline after a value",
            ),
            // case 35: CHOLESKY + SD conflict
            (
                "$OMEGA BLOCK(3) CHOLESKY SD\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                "mutually exclusive",
            ),
            // case 36: SD + VAR duplicate
            (
                "$OMEGA BLOCK(3) SD VAR\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                "duplicate diagonal axis flag",
            ),
            // case 37: CORR + COV duplicate
            (
                "$OMEGA BLOCK(3) CORR COV\n0.1\n0.01 0.1\n0.01 0.01 0.1\n",
                "duplicate off-diagonal axis flag",
            ),
            // diagonal values should also reject conflicting inline parametrization flags
            ("$OMEGA 0.1 SD VAR\n", "conflicting parametrization flag"),
            (
                "$OMEGA 0.1 CHOLESKY SD\n",
                "conflicting parametrization flag",
            ),
            // case 38: flag in parens in BLOCK
            (
                "$OMEGA BLOCK(3)\n0.01\n(0.02 SD)x2\n(0.03)x3\n",
                "parametrization flags inside parentheses",
            ),
            // case 39: SAME with parametrization
            (
                "$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n$OMEGA BLOCK(2) SAME SD\n",
                "not allowed on a SAME block",
            ),
            // case 40: SAME with FIX
            (
                "$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n$OMEGA BLOCK(2) SAME FIX\n",
                "not allowed on a SAME block",
            ),
            // case 41: SAME without BLOCK
            ("$OMEGA SAME\n", "SAME requires an explicit BLOCK"),
            // case 42: SAME(m) without BLOCK
            ("$OMEGA SAME(3)\n", "SAME requires an explicit BLOCK"),
            // case 43: named param missing value
            ("$OMEGA\nECL=\nEV=0.09\n", "expected a number after '='"),
            // case 44: CORR on diagonal
            ("$OMEGA\n0.04 CORR\n", "off-diagonal flag"),
            // case 45: COV on diagonal
            ("$OMEGA\n0.04 COV\n", "off-diagonal flag"),
            // case 46: CORR on BLOCK(1)
            (
                "$OMEGA BLOCK(1) CORR\n0.04\n",
                "BLOCK(1) has no off-diagonal elements",
            ),
            // case 47: COV on BLOCK(1)
            (
                "$OMEGA BLOCK(1) COV\n0.04\n",
                "BLOCK(1) has no off-diagonal elements",
            ),
            // SAME with values
            (
                "$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n$OMEGA BLOCK(2) SAME\n0.1 SD\n",
                "SAME block cannot contain parameter values",
            ),
        ];

        for (input, expected_msg) in cases {
            let full = minimal(input);
            let errs = crate::model::Model::parse(&full)
                .expect_err(&format!("expected error for: {input}"));
            let found = errs.iter().any(|e| e.to_string().contains(expected_msg));
            assert!(
                found,
                "expected error containing '{expected_msg}' for input: {input}, got: {errs:?}"
            );
        }
    }
}
