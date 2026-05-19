use std::path::Path;

use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};

use crate::ast::{
    Abbreviated, CodeBlock, Covariance, Data, Estimation, InputColumn, Msfi, OmegaSigmaBlock,
    Problem, Simulation, Subroutines, Table, ThetaParameter,
};
use crate::cst::CstNode;
use crate::errors;
use crate::lexer::SpannedToken;
use crate::lower::Lowerer;
use crate::parser::Parser;
use errors::Diagnostic;

mod copy;
mod estimates;
pub mod parameters;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Model {
    // CST
    pub(crate) cst: CstNode,
    pub(crate) tokens: Vec<SpannedToken>,
    pub source: String,

    // AST
    pub problem: Problem,
    pub input_columns: Vec<InputColumn>,
    pub data: Data,
    pub thetas: Vec<ThetaParameter>,
    pub omega_blocks: Vec<OmegaSigmaBlock>,
    pub sigma_blocks: Vec<OmegaSigmaBlock>,
    pub estimations: Vec<Estimation>,
    pub tables: Vec<Table>,
    pub simulation: Option<Simulation>,
    pub msfi: Option<Msfi>,
    pub covariance: Option<Covariance>,
    pub subroutines: Option<Subroutines>,
    pub abbreviated: Option<Abbreviated>,
    pub pk: Option<CodeBlock>,
    pub error: Option<CodeBlock>,
    pub des: Option<CodeBlock>,
    pub pred: Option<CodeBlock>,
}

impl Model {
    pub(crate) fn inner_parse(input: &str) -> Result<Model, Vec<Diagnostic>> {
        let parser = Parser::new(input);
        let (cst, tokens, source) = match parser.parse() {
            Ok(result) => result,
            Err(diag) => return Err(vec![diag]),
        };
        let lowerer = Lowerer::new(tokens.as_slice());
        let (mut model, diagnostics) = lowerer.lower(&cst);
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        model.cst = cst;
        model.tokens = tokens;
        model.source = source;
        Ok(model)
    }

    pub fn parse(path: impl AsRef<Path>, input: &str) -> AnyhowResult<Model> {
        Model::inner_parse(input).map_err(|diags| {
            // Render against the same normalized source the parser used (see Parser::new),
            // otherwise byte-offset spans land on the wrong line/column for CRLF inputs.
            let normalized = input.replace("\r\n", "\n");
            anyhow::anyhow!(
                "{}",
                diags
                    .iter()
                    .map(|d| d.render(path.as_ref(), &normalized))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
    }

    #[cfg(test)]
    pub(crate) fn debug_ast(&self) -> String {
        use std::fmt::Write;

        let mut out = String::new();
        out.write_str(format!("problem: '{}'\n", self.problem.text).as_str())
            .unwrap();
        if !self.input_columns.is_empty() {
            out.write_str("input:\n").unwrap();
            for input_column in &self.input_columns {
                out.write_str(&format!("  {input_column:?}\n")).unwrap();
            }
        }

        out.write_str("data:\n").unwrap();
        out.write_str(&format!("  {:?}\n", self.data.path)).unwrap();
        if !self.data.ignore.is_empty() {
            out.write_str("  ignore:\n").unwrap();

            for ignore in &self.data.ignore {
                out.write_str(&format!("    {ignore:?}\n")).unwrap();
            }
        }
        if !self.data.accept.is_empty() {
            out.write_str("  accept:\n").unwrap();

            for accept in &self.data.accept {
                out.write_str(&format!("    {accept:?}\n")).unwrap();
            }
        }
        if let Some(v) = &self.data.null_value {
            out.write_str(&format!("  null value: {v:?}\n")).unwrap();
        }
        if let Some(v) = &self.data.num_records {
            out.write_str(&format!("  num records: {v:?}\n")).unwrap();
        }
        if !self.data.other_options.is_empty() {
            out.write_str("  other options:\n").unwrap();
            for (key, val) in &self.data.other_options {
                if let Some(v) = val {
                    out.write_str(&format!("    {key}={v}\n")).unwrap();
                } else {
                    out.write_str(&format!("    {key}\n")).unwrap();
                }
            }
        }

        for (i, theta) in self.thetas.iter().enumerate() {
            out.write_str(&format!("theta[{i}]: {theta:?}\n")).unwrap();
        }

        out.write_str("\n").unwrap();

        for (i, omega) in self.omega_blocks.iter().enumerate() {
            out.write_str(&format!("omega[{i}]: {omega:?}")).unwrap();
        }
        out.write_str("\n").unwrap();

        for (i, sigma) in self.sigma_blocks.iter().enumerate() {
            out.write_str(&format!("sigma[{i}]: {sigma:?}")).unwrap();
        }
        out.write_str("\n").unwrap();

        for (i, estimation) in self.estimations.iter().enumerate() {
            out.write_str(&format!("estimation[{i}]: {estimation:?}\n"))
                .unwrap();
        }

        for (i, table) in self.tables.iter().enumerate() {
            out.write_str(&format!("table[{i}]: {table:?}\n")).unwrap();
        }

        if let Some(v) = &self.simulation {
            out.write_str(&format!("simulation: {v:?}\n")).unwrap();
        }

        if let Some(v) = &self.msfi {
            out.write_str(&format!("msfi: {v:?}\n")).unwrap();
        }

        if let Some(v) = &self.covariance {
            out.write_str(&format!("covariance: {v:?}\n")).unwrap();
        }

        if let Some(v) = &self.subroutines {
            out.write_str("subroutines:\n").unwrap();
            for subroutine in &v.entries {
                out.write_str(&format!("  {subroutine:?}\n")).unwrap();
            }
        }

        if let Some(v) = &self.abbreviated {
            out.write_str("abbreviated:\n").unwrap();
            for r in &v.replaces {
                out.write_str(&format!("  REPLACE {}={}\n", r.from, r.to))
                    .unwrap();
            }
            for d in &v.declares {
                out.write_str(&format!("  DECLARE {d}\n")).unwrap();
            }
            for (key, val) in &v.options {
                if let Some(v) = val {
                    out.write_str(&format!("  {key}={v}\n")).unwrap();
                } else {
                    out.write_str(&format!("  {key}\n")).unwrap();
                }
            }
        }

        for (label, block) in [
            ("pk", &self.pk),
            ("error", &self.error),
            ("des", &self.des),
            ("pred", &self.pred),
        ] {
            if let Some(cb) = block {
                out.write_str(&format!("{label}:\n")).unwrap();
                for stmt in &cb.statements {
                    out.write_str(&format!("  {stmt}\n")).unwrap();
                }
            }
        }

        out
    }
}
