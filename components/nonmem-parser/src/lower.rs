use crate::ast::{
    Abbreviated, BlockStructure, CodeBlock, ComparisonOperator, Covariance, Data, DataFilter,
    DataValueFilter, DataValueFilterKind, Estimation, EstimationMethod, InputColumn,
    InputColumnKind, OmegaSigmaBlock, OmegaSigmaParam, Parametrization, Problem, Replace,
    Simulation, Subroutine, Subroutines, Table, ThetaParameter,
};
use crate::cst::{CstChild, CstNode, NodeKind};
use crate::errors::Diagnostic;
use crate::lexer::{SpannedToken, Token};
use crate::model::Model;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

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
                    let text = self.tokens[toks[0]].text.to_uppercase();
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
        if dot_parts.len() == 3 && !dot_parts[0].is_empty() {
            if let Ok(op) = dot_parts[1].to_uppercase().parse::<ComparisonOperator>() {
                let value = Self::parse_value(dot_parts[2]);
                return Some(DataFilter::ValueFilter(DataValueFilter {
                    field: dot_parts[0].to_string(),
                    op,
                    value,
                }));
            }
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
            if let Some(pos) = joined.find(sym) {
                if pos > 0 && pos + sym.len() < joined.len() {
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

    fn lower_omega_sigma(&mut self, node: &CstNode, record_idx: usize) -> OmegaSigmaBlock {
        // 1. Structure: Block(n), BlockSame(n)[xrepeats], or Diagonal
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
        } else if let Some(repeats) = same_repeats {
            BlockStructure::BlockSame { size: 1, repeats }
        } else {
            BlockStructure::Diagonal
        };

        // 2. Record-level flags: FIX, parametrization
        let mut fixed = false;
        let mut parametrization: Option<Parametrization> = None;
        for flag_node in self.find_all_children(node, NodeKind::Flag) {
            for &idx in &self.non_trivia_children(flag_node) {
                let text = self.tokens[idx].text.to_uppercase();
                match text.as_str() {
                    "FIX" | "FIXED" => fixed = true,
                    "CORR" | "CORRELATION" => {
                        parametrization = Some(Parametrization::Correlation);
                    }
                    "SD" | "STANDARD" => {
                        parametrization = Some(Parametrization::StandardDeviation);
                    }
                    "CHOLESKY" => {
                        parametrization = Some(Parametrization::Cholesky);
                    }
                    _ => {}
                }
            }
        }

        // 3. A NAMES field
        let names: Vec<String> = self
            .find_first_child(node, NodeKind::ParamNames)
            .map(|n| self.extract_names(n))
            .unwrap_or_default();

        // 4. A VALUES field
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

        // 5. Parameters
        let is_same = matches!(structure, BlockStructure::BlockSame { .. });
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
                    if is_same {
                        let span = self
                            .non_trivia_children(param)
                            .first()
                            .map(|&i| self.tokens[i].span.clone())
                            .unwrap_or_default();
                        self.push_error(Diagnostic::lowering(
                            "SAME block cannot contain parameter values",
                            span,
                        ));
                        continue;
                    }

                    let non_trivia = self.non_trivia_children(param);

                    // Named form: Symbol = Number...
                    let is_named = non_trivia
                        .first()
                        .map(|&i| self.tokens[i].token == Token::Symbol)
                        .unwrap_or(false)
                        && non_trivia
                            .get(1)
                            .map(|&i| self.tokens[i].token == Token::Equals)
                            .unwrap_or(false);
                    let name = if is_named {
                        Some(self.tokens[non_trivia[0]].text.clone())
                    } else {
                        None
                    };

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

                    let has_paren = non_trivia
                        .iter()
                        .any(|&i| self.tokens[i].token == Token::LeftParen);

                    if has_paren && nums.len() == 1 {
                        // (value) xN
                        let repeat = self.find_repeat_number(param).unwrap_or(1);
                        let value = self.parse_number(nums[0]);
                        let fix = self.has_fix(param);
                        for _ in 0..repeat {
                            parameters.push(OmegaSigmaParam {
                                value,
                                fixed: fix,
                                name: name.clone(),
                                comment: None,
                                param_child_idx: child_idx,
                                value_idx: nums[0],
                            });
                        }
                    } else {
                        // Bare number(s) — possibly multiple in a label row
                        let fix_all = self.has_fix(param);
                        for (i, &num_idx) in nums.iter().enumerate() {
                            // Per-value FIX: check if a Flag(FIX) appears between
                            // this number and the next in the Param's children
                            let mut per_fix = false;
                            let mut past = false;
                            let next = nums.get(i + 1).copied();
                            for c in &param.children {
                                match c {
                                    CstChild::Token(idx) if *idx == num_idx => past = true,
                                    CstChild::Token(idx) if next == Some(*idx) => break,
                                    CstChild::Node(n) if n.kind == NodeKind::Flag && past => {
                                        per_fix = self.non_trivia_children(n).iter().any(|&j| {
                                            let t = &self.tokens[j].text;
                                            t.eq_ignore_ascii_case("FIX")
                                                || t.eq_ignore_ascii_case("FIXED")
                                        });
                                        if per_fix {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            parameters.push(OmegaSigmaParam {
                                value: self.parse_number(num_idx),
                                fixed: per_fix || fix_all,
                                name: if i == 0 { name.clone() } else { None },
                                comment: None,
                                param_child_idx: child_idx,
                                value_idx: num_idx,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // 6. Expand VALUES(diag, odiag) into full lower-triangle for BLOCK(n)
        if !values_nums.is_empty()
            && parameters.is_empty()
            && let BlockStructure::Block { size } = structure
            && values_nums.len() == 2
        {
            let (diag, odiag) = (values_nums[0], values_nums[1]);
            for row in 0..size {
                for col in 0..=row {
                    parameters.push(OmegaSigmaParam {
                        value: if row == col { diag } else { odiag },
                        fixed: false,
                        name: None,
                        comment: None,
                        param_child_idx: 0,
                        value_idx: 0,
                    });
                }
            }
        }
        OmegaSigmaBlock {
            structure,
            parametrization,
            fixed,
            names,
            parameters,
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
                    let text = self.tokens[toks[0]].text.to_uppercase();
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

    fn lower_simulation(&self, node: &CstNode, record_idx: usize) -> Simulation {
        let options = self.collect_options(node);
        Simulation {
            options,
            record_idx,
        }
    }

    fn lower_covariance(&self, node: &CstNode, record_idx: usize) -> Covariance {
        let options = self.collect_options(node);
        Covariance {
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
                        model.input_columns = self.lower_input(node);
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
                            .push(self.lower_omega_sigma(node, record_idx));
                    }
                    NodeKind::Sigma => {
                        model
                            .sigma_blocks
                            .push(self.lower_omega_sigma(node, record_idx));
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
        (model, self.errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use insta::{assert_snapshot, glob};

    #[test]
    fn can_lower() {
        glob!("../test_data/", "*.mod", |path| {
            let input = fs_err::read_to_string(path).unwrap();
            let parser = Parser::new(&input);
            let (cst, tokens) = parser.parse().unwrap();
            let lowerer = Lowerer::new(tokens.as_slice());
            let (model, diagnostics) = lowerer.lower(&cst);
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert_snapshot!(model.debug_ast());
        });
    }
}
