//! Non-mutating rewrites of individual `$THETA` value specs.
//!
//! The write path mirrors `update_problem_statement`'s tombstone pattern: the
//! new spec text lands on the first value token of the Param node and every
//! other token in the node is blanked, so token indices stay stable and
//! everything outside the rewritten spec renders byte-identically.
//! Comments and surrounding whitespace live in the parent record node, not in
//! the Param node, so `; NAME cov` annotations survive the rewrite.

use std::collections::{BTreeMap, HashMap};

use anyhow::{Result as AnyhowResult, bail};

use crate::cst::{CstChild, CstNode, NodeKind};
use crate::lexer::Token;
use crate::model::Model;

/// Collect every token index inside `node` in document order, recursing
/// through nested nodes (Parens, Flag, Repeat, ...).
fn collect_token_indices(node: &CstNode, out: &mut Vec<usize>) {
    for child in &node.children {
        match child {
            CstChild::Token(idx) => out.push(*idx),
            CstChild::Node(n) => collect_token_indices(n, out),
            // Code blocks never appear inside a Param node
            CstChild::CodeBlock(_) => {}
        }
    }
}

fn node_contains_repeat(node: &CstNode) -> bool {
    node.children.iter().any(|c| match c {
        CstChild::Node(n) => n.kind == NodeKind::Repeat || node_contains_repeat(n),
        _ => false,
    })
}

impl Model {
    /// Locate the Param CST node for the 0-based theta `index`.
    fn theta_param_node(&self, index: usize) -> AnyhowResult<&CstNode> {
        let Some(theta) = self.thetas.get(index) else {
            bail!(
                "theta index {} out of range: model has {} thetas",
                index + 1,
                self.thetas.len()
            );
        };

        let Some(CstChild::Node(record)) = self.cst.children.get(theta.record_idx) else {
            bail!("internal error: theta record index does not point at a record node");
        };
        let Some(CstChild::Node(param)) = record.children.get(theta.param_child_idx) else {
            bail!("internal error: theta param index does not point at a Param node");
        };
        if param.kind != NodeKind::Param {
            bail!(
                "internal error: expected a Param node for THETA{}",
                index + 1
            );
        }
        Ok(param)
    }

    /// Build a token-replacement map that rewrites the value specs of the
    /// given thetas. Keys of `specs` are 0-based theta indices; values are the
    /// replacement spec text, e.g. `"0.1"` or `"0 FIX"` or `"(0, 0.1, 5)"`.
    ///
    /// The rewrite covers the whole value spec — bounds, init, and any FIX
    /// flag — but preserves a `NAME=` label prefix on named thetas. Trailing
    /// comments are untouched (they belong to the record, not the param).
    ///
    /// Errors if an index is out of range or its theta was produced by an
    /// `(value) xN` repeat (rewriting one would rewrite them all).
    pub fn theta_spec_replacements(
        &self,
        specs: &BTreeMap<usize, String>,
    ) -> AnyhowResult<HashMap<usize, String>> {
        let mut replacements: HashMap<usize, String> = HashMap::new();

        for (&index, spec) in specs {
            let param = self.theta_param_node(index)?;

            if node_contains_repeat(param) {
                bail!(
                    "THETA{} uses an xN repeat spec; rewriting it would change every repeated theta",
                    index + 1
                );
            }

            let mut token_indices = Vec::new();
            collect_token_indices(param, &mut token_indices);

            // Preserve a `NAME =` prefix on named thetas: start the rewrite at
            // the first token that opens the value spec.
            let start = token_indices
                .iter()
                .position(|&i| {
                    matches!(
                        self.tokens[i].token,
                        Token::LeftParen | Token::Int | Token::Float | Token::Infinity
                    )
                })
                .unwrap_or(0);

            let value_tokens = &token_indices[start..];
            let Some(&first) = value_tokens.first() else {
                bail!("internal error: THETA{} has no value tokens", index + 1);
            };

            replacements.insert(first, spec.clone());
            for &idx in &value_tokens[1..] {
                replacements.insert(idx, String::new());
            }
        }

        Ok(replacements)
    }

    /// Token replacements that blank out the `$COVARIANCE` record entirely.
    /// Empty map if the model has no `$COVARIANCE`.
    pub fn covariance_removal_replacements(&self) -> HashMap<usize, String> {
        let mut replacements = HashMap::new();
        if let Some(cov) = &self.covariance
            && let Some(CstChild::Node(record)) = self.cst.children.get(cov.record_idx)
        {
            let mut token_indices = Vec::new();
            collect_token_indices(record, &mut token_indices);
            for idx in token_indices {
                replacements.insert(idx, String::new());
            }
        }
        replacements
    }

    /// Render the model content with an arbitrary token replacement map, for
    /// combining maps produced by the helpers above.
    pub fn render_with_replacements(&self, replacements: &HashMap<usize, String>) -> String {
        self.cst.text_with_replacements(&self.tokens, replacements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_model(input: &str) -> Model {
        Model::inner_parse(input).unwrap()
    }

    fn render_theta_specs(model: &Model, specs: &BTreeMap<usize, String>) -> AnyhowResult<String> {
        let replacements = model.theta_spec_replacements(specs)?;
        Ok(model.render_with_replacements(&replacements))
    }

    const SCM_TEMPLATE: &str = "\
$PROBLEM scm template
$INPUT ID TIME AMT DV WT CRCL
$DATA data.csv IGNORE=@
$SUBROUTINES ADVAN2 TRANS2
$PK
TVCL = THETA(1) * (WT/70)**THETA(4)
CL = TVCL * EXP(ETA(1))
V  = THETA(2) * EXP(ETA(2))
KA = THETA(3)
$ERROR
Y = F * (1 + EPS(1))
$THETA (0, 3)      ; TVCL (L/h)
$THETA (0, 20)     ; TVV (L)
$THETA (0, 1.2)    ; TVKA (1/h)
$THETA (0 FIX)     ; WT_CL cov
$THETA 0 FIX       ; CRCL_CL cov
$OMEGA 0.1
$OMEGA 0.1
$SIGMA 0.02
$ESTIMATION METHOD=1 INTER MAXEVAL=9999
$COVARIANCE
";

    #[test]
    fn release_paren_fixed_theta_preserves_everything_else() {
        let model = parse_model(SCM_TEMPLATE);
        let specs = BTreeMap::from([(3usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();

        assert!(out.contains("$THETA 0.1     ; WT_CL cov"), "got:\n{out}");
        // Only the rewritten line may differ.
        for (orig, new) in SCM_TEMPLATE.lines().zip(out.lines()) {
            if orig.contains("WT_CL") {
                continue;
            }
            assert_eq!(orig, new);
        }
        assert_eq!(SCM_TEMPLATE.lines().count(), out.lines().count());
    }

    #[test]
    fn release_bare_fixed_theta() {
        let model = parse_model(SCM_TEMPLATE);
        let specs = BTreeMap::from([(4usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();
        assert!(
            out.contains("$THETA 0.1       ; CRCL_CL cov"),
            "got:\n{out}"
        );
    }

    #[test]
    fn release_multiple_and_reparse() {
        let model = parse_model(SCM_TEMPLATE);
        let specs = BTreeMap::from([(3usize, "0.1".to_string()), (4usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();

        let reparsed = Model::inner_parse(&out).unwrap();
        assert_eq!(reparsed.thetas.len(), 5);
        assert!((reparsed.thetas[3].init - 0.1).abs() < 1e-12);
        assert!(!reparsed.thetas[3].fixed);
        assert!((reparsed.thetas[4].init - 0.1).abs() < 1e-12);
        assert!(!reparsed.thetas[4].fixed);
        // Untouched thetas keep their spec
        assert!((reparsed.thetas[0].init - 3.0).abs() < 1e-12);
        assert_eq!(reparsed.thetas[0].lower, Some(0.0));
    }

    #[test]
    fn refix_a_released_theta() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA 0.1 ; WT_CL cov
";
        let model = parse_model(input);
        let specs = BTreeMap::from([(0usize, "0 FIX".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();
        assert!(out.contains("$THETA 0 FIX ; WT_CL cov"), "got:\n{out}");

        let reparsed = Model::inner_parse(&out).unwrap();
        assert!(reparsed.thetas[0].fixed);
        assert_eq!(reparsed.thetas[0].init, 0.0);
    }

    #[test]
    fn rewrite_bounded_theta_replaces_whole_spec() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA (0, 1.5, 10) FIX ; TVCL
";
        let model = parse_model(input);
        let specs = BTreeMap::from([(0usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();
        assert!(out.contains("$THETA 0.1 ; TVCL"), "got:\n{out}");
    }

    #[test]
    fn named_theta_keeps_its_label() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA CL=(0, 1.5, 10)
";
        let model = parse_model(input);
        let specs = BTreeMap::from([(0usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();
        assert!(out.contains("$THETA CL=0.1"), "got:\n{out}");
    }

    #[test]
    fn multiple_thetas_on_one_line() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA (0, 3) (0 FIX) (0 FIX) ; shared comment
";
        let model = parse_model(input);
        let specs = BTreeMap::from([(1usize, "0.1".to_string())]);
        let out = render_theta_specs(&model, &specs).unwrap();
        assert!(
            out.contains("$THETA (0, 3) 0.1 (0 FIX) ; shared comment"),
            "got:\n{out}"
        );
    }

    #[test]
    fn repeat_spec_is_rejected() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA (0.1) x3
";
        let model = parse_model(input);
        assert_eq!(model.thetas.len(), 3);
        let specs = BTreeMap::from([(1usize, "0.5".to_string())]);
        let err = render_theta_specs(&model, &specs).unwrap_err();
        assert!(err.to_string().contains("repeat"), "got: {err}");
    }

    #[test]
    fn out_of_range_index_is_rejected() {
        let model = parse_model(SCM_TEMPLATE);
        let specs = BTreeMap::from([(9usize, "0.1".to_string())]);
        let err = render_theta_specs(&model, &specs).unwrap_err();
        assert!(err.to_string().contains("out of range"), "got: {err}");
    }

    #[test]
    fn covariance_removal_drops_the_record() {
        let model = parse_model(SCM_TEMPLATE);
        let replacements = model.covariance_removal_replacements();
        let out = model.render_with_replacements(&replacements);
        assert!(!out.contains("$COVARIANCE"), "got:\n{out}");
        // Still parseable, covariance gone
        let reparsed = Model::inner_parse(&out).unwrap();
        assert!(reparsed.covariance.is_none());
    }

    #[test]
    fn combined_theta_and_covariance_replacements() {
        let model = parse_model(SCM_TEMPLATE);
        let specs = BTreeMap::from([(3usize, "0.1".to_string())]);
        let mut replacements = model.theta_spec_replacements(&specs).unwrap();
        replacements.extend(model.covariance_removal_replacements());
        let out = model.render_with_replacements(&replacements);
        assert!(out.contains("$THETA 0.1     ; WT_CL cov"));
        assert!(!out.contains("$COVARIANCE"));
    }

    #[test]
    fn no_covariance_record_yields_empty_map() {
        let input = "\
$PROBLEM t
$INPUT ID
$DATA data.csv
$THETA 1
";
        let model = parse_model(input);
        assert!(model.covariance_removal_replacements().is_empty());
    }
}
