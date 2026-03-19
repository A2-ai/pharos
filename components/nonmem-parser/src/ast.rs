use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;

// $INPUT ---
#[derive(Clone, PartialEq)]
pub struct InputColumn {
    pub kind: InputColumnKind,
    pub(crate) child_idx: usize,
}

impl Debug for InputColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self.kind))
    }
}

#[derive(Clone, PartialEq)]
pub enum InputColumnKind {
    /// ID
    Included(String),
    /// DOSE=AMT
    Aliased { from: String, to: String },
    /// DATE=DROP
    Dropped(String),
}

impl Debug for InputColumnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputColumnKind::Included(name) => f.write_str(&format!("Included({name})")),
            InputColumnKind::Dropped(name) => f.write_str(&format!("Dropped({name})")),
            InputColumnKind::Aliased { from, to } => {
                f.write_str(&format!("Aliased({from} -> {to})"))
            }
        }
    }
}

// $DATA ---
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Lower,
    LowerOrEqual,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonOperator::Equal => f.write_str("EQ"),
            ComparisonOperator::NotEqual => f.write_str("NE"),
            ComparisonOperator::Greater => f.write_str("GT"),
            ComparisonOperator::GreaterOrEqual => f.write_str("GE"),
            ComparisonOperator::Lower => f.write_str("LT"),
            ComparisonOperator::LowerOrEqual => f.write_str("LE"),
        }
    }
}

impl FromStr for ComparisonOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EQ" => Ok(ComparisonOperator::Equal),
            "NE" => Ok(ComparisonOperator::NotEqual),
            "GT" => Ok(ComparisonOperator::Greater),
            "GE" => Ok(ComparisonOperator::GreaterOrEqual),
            "LT" => Ok(ComparisonOperator::Lower),
            "LE" => Ok(ComparisonOperator::LowerOrEqual),
            _ => Err(
                "Invalid control comparison operator: only EQ, NE, GT, GE, LT or LE are allowed"
                    .to_string(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValueFilterKind {
    Number(f64),
    String(String),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DataValueFilter {
    pub field: String,
    pub op: ComparisonOperator,
    pub value: DataValueFilterKind,
}

impl Debug for DataValueFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?} {} {:?}", self.field, self.op, self.value))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataFilter {
    ValueFilter(DataValueFilter),
    Marker(String),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Data {
    pub path: String,
    pub ignore: Vec<DataFilter>,
    pub accept: Vec<DataFilter>,
    pub num_records: Option<usize>,
    pub null_value: Option<String>,
    /// Unrecognized options: flags (None) and key-value pairs (Some(value))
    #[serde(default)]
    pub other_options: Vec<(String, Option<String>)>,
}

// $THETA ---
#[derive(Clone, PartialEq)]
pub struct ThetaParameter {
    pub name: Option<String>,
    pub lower: Option<f64>,
    pub init: f64,
    pub upper: Option<f64>,
    pub fixed: bool,
    pub comment: Option<String>,

    // index in CST for whole Theta record
    pub(crate) record_idx: usize,
    // index in Theta.children for this Param node
    pub(crate) param_child_idx: usize,
    pub(crate) lower_idx: Option<usize>,
    pub(crate) init_idx: usize,
    pub(crate) upper_idx: Option<usize>,
}

impl Debug for ThetaParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = vec![];
        if let Some(name) = &self.name {
            out.push(format!("name='{name}'"));
        }
        if let Some(v) = &self.lower {
            out.push(format!("lower={v}"));
        }
        out.push(format!("init={}", self.init));
        if let Some(v) = &self.upper {
            out.push(format!("upper={v}"));
        }
        if self.fixed {
            out.push("FIX".to_string());
        }
        if let Some(v) = &self.comment {
            out.push(format!("comment='{}'", v));
        }

        f.write_str(&out.join(" "))
    }
}

// $OMEGA / $SIGMA ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Parametrization {
    Correlation,
    StandardDeviation,
    Cholesky,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OmegaSigmaParam {
    pub value: f64,
    pub fixed: bool,
    pub name: Option<String>,

    pub(crate) param_child_idx: usize,
    pub(crate) value_idx: usize,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockStructure {
    /// Individual parameters: 0.04
    Diagonal,
    /// BLOCK(n): matrix block
    Block { size: usize },
    /// BLOCK(n) SAME[(m)]: repeat the previous block `repeats` times
    BlockSame { size: usize, repeats: usize },
}

impl Debug for BlockStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockStructure::Diagonal => f.write_str("Diagonal"),
            BlockStructure::Block { size } => f.write_str(&format!("Block({size})")),
            BlockStructure::BlockSame { size, repeats } if *repeats > 1 => {
                f.write_str(&format!("BlockSame({size})x{repeats}"))
            }
            BlockStructure::BlockSame { size, .. } => f.write_str(&format!("BlockSame({size})")),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct OmegaSigmaBlock {
    pub structure: BlockStructure,
    pub parametrization: Option<Parametrization>,
    pub fixed: bool,        // record-level FIX flag
    pub names: Vec<String>, // from ParamNames or label= syntax
    pub parameters: Vec<OmegaSigmaParam>,

    pub(crate) record_idx: usize,
}

impl Debug for OmegaSigmaBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(format!("{:?}", self.structure).as_str())?;

        if let Some(p) = &self.parametrization {
            f.write_str(format!(" {p:?}").as_str())?;
        }

        if self.fixed {
            f.write_str(" FIX")?;
        }
        if !self.names.is_empty() {
            f.write_str(&format!(" NAMES({})", self.names.join(",")))?;
        }
        f.write_str("\n")?;

        if self.parameters.is_empty() {
            return Ok(());
        }

        // For DIAGONAL: one value per line
        // For BLOCK(n): lower-triangular matrix rows (row 0 has 1 value, row 1 has 2, etc.)
        let row_size = match self.structure {
            BlockStructure::Block { size } => size,
            _ => 0, // diagonal: each param is its own row
        };

        let fmt_param = |f: &mut fmt::Formatter<'_>, p: &OmegaSigmaParam| {
            if let Some(name) = &p.name {
                f.write_str(&format!("name={name} "))?;
            }
            f.write_str(&format!("{}", p.value))?;
            if p.fixed {
                f.write_str(" FIX")?;
            }
            Ok(())
        };

        if row_size == 0 {
            // Diagonal
            for p in &self.parameters {
                f.write_str("  ")?;
                fmt_param(f, p)?;
                f.write_str("\n")?;
            }
        } else {
            // Block: lower triangle matrix layout
            let mut i = 0;
            for row in 0..row_size {
                f.write_str("  ")?;
                for col in 0..=row {
                    if col > 0 {
                        f.write_str("  ")?;
                    }
                    if i < self.parameters.len() {
                        fmt_param(f, &self.parameters[i])?;
                    }
                    i += 1;
                }
                f.write_str("\n")?;
            }
        }

        Ok(())
    }
}

// $EST --
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum EstimationMethod {
    #[default]
    Fo,
    Foce,
    Saem,
    Bayes,
    Imp,
    ImpMap,
    Its,
    Nuts,
}

impl fmt::Display for EstimationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstimationMethod::Fo => f.write_str("FO"),
            EstimationMethod::Foce => f.write_str("FOCE"),
            EstimationMethod::Saem => f.write_str("SAEM"),
            EstimationMethod::Bayes => f.write_str("Bayes"),
            EstimationMethod::Imp => f.write_str("IMP"),
            EstimationMethod::ImpMap => f.write_str("IMPMAP"),
            EstimationMethod::Its => f.write_str("ITS"),
            EstimationMethod::Nuts => f.write_str("NUTS"),
        }
    }
}

impl FromStr for EstimationMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str().replace("(NO PRIOR)", "").trim() {
            "0" | "FO" | "FIRST ORDER WITH INTERACTION" => Ok(EstimationMethod::Fo),
            "1" | "FOCE" | "COND" | "FIRST ORDER CONDITIONAL ESTIMATION WITH INTERACTION" => {
                Ok(EstimationMethod::Foce)
            }
            "SAEM" | "STOCHASTIC APPROXIMATION EXPECTATION-MAXIMIZATION" => {
                Ok(EstimationMethod::Saem)
            }
            "BAYES" | "MCMC BAYESIAN ANALYSIS" => Ok(EstimationMethod::Bayes),
            "IMP"
            | "IMPORTANCE SAMPLING"
            | "OBJECTIVE FUNCTION EVALUATION BY IMPORTANCE SAMPLING" => Ok(EstimationMethod::Imp),
            "IMPMAP" | "IMPORTANCE SAMPLING ASSISTED BY MAP ESTIMATION" => {
                Ok(EstimationMethod::ImpMap)
            }
            "ITS" | "ITERATIVE TWO STAGE" | "ITERATIVE TWO STAGE (NO PRIOR)" => {
                Ok(EstimationMethod::Its)
            }
            "NUTS" | "NUTS BAYESIAN ANALYSIS" => Ok(EstimationMethod::Nuts),
            "LAPLACE" => Ok(EstimationMethod::Bayes),
            _ => Err(format!("Unknown estimation method: {s}")),
        }
    }
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Estimation {
    pub method: EstimationMethod,
    pub msfo: Option<PathBuf>,
    pub file: Option<PathBuf>,
    /// All other options - value is None for flags (e.g., INTERACTION, POSTHOC)
    /// and Some(value) for key=value pairs (e.g., MAXEVAL=9999, PRINT=5)
    #[serde(default)]
    pub options: BTreeMap<String, Option<String>>,

    pub(crate) record_idx: usize,
    pub(crate) msfo_idx: Option<usize>,
    pub(crate) file_idx: Option<usize>,
}

impl Debug for Estimation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        parts.push(format!("method={}", self.method));
        if let Some(v) = &self.msfo {
            parts.push(format!("msfo={}", v.to_string_lossy()));
        }
        if let Some(v) = &self.file {
            parts.push(format!("file={}", v.to_string_lossy()));
        }

        for (key, val) in &self.options {
            if let Some(v) = val {
                parts.push(format!("{key}={v}"));
            } else {
                parts.push(key.to_string());
            }
        }

        f.write_str(&parts.join(" "))
    }
}

// $TABLE ---

#[derive(Clone, PartialEq)]
pub struct Table {
    pub file: Option<String>,
    pub options: Vec<(String, Option<String>)>,

    pub(crate) record_idx: usize,
    pub(crate) file_idx: Option<usize>,
}

impl Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        if let Some(v) = &self.file {
            parts.push(format!("file={v}"));
        }

        for (key, val) in &self.options {
            if let Some(v) = val {
                parts.push(format!("{key}={v}"));
            } else {
                parts.push(key.clone());
            }
        }

        f.write_str(&parts.join(" "))
    }
}

// $SIMULATION ---

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Simulation {
    /// All options including ONLYSIM as a flag
    #[serde(default)]
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

impl Debug for Simulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        for (key, val) in &self.options {
            if let Some(v) = val {
                parts.push(format!("{key}={v}"));
            } else {
                parts.push(key.to_string());
            }
        }

        f.write_str(&parts.join(" "))
    }
}

impl Simulation {
    pub fn is_only_sim(&self) -> bool {
        self.options.contains_key("ONLYSIM") || self.options.contains_key("ONLYSIMULATION")
    }
}

// $COVARIANCE ---

#[derive(Clone, PartialEq)]
pub struct Covariance {
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

impl Debug for Covariance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        for (key, val) in &self.options {
            if let Some(v) = val {
                parts.push(format!("{key}={v}"));
            } else {
                parts.push(key.to_string());
            }
        }

        f.write_str(&parts.join(" "))
    }
}

// $ABBREVIATED ---
#[derive(Debug, Clone, PartialEq)]
pub struct Replace {
    pub from: String,
    pub to: String,
}

#[derive(Clone, PartialEq)]
pub struct Abbreviated {
    pub replaces: Vec<Replace>,
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

impl Debug for Abbreviated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        for r in &self.replaces {
            parts.push(format!("REPLACE {}={}", r.from, r.to));
        }
        for (key, val) in &self.options {
            if let Some(v) = val {
                parts.push(format!("{key}={v}"));
            } else {
                parts.push(key.to_string());
            }
        }
        f.write_str(&parts.join(" "))
    }
}

// $SUBROUTINE ---
#[derive(Clone, PartialEq)]
pub enum Subroutine {
    Builtin {
        name: String,
        tolerance: Option<u32>,
    },
    Other(String),
}

impl Debug for Subroutine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Subroutine::Builtin { name, tolerance } => {
                if let Some(tolerance) = tolerance {
                    f.write_str(&format!("Builtin({name}, tol={tolerance})"))
                } else {
                    f.write_str(&format!("Builtin({name})"))
                }
            }
            Subroutine::Other(name) => f.write_str(&format!("Other({name})")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subroutines {
    pub entries: Vec<Subroutine>,
    pub(crate) record_idx: usize,
}
