use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::CopyOptions;
use crate::estimation::EstimationMethod;
use crate::output_files::ext::{ExtReader, get_parameter_estimates};
use crate::parsing::Token;
use crate::parsing::comments::{
    ParamName, ParsedOmegaComment, ParsedSigmaComment, ParsedThetaComment, parse_omega_param,
    parse_sigma_param, parse_theta_param,
};
use crate::parsing::errors::SyntaxError;
use crate::parsing::parser::Parser;
use crate::parsing::utils::{
    ParameterOrdering, apply_jittering, replace_stem_in_path, round_arbitrary_precision,
};
use anyhow::{Result as AnyhowResult, bail};
use config::CommentType;
use fs_err as fs;
use rand::prelude::*;

const OMEGA: &str = "OMEGA";
const SIGMA: &str = "SIGMA";
const ETA: &str = "ETA";
const EPS: &str = "EPS";

#[allow(clippy::too_many_arguments)]
fn update_parameter_blocks<T: ParamName>(
    blocks: &mut [ParameterBlock<T>],
    token_indices: &[Vec<usize>],
    tokens: &mut [Token],
    // If it's None, this means update its own value
    parameters: Option<HashMap<&str, f64>>,
    param_prefix: &str,
    excluded_parameters: &[String],
    jitter_percentage: Option<f64>,
    mut rng: Option<&mut StdRng>,
) {
    let mut param_counter = 1;

    let mut update_single_param = |param_name: &str, param: &mut Parameter<T>, token_idx: usize| {
        if param.is_fixed {
            return;
        }

        let value = if let Some(parameters) = &parameters {
            if let Some(value) = parameters.get(param_name) {
                *value
            } else {
                return;
            }
        } else {
            param.initial_value
        };

        let mut final_value = value;

        // Only apply jittering if NOT excluded
        if let (Some(jitter_pct), Some(ref mut rng_mut)) = (jitter_percentage, rng.as_mut())
            && !excluded_parameters.contains(&param_name.to_string())
        {
            let original_str = match &tokens[token_idx] {
                Token::Number { original, .. } => original.clone(),
                _ => value.to_string(),
            };

            final_value = apply_jittering(
                value,
                jitter_pct,
                rng_mut,
                param.lower_bound,
                param.upper_bound,
                &original_str,
            );
        }

        // Always update the parameter (regardless of jitter exclusion)
        param.initial_value = final_value;
        if let Token::Number { value, original } = &mut tokens[token_idx] {
            let rounded = round_arbitrary_precision(original, final_value);
            *value = rounded;
            *original = rounded.to_string();
        }
    };

    for (block_idx, block) in blocks.iter_mut().enumerate() {
        match &block.structure {
            BlockStructure::Diagonal => {
                for (param_idx, param) in block.parameters.iter_mut().enumerate() {
                    if block_idx < token_indices.len() && param_idx < token_indices[block_idx].len()
                    {
                        let param_name =
                            format!("{}({},{})", param_prefix, param_counter, param_counter);
                        update_single_param(
                            &param_name,
                            param,
                            token_indices[block_idx][param_idx],
                        );
                    }
                    param_counter += 1;
                }
            }
            BlockStructure::Block { size } => {
                let mut param_idx = 0;
                for row in 0..*size {
                    for col in 0..=row {
                        if param_idx < block.parameters.len()
                            && block_idx < token_indices.len()
                            && param_idx < token_indices[block_idx].len()
                        {
                            let param = &mut block.parameters[param_idx];
                            let param_name = format!(
                                "{}({},{})",
                                param_prefix,
                                param_counter + row,
                                param_counter + col
                            );
                            update_single_param(
                                &param_name,
                                param,
                                token_indices[block_idx][param_idx],
                            );
                        }
                        if param_idx < block.parameters.len() {
                            param_idx += 1;
                        }
                    }
                }
                param_counter += size;
            }
            BlockStructure::BlockSame { size } => {
                param_counter += size;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputColumn {
    Included(String),
    Aliased { from: String, to: String },
    Dropped(String),
}

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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(bound = "T: DeserializeOwned")]
pub struct Parameter<T: ParamName> {
    pub name: Option<String>,
    pub lower_bound: Option<f64>,
    pub initial_value: f64,
    pub upper_bound: Option<f64>,
    pub is_fixed: bool,
    pub comment: Option<String>,
    pub parsed_comment: Option<T>,
}

impl<T: ParamName> Parameter<T> {
    pub fn name(&self) -> Option<String> {
        if let Some(s) = &self.parsed_comment.as_ref() {
            s.name()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Parameterization {
    Correlation,
    StandardDeviation,
    Cholesky,
}

impl Parameterization {
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword.to_uppercase().as_str() {
            "CORR" | "CORRELATION" => Some(Self::Correlation),
            "SD" => Some(Self::StandardDeviation),
            "CHOLESKY" => Some(Self::Cholesky),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockStructure {
    Diagonal,                  // Individual parameters: 0.04
    Block { size: usize },     // BLOCK(n): matrix block
    BlockSame { size: usize }, // BLOCK(n) SAME: repeat previous
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound = "T: DeserializeOwned")]
pub struct ParameterBlock<T: ParamName> {
    pub structure: BlockStructure,
    // None = default
    pub parametrization: Option<Parameterization>,
    pub parameters: Vec<Parameter<T>>,
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
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Simulation {
    /// All options including ONLYSIM as a flag
    #[serde(default)]
    pub options: BTreeMap<String, Option<String>>,
}

impl Simulation {
    pub fn is_only_sim(&self) -> bool {
        self.options.contains_key("ONLYSIM") || self.options.contains_key("ONLYSIMULATION")
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Covariance {
    #[serde(default)]
    pub options: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Subroutine {
    Builtin {
        name: String,
        tolerance: Option<u32>,
    },
    Other(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelTokenRanges {
    pub theta_initial_values: Vec<usize>,
    pub table_files: Vec<usize>,
    // (file idx, msfo idx)
    pub estimations: Vec<(Option<usize>, Option<usize>)>,
    // Vec<Vec<..>> because we can have multiple params in each block
    pub omega_initial_values: Vec<Vec<usize>>,
    // Vec<Vec<..>> because we can have multiple params in each block
    pub sigma_initial_values: Vec<Vec<usize>>,
    // Index of the Token::Ignored containing the problem statement content
    pub problem_content: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Dataset {
    pub canonical_path: PathBuf,
    pub blake3_hash: String,
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Model {
    pub problem: String,
    pub input_columns: Vec<InputColumn>,
    pub data: Data,
    pub subroutines: Vec<Subroutine>,
    pub theta_parameters: Vec<Parameter<ParsedThetaComment>>,
    pub omega_blocks: Vec<ParameterBlock<ParsedOmegaComment>>,
    pub sigma_blocks: Vec<ParameterBlock<ParsedSigmaComment>>,
    pub estimations: Vec<Estimation>,
    pub tables: Vec<PathBuf>,
    pub simulation: Option<Simulation>,
    pub covariance: Option<Covariance>,
    // Token range tracking for editing
    pub token_ranges: ModelTokenRanges,
    // Original tokens for reconstruction
    pub(crate) tokens: Vec<Token>,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("problem", &self.problem)
            .field("input_columns", &self.input_columns)
            .field("data", &self.data)
            .field("subroutines", &self.subroutines)
            .field("theta_parameters", &self.theta_parameters)
            .field("omega_blocks", &self.omega_blocks)
            .field("sigma_blocks", &self.sigma_blocks)
            .field("estimations", &self.estimations)
            .field("tables", &self.tables)
            .field("simulation", &self.simulation)
            .field("covariance", &self.covariance)
            .finish()
    }
}

impl Model {
    pub fn parse(input: &str) -> Result<Self, SyntaxError> {
        let input = input.replace("\r\n", "\n");
        match Parser::new(&input).and_then(|mut p| p.parse()) {
            Ok(p) => Ok(p),
            Err(mut e) => {
                e.generate_report(&input);
                Err(e)
            }
        }
    }

    /// Deserialize a Model from JSON string
    pub fn from_json(json: &str) -> AnyhowResult<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize a Model to JSON string
    pub fn to_json(&self) -> AnyhowResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Iterate over OMEGA parameters in specified order, yielding (param_name, eta_label, parameter)
    /// param_name is OMEGA(i,j), eta_label is ETAj:ETAi or ETAi for OMEGA(i,i)
    pub fn get_omega_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> AnyhowResult<Vec<(String, String, &Parameter<ParsedOmegaComment>)>> {
        get_block_parameter_names(&self.omega_blocks, ordering, OMEGA, ETA)
    }

    /// Iterate over SIGMA parameters in specified order, yielding (param_name, eps_label, parameter)
    /// param_name is SIGMA(i,j), eps_label is EPSj:EPSi or EPSi for SIGMA(i,i)
    pub fn get_sigma_parameters(
        &self,
        ordering: ParameterOrdering,
    ) -> AnyhowResult<Vec<(String, String, &Parameter<ParsedSigmaComment>)>> {
        get_block_parameter_names(&self.sigma_blocks, ordering, SIGMA, EPS)
    }

    /// Parse the parameter comments and return the raw string of the comments that didn't parse
    /// for the given type.
    pub fn parse_comments(&mut self, typ_: CommentType) -> Vec<String> {
        let mut out = Vec::new();
        for theta in self.theta_parameters.iter_mut() {
            if let Some(c) = theta.comment.as_ref() {
                theta.parsed_comment = parse_theta_param(c.as_str(), typ_);
                if theta.parsed_comment.is_none() {
                    out.push(c.to_string());
                }
            }
        }

        for block in self.omega_blocks.iter_mut() {
            for p in block.parameters.iter_mut() {
                if let Some(c) = p.comment.as_ref() {
                    p.parsed_comment = parse_omega_param(c.as_str(), typ_);
                    if p.parsed_comment.is_none() {
                        out.push(c.to_string());
                    }
                }
            }
        }

        for block in self.sigma_blocks.iter_mut() {
            for p in block.parameters.iter_mut() {
                if let Some(c) = p.comment.as_ref() {
                    p.parsed_comment = parse_sigma_param(c.as_str(), typ_);
                    if p.parsed_comment.is_none() {
                        out.push(c.to_string());
                    }
                }
            }
        }

        out
    }

    /// Generate BTreeMap of NONMEM parameter names to user-friendly names
    pub fn get_parameter_names(
        &mut self,
        comment_type: Option<CommentType>,
    ) -> AnyhowResult<BTreeMap<String, Option<String>>> {
        if let Some(c) = comment_type {
            self.parse_comments(c);
        }

        let mut parameter_names = BTreeMap::new();

        // Add THETA parameter names
        for (i, param) in self.theta_parameters.iter().enumerate() {
            parameter_names.insert(format!("THETA{}", i + 1), param.name());
        }

        // Add OMEGA parameter names (RowMajor to match EXT file order)
        let omega_names = self.get_omega_parameters(ParameterOrdering::RowMajor)?;
        for (ext_name, _eta_label, param) in omega_names {
            parameter_names.insert(ext_name, param.name());
        }

        // Add SIGMA parameter names (RowMajor to match EXT file order)
        let sigma_names = self.get_sigma_parameters(ParameterOrdering::RowMajor)?;
        for (ext_name, _eps_label, param) in sigma_names {
            parameter_names.insert(ext_name, param.name());
        }

        Ok(parameter_names)
    }

    pub fn check_dataset(&self, model_dir: &Path) -> AnyhowResult<Dataset> {
        let p = model_dir.join(&self.data.path);
        if !p.exists() {
            bail!("Dataset {p:?} not found");
        }

        let data = fs::read(&p)?;
        let blake3_hash = format!("{}", blake3::hash(&data));

        Ok(Dataset {
            canonical_path: p.canonicalize()?,
            blake3_hash,
        })
    }

    pub fn paths_to_replace(&self) -> HashMap<String, String> {
        let mut output = HashMap::new();
        let mut paths = vec![];

        for est in &self.estimations {
            if let Some(p) = &est.msfo {
                paths.push(p)
            }
            if let Some(p) = &est.file {
                paths.push(p);
            }
        }
        paths.extend(self.tables.iter());
        for sub in &self.subroutines {
            if let Subroutine::Other(p) = sub {
                paths.push(p);
            }
        }

        for p in paths {
            // Extract just the filename from paths like "../2.TAB" -> "2.TAB"
            let original_filename = p.as_os_str().to_string_lossy().to_string();
            let new_filename = p
                .file_name()
                .unwrap_or(p.as_os_str())
                .to_string_lossy()
                .to_string();

            output.insert(original_filename, new_filename);
        }

        output
    }

    pub fn update_table_path(&mut self, index: usize, new_path: &str) {
        if let Some(table_path) = self.tables.get_mut(index)
            && let Token::Identifier(p) = &mut self.tokens[self.token_ranges.table_files[index]]
        {
            *table_path = PathBuf::from(&new_path);
            *p = new_path.to_string();
        }
    }

    pub fn update_estimation_paths(
        &mut self,
        index: usize,
        new_file_path: Option<&str>,
        new_msfo_path: Option<&str>,
    ) {
        if let Some(estimation) = self.estimations.get_mut(index) {
            let (file_idx, msfo_idx) = self.token_ranges.estimations[index];

            if let Some(new_path) = new_file_path
                && let Some(idx) = file_idx
                && let Token::Identifier(p) = &mut self.tokens[idx]
            {
                *p = new_path.to_string();
                estimation.file = Some(PathBuf::from(&new_path));
            }
            if let Some(new_path) = new_msfo_path
                && let Some(idx) = msfo_idx
                && let Token::Identifier(p) = &mut self.tokens[idx]
            {
                *p = new_path.to_string();
                estimation.msfo = Some(PathBuf::from(&new_path));
            }
        }
    }

    pub fn update_problem_statement(&mut self, new_file_stem: &str, original_stem: &str) {
        let metadata_pattern = " created from pharos see ";
        let metadata_suffix = "_metadata.json for details.";
        let new_metadata_ref = format!("{}{}{}", metadata_pattern, new_file_stem, metadata_suffix);

        // Check if metadata reference already exists and update accordingly
        if self.problem.contains(metadata_pattern) {
            // Replace old stem with new stem in the problem statement
            self.problem = self.problem.replace(original_stem, new_file_stem);
        } else {
            // Add new metadata reference
            self.problem.push_str(&new_metadata_ref);
        }

        // Update the problem content token using the stored index
        if let Some(idx) = self.token_ranges.problem_content
            && let Token::Ignored(content) = &mut self.tokens[idx]
        {
            // Preserve the original formatting by extracting leading and trailing whitespace
            let leading_whitespace = content
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();

            let trailing_whitespace = content
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();

            let mut formatted_problem = String::new();
            formatted_problem.push_str(&leading_whitespace);
            formatted_problem.push_str(&self.problem);
            formatted_problem.push_str(&trailing_whitespace);

            *content = formatted_problem;
        }
    }

    pub fn copy(&self, original_filename: &str, new_filename: &str) -> AnyhowResult<Model> {
        let mut new_model = self.clone();

        // Extract stems from both filenames
        let original_stem = Path::new(original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original_filename);

        let new_stem = Path::new(new_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(new_filename);

        // If stems are the same, no replacement needed
        if original_stem == new_stem {
            return Ok(new_model);
        }

        // Update table paths
        let table_updates: Vec<_> = new_model
            .tables
            .iter()
            .enumerate()
            .filter_map(|(idx, table_path)| {
                let path = table_path.display().to_string();
                replace_stem_in_path(&path, original_stem, new_stem).map(|new_name| (idx, new_name))
            })
            .collect();

        for (idx, new_name) in table_updates {
            new_model.update_table_path(idx, &new_name);
        }

        // Update estimation paths
        let est_updates: Vec<_> = new_model
            .estimations
            .iter()
            .enumerate()
            .filter_map(|(idx, estimation)| {
                let new_file_path = estimation.file.as_ref().and_then(|file| {
                    let path = file.display().to_string();
                    replace_stem_in_path(&path, original_stem, new_stem)
                });

                let new_msfo_path = estimation.msfo.as_ref().and_then(|file| {
                    let path = file.display().to_string();
                    replace_stem_in_path(&path, original_stem, new_stem)
                });

                if new_file_path.is_some() || new_msfo_path.is_some() {
                    Some((idx, new_file_path, new_msfo_path))
                } else {
                    None
                }
            })
            .collect();

        for (idx, file_path, msfo_path) in est_updates {
            new_model.update_estimation_paths(idx, file_path.as_deref(), msfo_path.as_deref());
        }

        // Update the problem statement to reference the metadata file
        new_model.update_problem_statement(new_stem, original_stem);

        Ok(new_model)
    }

    pub fn theta_perturbation(
        &self,
        degree: f64,
        num_retries: usize,
        seed: Option<u64>,
    ) -> AnyhowResult<Vec<Model>> {
        if degree <= 0.0 || degree >= 1.0 {
            bail!("Degree must be between 0 and 1 (exclusive)");
        }
        let mut rng = if let Some(seed) = seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_os_rng()
        };

        let mut models = vec![];

        for _ in 0..num_retries {
            let mut new_model = self.clone();
            for (idx, param) in new_model.theta_parameters.iter_mut().enumerate() {
                if param.is_fixed {
                    continue;
                }

                let token_idx = new_model.token_ranges.theta_initial_values[idx];
                let original_str =
                    if let Token::Number { original, .. } = &new_model.tokens[token_idx] {
                        original.clone()
                    } else {
                        param.initial_value.to_string()
                    };

                let new_estimate = apply_jittering(
                    param.initial_value,
                    degree,
                    &mut rng,
                    param.lower_bound,
                    param.upper_bound,
                    &original_str,
                );
                param.initial_value = new_estimate;
                if let Token::Number { value, original } = &mut new_model.tokens[token_idx] {
                    *value = new_estimate;
                    *original = new_estimate.to_string();
                }
            }

            models.push(new_model);
        }

        Ok(models)
    }

    /// Canonicalizes the DATA path and replace any output path that would is relative to be
    /// just the filename. This ensures a run folder has all the data
    pub fn with_modified_paths(&self, dataset_path: &Path) -> String {
        let mut output = String::new();
        let paths_to_replace = self.paths_to_replace();

        for tok in &self.tokens {
            match tok {
                Token::Identifier(s) => {
                    if let Some(replacement) = paths_to_replace.get(s) {
                        output.push_str(replacement);
                    } else if s.as_str() == self.data.path {
                        output.push_str(&dataset_path.to_string_lossy());
                    } else {
                        output.push_str(s);
                    }
                }
                _ => output.push_str(tok.to_string().as_str()),
            }
        }
        output
    }

    pub fn model_content(&self) -> String {
        self.tokens.iter().map(|t| t.to_string()).collect()
    }

    /// Update initial parameter estimates from a .ext file
    /// You can control what is updated based on the given options struct, as well as whether
    /// to jitter the values.
    pub fn update_initial_estimates(&mut self, options: &CopyOptions) -> AnyhowResult<()> {
        let parameter_tables = if options.is_updating_params()
            && let Some(ext_path) = &options.ext_path
        {
            // Read parameter estimates from .ext file
            let ext_reader = ExtReader::default()
                .final_estimates_and_stderr_and_fixed()
                .only_last();
            // This should be a vec of length=1
            let parameter_tables =
                get_parameter_estimates(ext_path, &ext_reader, None, false, None)?;

            if parameter_tables.is_empty() {
                bail!("No parameter estimates found in {}", ext_path.display());
            }
            Some(parameter_tables)
        } else {
            None
        };

        let mut rng = if options.has_jittering() {
            Some(if let Some(seed) = options.seed {
                StdRng::seed_from_u64(seed)
            } else {
                StdRng::from_os_rng()
            })
        } else {
            None
        };

        let (update_thetas_from_ext, jitter) = options.theta_updates();
        if update_thetas_from_ext || jitter.is_some() {
            let self_theta_parameters: HashMap<_, _> = self
                .theta_parameters
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let theta_name = format!("THETA{}", i + 1);
                    (theta_name.clone(), p.initial_value)
                })
                .collect();

            for (i, theta_param) in self.theta_parameters.iter_mut().enumerate() {
                if theta_param.is_fixed {
                    continue;
                }
                let theta_name = format!("THETA{}", i + 1);

                let parameters: HashMap<_, _> = if let Some(parameter_tables) = &parameter_tables {
                    parameter_tables[0]
                        .theta
                        .iter()
                        .map(|x| (x.name.clone(), x.estimate))
                        .collect()
                } else {
                    self_theta_parameters.clone()
                };

                if let Some(estimate) = parameters.get(theta_name.as_str()) {
                    let mut final_value = *estimate;

                    // Only apply jittering if NOT excluded
                    if let (Some(jitter_pct), Some(rng_mut)) = (jitter, rng.as_mut())
                        && !options.excluded_parameters().contains(&theta_name)
                    {
                        let original_str =
                            match &self.tokens[self.token_ranges.theta_initial_values[i]] {
                                Token::Number { original, .. } => original.clone(),
                                _ => estimate.to_string(),
                            };

                        final_value = apply_jittering(
                            *estimate,
                            jitter_pct,
                            rng_mut,
                            theta_param.lower_bound,
                            theta_param.upper_bound,
                            &original_str,
                        );
                    }

                    // Always update the parameter (regardless of jitter exclusion)
                    theta_param.initial_value = final_value;
                    if let Token::Number { value, original } =
                        &mut self.tokens[self.token_ranges.theta_initial_values[i]]
                    {
                        let rounded = round_arbitrary_precision(original, final_value);
                        *value = rounded;
                        *original = rounded.to_string();
                    }
                }
            }
        }

        if options.omega_updates() {
            let parameters: Option<HashMap<_, _>> =
                parameter_tables.as_ref().map(|parameter_tables| {
                    parameter_tables[0]
                        .random_effects
                        .iter()
                        .filter(|x| x.is_omega())
                        .map(|x| (x.name.as_str(), x.estimate))
                        .collect()
                });

            update_parameter_blocks(
                &mut self.omega_blocks,
                &self.token_ranges.omega_initial_values,
                &mut self.tokens,
                parameters,
                "OMEGA",
                &options.excluded_parameters(),
                None,
                rng.as_mut(),
            );
        }

        if options.sigma_updates() {
            let parameters: Option<HashMap<_, _>> =
                parameter_tables.as_ref().map(|parameter_tables| {
                    parameter_tables[0]
                        .random_effects
                        .iter()
                        .filter(|x| x.is_sigma())
                        .map(|x| (x.name.as_str(), x.estimate))
                        .collect()
                });

            update_parameter_blocks(
                &mut self.sigma_blocks,
                &self.token_ranges.sigma_initial_values,
                &mut self.tokens,
                parameters,
                "SIGMA",
                &options.excluded_parameters(),
                None,
                rng.as_mut(),
            );
        }

        Ok(())
    }
}

/// Generic helper to iterate over parameter blocks in specified order
fn get_block_parameter_names<'a, T: ParamName>(
    blocks: &'a [ParameterBlock<T>],
    ordering: ParameterOrdering,
    param_prefix: &str,
    raneff_prefix: &str,
) -> AnyhowResult<Vec<(String, String, &'a Parameter<T>)>> {
    let mut results = Vec::new();
    let mut base_counter = 1;

    for (block_index, block) in blocks.iter().enumerate() {
        match &block.structure {
            BlockStructure::Diagonal => {
                for (param_idx, param) in block.parameters.iter().enumerate() {
                    let num = base_counter + param_idx;
                    let param_name = format!("{param_prefix}({num},{num})");
                    let raneff_label = format!("{raneff_prefix}{num}");
                    results.push((param_name, raneff_label, param));
                }
                base_counter += block.parameters.len();
            }
            BlockStructure::Block { size } | BlockStructure::BlockSame { size } => {
                // Determine which parameters to use
                let parameters = match &block.structure {
                    BlockStructure::Block { .. } => &block.parameters,
                    BlockStructure::BlockSame { .. } => {
                        // Find reference block - search backwards for most recent Block with matching size
                        let mut reference_block = None;
                        for i in (0..block_index).rev() {
                            if let BlockStructure::Block { size: ref_size } = &blocks[i].structure
                                && *ref_size == *size
                            {
                                reference_block = Some(&blocks[i]);
                                break;
                            }
                        }

                        let Some(ref_block) = reference_block else {
                            bail!(
                                "BlockSame {{size: {size}}} found but no previous Block {{size: {size}}} to reference"
                            )
                        };
                        &ref_block.parameters
                    }
                    _ => unreachable!(),
                };

                for (param_idx, (row, col)) in
                    ordering.get_coordinates(*size).into_iter().enumerate()
                {
                    if param_idx >= parameters.len() {
                        break;
                    }

                    let param = &parameters[param_idx];
                    let param_row = base_counter + row;
                    let param_col = base_counter + col;
                    let param_name = format!("{param_prefix}({param_row},{param_col})");
                    let raneff_label = if row == col {
                        format!("{raneff_prefix}{param_row}")
                    } else {
                        format!("{raneff_prefix}{param_col}:{raneff_prefix}{param_row}")
                    };
                    results.push((param_name, raneff_label, param));
                }
                base_counter += size;
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy::UpdateType;
    use fs_err as fs;
    use insta::{assert_debug_snapshot, assert_snapshot, glob};

    #[test]
    fn can_handle_errors() {
        glob!("../../test_data/parser_errors", "*.mod", |path| {
            let input = fs::read_to_string(path).unwrap();
            let err = Model::parse(&input).unwrap_err();
            assert_snapshot!(err);
        });
    }

    #[test]
    fn can_copy_model_and_replace_numbers() {
        let input = fs::read_to_string("test_data/parser/everything.mod").unwrap();
        let model = Model::parse(&input).unwrap();
        let model2 = model.copy("run001.mod", "run002.mod").unwrap();

        assert_snapshot!(model2.with_modified_paths(Path::new("test.csv")));
    }

    #[test]
    fn test_problem_statement_update() {
        let input = fs::read_to_string("test_data/parser/everything-prob.mod").unwrap();
        let model = Model::parse(&input).unwrap();

        // First copy: run001 -> run002
        let model2 = model.copy("run001.mod", "run002.mod").unwrap();
        assert_snapshot!(model2.with_modified_paths(Path::new("test.csv")));

        let model3 = model2.copy("run002.mod", "run003.mod").unwrap();
        assert_snapshot!(model3.with_modified_paths(Path::new("test.csv")));
    }

    #[test]
    fn test_update_initial_estimates_with_real_data() {
        let input = fs::read_to_string("models/BQL/bql.mod").unwrap();
        let mut model = Model::parse(&input).unwrap();
        let mut options = CopyOptions::default();
        options.ext_path = Some(PathBuf::from("test_data/ext/bql.ext"));
        options.update.push(UpdateType::All);
        model.update_initial_estimates(&options).unwrap();

        assert_snapshot!(model.model_content());
    }

    #[test]
    fn test_update_initial_estimates_with_real_data_w_jittering() {
        let input = fs::read_to_string("models/BQL/bql.mod").unwrap();
        let mut model = Model::parse(&input).unwrap();
        let mut options = CopyOptions::default();
        options.update.clear();
        options.update.push(UpdateType::Theta);
        options.update.push(UpdateType::Sigma);
        options.ext_path = Some(PathBuf::from("test_data/ext/bql.ext"));
        options.jitter = Some(0.2);
        options.seed = Some(42);
        model.update_initial_estimates(&options).unwrap();

        assert_snapshot!(model.model_content());
    }

    #[test]
    fn test_json_roundtrip() {
        let input = fs::read_to_string("test_data/parser/everything.mod").unwrap();
        let original_model = Model::parse(&input).unwrap();

        // Serialize to JSON
        let json = original_model.to_json().unwrap();

        // Deserialize back from JSON
        let roundtrip_model = Model::from_json(&json).unwrap();

        // They should be equal
        assert_eq!(original_model, roundtrip_model);
    }

    #[test]
    fn can_parse_comments_type1() {
        let input = fs::read_to_string("test_data/comments/type1.mod").unwrap();
        let mut model = Model::parse(&input).unwrap();
        let invalid = model.parse_comments(CommentType::Type1);
        assert!(invalid.is_empty());
        assert_debug_snapshot!(model);
    }

    #[test]
    fn test_jitter_excluded_parameters_scenarios() {
        glob!("../../test_data/run_output", "**/*.mod", |mod_path| {
            let model_name = mod_path.file_stem().unwrap().to_string_lossy();

            // Find corresponding .ext file
            let ext_path = mod_path.with_extension("ext");
            if !ext_path.exists() {
                panic!("No .ext file found for {}", model_name);
            }

            let test_scenarios = vec![
                (
                    "theta_update_with_exclusion",
                    CopyOptions {
                        update: vec![UpdateType::Theta],
                        jitter: Some(0.2),
                        jitter_excluded: Some("THETA1".to_string()),
                        seed: Some(42),
                        ext_path: Some(ext_path.clone()),
                        ..Default::default()
                    },
                ),
                (
                    "all_update_with_multiple_exclusions",
                    CopyOptions {
                        update: vec![UpdateType::All],
                        jitter: Some(0.15),
                        jitter_excluded: Some("THETA1,OMEGA(2,2)".to_string()),
                        seed: Some(42),
                        ext_path: Some(ext_path.clone()),
                        ..Default::default()
                    },
                ),
                (
                    "omega_update_with_exclusion",
                    CopyOptions {
                        update: vec![UpdateType::Omega],
                        jitter: None,
                        jitter_excluded: Some("OMEGA(1,1)".to_string()),
                        seed: Some(42),
                        ext_path: Some(ext_path.clone()),
                        ..Default::default()
                    },
                ),
                (
                    "mixed_update_no_exclusions_baseline",
                    CopyOptions {
                        update: vec![UpdateType::Theta, UpdateType::Omega],
                        jitter: Some(0.2),
                        jitter_excluded: None,
                        seed: Some(804),
                        ext_path: Some(ext_path.clone()),
                        ..Default::default()
                    },
                ),
            ];

            for (scenario_name, options) in test_scenarios {
                let input = fs::read_to_string(mod_path).unwrap();
                let mut model = Model::parse(&input).unwrap();

                model.update_initial_estimates(&options).unwrap();

                // Snapshot name: model_scenario
                let snapshot_name = format!("{}_{}", model_name, scenario_name);
                assert_snapshot!(snapshot_name, model.model_content());
            }
        });
    }

    #[test]
    fn can_do_theta_perturbation() {
        let input = fs::read_to_string("test_data/parser/multiline_table.mod").unwrap();
        let model = Model::parse(&input).unwrap();
        let retries = model.theta_perturbation(0.1, 3, Some(42)).unwrap();
        let params = retries
            .iter()
            .map(|x| x.with_modified_paths(Path::new("/home/vincent/dataset.csv")))
            .collect::<Vec<_>>();
        assert_debug_snapshot!(params);
    }

    #[test]
    fn can_do_theta_perturbation_extended() {
        let input = fs::read_to_string("test_data/parser/theta_extended.mod").unwrap();
        let model = Model::parse(&input).unwrap();
        let retries = model.theta_perturbation(0.1, 3, Some(42)).unwrap();
        let params = retries
            .iter()
            .map(|x| x.with_modified_paths(Path::new("/home/vincent/dataset.csv")))
            .collect::<Vec<_>>();
        assert_debug_snapshot!(params);
    }
}
