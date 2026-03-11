use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

// $INPUT ---
#[derive(Debug, Clone, PartialEq)]
pub struct InputColumn {
    pub kind: InputColumnKind,
    pub(crate) child_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputColumnKind {
    /// ID
    Included(String),
    /// DOSE=AMT
    Aliased { from: String, to: String },
    /// DATE=DROP
    Dropped(String),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataValueFilter {
    pub field: String,
    pub op: ComparisonOperator,
    pub value: DataValueFilterKind,
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
}

// $THETA ---
#[derive(Debug, Clone, PartialEq)]
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
    pub comment: Option<String>,

    pub(crate) param_child_idx: usize,
    pub(crate) value_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockStructure {
    /// Individual parameters: 0.04
    Diagonal,
    /// BLOCK(n): matrix block
    Block { size: usize },
    /// BLOCK(n) SAME: repeat previous
    BlockSame { size: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct OmegaSigmaBlock {
    pub structure: BlockStructure,
    pub parametrization: Option<Parametrization>,
    pub fixed: bool,        // record-level FIX flag
    pub names: Vec<String>, // from ParamNames or label= syntax
    pub parameters: Vec<OmegaSigmaParam>,

    pub(crate) record_idx: usize,
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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

// $TABLE ---

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub file: Option<String>,
    pub options: BTreeMap<String, Option<String>>,

    pub(crate) record_idx: usize,
    pub(crate) file_idx: Option<usize>,
}

// $SIMULATION ---

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Simulation {
    /// All options including ONLYSIM as a flag
    #[serde(default)]
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

impl Simulation {
    pub fn is_only_sim(&self) -> bool {
        self.options.contains_key("ONLYSIM") || self.options.contains_key("ONLYSIMULATION")
    }
}

// $COVARIANCE ---

#[derive(Debug, Clone, PartialEq)]
pub struct Covariance {
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

// $SUBROUTINE ---
#[derive(Debug, Clone, PartialEq)]
pub enum Subroutine {
    Builtin {
        name: String,
        tolerance: Option<u32>,
    },
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subroutines {
    pub entries: Vec<Subroutine>,
    pub(crate) record_idx: usize,
}
