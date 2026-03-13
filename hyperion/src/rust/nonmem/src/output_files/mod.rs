use extendr_api::Result;
use extendr_api::prelude::*;

// Re-export submodules
pub mod ext;
pub mod grd;
pub mod shk;
pub mod transforms;

use hyperion_core::extendr_err;

// String constants to avoid repeated allocations
pub const THETA: &str = "THETA";
pub const OMEGA: &str = "OMEGA";
pub const SIGMA: &str = "SIGMA";

/// Flexible parameter row with optional fields
#[derive(Debug)]
pub struct ParameterRow {
    pub kind: String,
    pub name: String,
    pub random_effect: Option<Rstr>,
    pub estimate: f64,
    pub sd: Rfloat,
    pub corr: Rfloat,
    pub stderr: Rfloat,
    pub rse: Rfloat,
    pub shrinkage: Rfloat,
    pub fixed: bool,
    pub diagonal: Option<bool>,
    pub table_idx: Option<i32>,
    pub method: Option<String>,
}

#[derive(Default)]
pub struct ParameterRowBuilder {
    kind: String,
    name: String,
    random_effect: Option<Rstr>,
    estimate: f64,
    sd: Rfloat,
    corr: Rfloat,
    stderr: Rfloat,
    rse: Rfloat,
    shrinkage: Rfloat,
    fixed: bool,
    diagonal: Option<bool>,
    table_idx: Option<i32>,
    method: Option<String>,
}

impl ParameterRowBuilder {
    pub fn new(kind: &str, name: String, estimate: f64) -> Self {
        Self {
            kind: kind.to_owned(),
            name,
            random_effect: None,
            estimate,
            sd: Rfloat::na(),
            corr: Rfloat::na(),
            stderr: Rfloat::na(),
            rse: Rfloat::na(),
            shrinkage: Rfloat::na(),
            fixed: false,
            diagonal: None,
            table_idx: None,
            method: None,
        }
    }

    pub fn with_stderr_rse(mut self, stderr: Option<f64>, rse: Option<f64>, fixed: bool) -> Self {
        self.stderr = if fixed {
            Rfloat::na()
        } else {
            stderr.map_or(Rfloat::na(), Rfloat::from)
        };
        self.rse = if fixed {
            Rfloat::na()
        } else {
            rse.map_or(Rfloat::na(), Rfloat::from)
        };
        self.fixed = fixed;
        self
    }

    pub fn with_sd(mut self, sd: Option<f64>) -> Self {
        self.sd = sd.map_or(Rfloat::na(), Rfloat::from);
        self
    }

    pub fn with_corr(mut self, corr: Option<f64>) -> Self {
        self.corr = corr.map_or(Rfloat::na(), Rfloat::from);
        self
    }

    pub fn with_diagonal(mut self, diagonal: bool) -> Self {
        self.diagonal = Some(diagonal);
        self
    }

    pub fn with_table_idx(mut self, idx: i32) -> Self {
        self.table_idx = Some(idx);
        self
    }

    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }

    pub fn with_shrinkage(mut self, shrinkage: Option<f64>, fixed: bool) -> Self {
        self.shrinkage = if fixed {
            Rfloat::na()
        } else {
            shrinkage.map_or(Rfloat::na(), Rfloat::from)
        };
        self
    }

    pub fn with_random_effect(mut self, random_effect: String) -> Self {
        self.random_effect = Some(Rstr::from(random_effect));
        self
    }

    pub fn build(self) -> ParameterRow {
        ParameterRow {
            kind: self.kind,
            name: self.name,
            random_effect: self.random_effect,
            estimate: self.estimate,
            sd: self.sd,
            corr: self.corr,
            stderr: self.stderr,
            rse: self.rse,
            shrinkage: self.shrinkage,
            fixed: self.fixed,
            diagonal: self.diagonal,
            table_idx: self.table_idx,
            method: self.method,
        }
    }
}

/// Build a dataframe from parameter rows
pub fn build_parameters_df(
    rows: Vec<ParameterRow>,
    with_table_idx: bool,
    with_method: bool,
) -> Result<Robj> {
    if rows.is_empty() {
        return Err(extendr_err!("No parameter rows to build dataframe"));
    }

    let mut pairs: Vec<(&str, Robj)> = vec![
        (
            "kind",
            rows.iter().map(|r| &r.kind).collect::<Vec<_>>().into_robj(),
        ),
        (
            "name",
            rows.iter().map(|r| &r.name).collect::<Vec<_>>().into_robj(),
        ),
        (
            "random_effect",
            rows.iter()
                .map(|r| r.random_effect.clone().unwrap_or(Rstr::na()))
                .collect::<Vec<_>>()
                .into_robj(),
        ),
        (
            "estimate",
            rows.iter()
                .map(|r| r.estimate)
                .collect::<Vec<_>>()
                .into_robj(),
        ),
        (
            "sd",
            rows.iter().map(|r| r.sd).collect::<Vec<_>>().into_robj(),
        ),
        (
            "corr",
            rows.iter().map(|r| r.corr).collect::<Vec<_>>().into_robj(),
        ),
        (
            "stderr",
            rows.iter()
                .map(|r| r.stderr)
                .collect::<Vec<_>>()
                .into_robj(),
        ),
        (
            "rse",
            rows.iter().map(|r| r.rse).collect::<Vec<_>>().into_robj(),
        ),
        (
            "shrinkage",
            rows.iter()
                .map(|r| r.shrinkage)
                .collect::<Vec<_>>()
                .into_robj(),
        ),
        (
            "fixed",
            rows.iter().map(|r| r.fixed).collect::<Vec<_>>().into_robj(),
        ),
        (
            "diagonal",
            rows.iter()
                .map(|r| r.diagonal)
                .collect::<Vec<_>>()
                .into_robj(),
        ),
    ];

    if with_table_idx {
        pairs.push((
            "table_idx",
            rows.iter()
                .map(|r| r.table_idx.unwrap_or(1))
                .collect::<Vec<_>>()
                .into_robj(),
        ));
    }

    if with_method {
        pairs.push((
            "method",
            rows.iter()
                .map(|r| r.method.as_deref().unwrap_or("Unknown"))
                .collect::<Vec<_>>()
                .into_robj(),
        ));
    }

    let list = List::from_pairs(pairs);
    Ok(data_frame!(list))
}

extendr_module! {
    mod output_files;

    use ext;
    use grd;
    use shk;
    use transforms;
}
