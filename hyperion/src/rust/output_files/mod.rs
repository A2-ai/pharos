use extendr_api::prelude::*;

// Re-export submodules
pub mod ext;
pub mod grd;
pub mod shk;

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
    pub value: f64,
    pub stderr: Rfloat,
    pub rse: Rfloat,
    pub shrinkage: Rfloat,
    pub fixed: bool,
    pub table_idx: Option<i32>,
    pub method: Option<String>,
}

#[derive(Default)]
pub struct ParameterRowBuilder {
    kind: String,
    name: String,
    random_effect: Option<Rstr>,
    value: f64,
    stderr: Rfloat,
    rse: Rfloat,
    shrinkage: Rfloat,
    fixed: bool,
    table_idx: Option<i32>,
    method: Option<String>,
}

impl ParameterRowBuilder {
    pub fn new(kind: &str, name: String, estimate: f64) -> Self {
        Self {
            kind: kind.to_owned(),
            name,
            random_effect: None,
            value: estimate,
            stderr: Rfloat::na(),
            rse: Rfloat::na(),
            shrinkage: Rfloat::na(),
            fixed: false,
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
            value: self.value,
            stderr: self.stderr,
            rse: self.rse,
            shrinkage: self.shrinkage,
            fixed: self.fixed,
            table_idx: self.table_idx,
            method: self.method,
        }
    }
}

pub struct ParameterTable {
    rows: Vec<ParameterRow>,
    columns: Vec<String>,
}

impl ParameterTable {
    pub fn new(rows: Vec<ParameterRow>, columns: Vec<String>) -> Self {
        Self { rows, columns }
    }

    // These builder methods are not used since columns the resulting
    // Vec<String> is now an argument. Likely will remove but keeping
    // for now.
    pub fn with_kind(mut self) -> Self {
        self.columns.push("kind".to_string());
        self
    }

    pub fn with_name(mut self) -> Self {
        self.columns.push("name".to_string());
        self
    }

    pub fn with_value(mut self) -> Self {
        self.columns.push("value".to_string());
        self
    }

    pub fn with_stderr(mut self) -> Self {
        self.columns.push("stderr".to_string());
        self
    }

    pub fn with_rse(mut self) -> Self {
        self.columns.push("rse".to_string());
        self
    }

    pub fn with_shrinkage(mut self) -> Self {
        self.columns.push("shrinkage".to_string());
        self
    }

    pub fn with_fixed(mut self) -> Self {
        self.columns.push("fixed".to_string());
        self
    }

    pub fn with_table_idx(mut self) -> Self {
        self.columns.push("table_idx".to_string());
        self
    }

    pub fn with_method(mut self) -> Self {
        self.columns.push("method".to_string());
        self
    }

    pub fn with_random_effect(mut self) -> Self {
        self.columns.push("random_effect".to_string());
        self
    }

    pub fn build_df(self) -> Result<Robj> {
        if self.rows.is_empty() {
            return Err(Error::Other(
                "No parameter rows to build dataframe".to_string(),
            ));
        }

        let mut pairs: Vec<(&str, Robj)> = Vec::new();

        // Build columns in the order they were specified
        for column in &self.columns {
            match column.as_str() {
                "kind" => pairs.push((
                    "kind",
                    self.rows
                        .iter()
                        .map(|r| &r.kind)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "name" => pairs.push((
                    "name",
                    self.rows
                        .iter()
                        .map(|r| &r.name)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "value" => pairs.push((
                    "value",
                    self.rows
                        .iter()
                        .map(|r| r.value)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "stderr" => pairs.push((
                    "stderr",
                    self.rows
                        .iter()
                        .map(|r| r.stderr)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "rse" => pairs.push((
                    "rse",
                    self.rows
                        .iter()
                        .map(|r| r.rse)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "shrinkage" => pairs.push((
                    "shrinkage",
                    self.rows
                        .iter()
                        .map(|r| r.shrinkage)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "fixed" => pairs.push((
                    "fixed",
                    self.rows
                        .iter()
                        .map(|r| r.fixed)
                        .collect::<Vec<_>>()
                        .into_robj(),
                )),
                "table_idx" => {
                    let table_indices: Vec<i32> =
                        self.rows.iter().map(|r| r.table_idx.unwrap_or(1)).collect();
                    pairs.push(("table_idx", table_indices.into_robj()));
                }
                "method" => {
                    let methods: Vec<&str> = self
                        .rows
                        .iter()
                        .map(|r| r.method.as_deref().unwrap_or("Unknown"))
                        .collect();
                    pairs.push(("method", methods.into_robj()));
                }
                "random_effect" => {
                    let random_effects: Vec<Rstr> = self
                        .rows
                        .iter()
                        .map(|r| r.random_effect.clone().unwrap_or(Rstr::na()))
                        .collect();
                    pairs.push(("random_effect", random_effects.into_robj()));
                }
                _ => {} // Ignore unknown column names
            }
        }

        let list = List::from_pairs(pairs);
        let df = data_frame!(list);
        Ok(df)
    }
}

extendr_module! {
    mod output_files;

    use ext;
    use grd;
    use shk;
}
