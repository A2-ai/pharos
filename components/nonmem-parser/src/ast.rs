use crate::nmtran::NmtranToken;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;

// $PROBLEM ---
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Problem {
    pub text: String,
    pub(crate) record_idx: usize,
}

// $INPUT ---
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct InputColumn {
    pub kind: InputColumnKind,
    pub(crate) child_idx: usize,
}

impl Debug for InputColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self.kind))
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
    EqualNumeric,
    NotEqualNumeric,
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
            ComparisonOperator::EqualNumeric => f.write_str("EQN"),
            ComparisonOperator::NotEqualNumeric => f.write_str("NEN"),
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
            "EQN" => Ok(ComparisonOperator::EqualNumeric),
            "NEN" => Ok(ComparisonOperator::NotEqualNumeric),
            _ => Err(
                "Invalid control comparison operator: only EQ, NE, GT, GE, LT, LE, EQN or NEN are allowed"
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
    #[serde(skip)]
    pub(crate) path_idx: Option<usize>,
    pub ignore: Vec<DataFilter>,
    pub accept: Vec<DataFilter>,
    pub num_records: Option<usize>,
    pub null_value: Option<String>,
    /// Unrecognized options: flags (None) and key-value pairs (Some(value))
    #[serde(default)]
    pub other_options: Vec<(String, Option<String>)>,
}

// $THETA ---
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

/// What a diagonal-position value represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagonalScale {
    /// SD or STANDARD keyword — value is a standard deviation, not a variance.
    StandardDeviation,
    /// VAR or VARIANCE keyword — value is a variance (the default, explicitly stated).
    Variance,
}

/// What an off-diagonal-position value represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffDiagonalScale {
    /// CORR or CORRELATION keyword — value is a correlation, not a covariance.
    Correlation,
    /// COV, COVAR, or COVARIANCE keyword — value is a covariance (the default, explicitly stated).
    Covariance,
}

/// How to interpret the numeric values in an omega/sigma block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Parametrization {
    /// CHOLESKY keyword — values are the Cholesky factor. Mutually exclusive with axis flags.
    Cholesky,
    /// Two independent axes for diagonal and off-diagonal interpretation.
    /// `None` on a field means no explicit flag was specified for that axis.
    Axes {
        diagonal: Option<DiagonalScale>,
        off_diagonal: Option<OffDiagonalScale>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OmegaSigmaParam {
    pub value: f64,
    pub name: Option<String>,
    pub comment: Option<String>,

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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
            match p {
                Parametrization::Cholesky => f.write_str(" Cholesky")?,
                Parametrization::Axes {
                    diagonal,
                    off_diagonal,
                } => {
                    if let Some(d) = diagonal {
                        match d {
                            DiagonalScale::StandardDeviation => {
                                f.write_str(" Standard Deviation")?
                            }
                            DiagonalScale::Variance => f.write_str(" Variance")?,
                        }
                    }
                    if let Some(od) = off_diagonal {
                        match od {
                            OffDiagonalScale::Correlation => f.write_str(" Correlation")?,
                            OffDiagonalScale::Covariance => f.write_str(" Covariance")?,
                        }
                    }
                }
            }
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
            if let Some(comment) = &p.comment {
                f.write_str(&format!(" comment='{comment}'"))?;
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Replace {
    pub from: String,
    pub to: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Abbreviated {
    pub replaces: Vec<Replace>,
    pub declares: Vec<String>,
    pub options: BTreeMap<String, Option<String>>,
    pub(crate) record_idx: usize,
}

impl Debug for Abbreviated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        for r in &self.replaces {
            parts.push(format!("REPLACE {}={}", r.from, r.to));
        }
        for d in &self.declares {
            parts.push(format!("DECLARE {d}"));
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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Subroutine {
    Builtin {
        name: String,
        tolerance: Option<u32>,
    },
    Other {
        path: String,
        path_idx: usize,
    },
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
            Subroutine::Other { path, .. } => f.write_str(&format!("Other({path})")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subroutines {
    pub entries: Vec<Subroutine>,
    pub(crate) record_idx: usize,
}

// Code blocks ($PK, $ERROR, $DES, $PRED) ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeBlock {
    pub statements: Vec<NmtranStatement>,
    pub(crate) record_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NmtranStatement {
    Assignment {
        target: String,
        indices: Vec<String>,
        expr: NmtranExpr,
    },
    If {
        condition: NmtranExpr,
        body: Vec<NmtranStatement>,
        elseif_branches: Vec<(NmtranExpr, Vec<NmtranStatement>)>,
        else_body: Option<Vec<NmtranStatement>>,
    },
    DoWhile {
        condition: NmtranExpr,
        body: Vec<NmtranStatement>,
    },
    Call {
        subroutine: String,
        args: Vec<NmtranExpr>,
    },
    Exit {
        args: Vec<String>,
    },
    Unknown {
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => f.write_str("+"),
            BinaryOp::Sub => f.write_str("-"),
            BinaryOp::Mul => f.write_str("*"),
            BinaryOp::Div => f.write_str("/"),
            BinaryOp::Pow => f.write_str("**"),
            BinaryOp::Eq => f.write_str(".EQ."),
            BinaryOp::Ne => f.write_str(".NE."),
            BinaryOp::Lt => f.write_str(".LT."),
            BinaryOp::Le => f.write_str(".LE."),
            BinaryOp::Gt => f.write_str(".GT."),
            BinaryOp::Ge => f.write_str(".GE."),
            BinaryOp::And => f.write_str(".AND."),
            BinaryOp::Or => f.write_str(".OR."),
        }
    }
}

impl From<&NmtranToken> for BinaryOp {
    fn from(tok: &NmtranToken) -> Self {
        match tok {
            NmtranToken::Plus => BinaryOp::Add,
            NmtranToken::Minus => BinaryOp::Sub,
            NmtranToken::Star => BinaryOp::Mul,
            NmtranToken::Slash => BinaryOp::Div,
            NmtranToken::StarStar => BinaryOp::Pow,
            NmtranToken::DotEq | NmtranToken::EqEq => BinaryOp::Eq,
            NmtranToken::DotNe | NmtranToken::SlashEq => BinaryOp::Ne,
            NmtranToken::DotLt | NmtranToken::Lt => BinaryOp::Lt,
            NmtranToken::DotLe | NmtranToken::LtEq => BinaryOp::Le,
            NmtranToken::DotGt | NmtranToken::Gt => BinaryOp::Gt,
            NmtranToken::DotGe | NmtranToken::GtEq => BinaryOp::Ge,
            NmtranToken::DotAnd => BinaryOp::And,
            NmtranToken::DotOr => BinaryOp::Or,
            _ => BinaryOp::Add,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => f.write_str("-"),
            UnaryOp::Pos => f.write_str("+"),
            UnaryOp::Not => f.write_str(".NOT."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NmtranExpr {
    Number(f64),
    Ident(String),
    FunctionCall {
        name: String,
        args: Vec<NmtranExpr>,
    },
    BinaryExpr {
        op: BinaryOp,
        lhs: Box<NmtranExpr>,
        rhs: Box<NmtranExpr>,
    },
    UnaryExpr {
        op: UnaryOp,
        operand: Box<NmtranExpr>,
    },
    Paren(Box<NmtranExpr>),
}

impl fmt::Display for NmtranStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NmtranStatement::Assignment {
                target,
                indices,
                expr,
            } => {
                if indices.is_empty() {
                    write!(f, "{target} = {expr}")
                } else {
                    write!(f, "{target}({}) = {expr}", indices.join(", "))
                }
            }
            NmtranStatement::If {
                condition,
                body,
                elseif_branches,
                else_body,
            } => {
                write!(f, "IF ({condition}) THEN")?;
                for stmt in body {
                    write!(f, " {{ {stmt} }}")?;
                }
                for (cond, stmts) in elseif_branches {
                    write!(f, " ELSEIF ({cond})")?;
                    for stmt in stmts {
                        write!(f, " {{ {stmt} }}")?;
                    }
                }
                if let Some(else_stmts) = else_body {
                    write!(f, " ELSE")?;
                    for stmt in else_stmts {
                        write!(f, " {{ {stmt} }}")?;
                    }
                }
                Ok(())
            }
            NmtranStatement::DoWhile { condition, body } => {
                write!(f, "DO WHILE ({condition})")?;
                for stmt in body {
                    write!(f, " {{ {stmt} }}")?;
                }
                Ok(())
            }
            NmtranStatement::Call { subroutine, args } => {
                if args.is_empty() {
                    write!(f, "CALL {subroutine}")
                } else {
                    let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    write!(f, "CALL {subroutine}({})", args_str.join(", "))
                }
            }
            NmtranStatement::Exit { args } => {
                if args.is_empty() {
                    write!(f, "EXIT")
                } else {
                    write!(f, "EXIT {}", args.join(" "))
                }
            }
            NmtranStatement::Unknown { text } => write!(f, "{text}"),
        }
    }
}

impl fmt::Display for NmtranExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NmtranExpr::Number(n) => {
                if *n == n.trunc() && n.is_finite() {
                    write!(f, "{n:.0}")
                } else {
                    write!(f, "{n}")
                }
            }
            NmtranExpr::Ident(name) => write!(f, "{name}"),
            NmtranExpr::FunctionCall { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{name}({})", args_str.join(", "))
            }
            NmtranExpr::BinaryExpr { op, lhs, rhs } => {
                let sep = match op {
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow => format!("{op}"),
                    _ => format!(" {op} "),
                };
                write!(f, "{lhs}{sep}{rhs}")
            }
            NmtranExpr::UnaryExpr { op, operand } => match op {
                UnaryOp::Not => write!(f, "{op} {operand}"),
                _ => write!(f, "{op}{operand}"),
            },
            NmtranExpr::Paren(inner) => write!(f, "({inner})"),
        }
    }
}
