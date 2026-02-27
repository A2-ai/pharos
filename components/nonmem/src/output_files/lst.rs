//! Parses .lst output file
use anyhow::Result as AnyhowResult;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::Model;

static PROBLEM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\$PROB(?:LEM)?\s+").unwrap());
static SIGNED_NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[-+]?(\d*\.\d+|\d+)\s*$").unwrap());
static LAST_NUMBER_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:\.\d+)?)$").unwrap());

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RunHeuristics {
    pub covariance_step_aborted: Option<bool>,
    pub eigenvalue_issues: Option<bool>,
    pub parameter_near_boundary: Option<bool>,
    pub hessian_reset: Option<bool>,
    pub minimization_terminated: Option<bool>,
}

impl RunHeuristics {
    /// Create heuristics with `Some(false)` defaults for checks that are
    /// applicable given the model structure. Inapplicable checks stay `None`.
    pub fn from_model(model: &Model) -> Self {
        let mut h = Self::default();

        let is_sim_only = model.simulation.as_ref().is_some_and(|s| s.is_only_sim());
        let has_estimation = !model.estimations.is_empty() && !is_sim_only;

        if model.covariance.is_some() {
            h.covariance_step_aborted = Some(false);
        }
        if has_estimation {
            h.minimization_terminated = Some(false);
            h.hessian_reset = Some(false);
            h.parameter_near_boundary = Some(false);
        }

        h
    }
}

/// RunDetails contains key information about logistics of the model run
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RunDetails {
    pub problem: String,
    pub number_data_records: usize,
    pub number_subjects: usize,
    pub number_obs: usize,
    pub estimation_time: Vec<f64>,
    pub covariance_time: Vec<f64>,
    pub postprocess_time: f64,
    pub estimation_methods: Vec<String>,
    pub function_evaluations: usize,
    pub significant_digits: usize,
    pub only_sim: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LstSummary {
    pub run_details: RunDetails,
    pub run_heuristics: RunHeuristics,
}

fn parse_timing(line: &str) -> f64 {
    if let Some(captures) = SIGNED_NUMBER_RE.find(line) {
        captures.as_str().parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    }
}

fn parse_last_int(line: &str) -> usize {
    if let Some(captures) = LAST_NUMBER_LINE_RE.captures(line) {
        captures[1].parse::<usize>().unwrap_or(0)
    } else {
        0
    }
}

fn parse_run_details(content: &str) -> RunDetails {
    let mut run_details = RunDetails::default();

    for line in content.lines() {
        if line.contains("NO. OF DATA RECS IN DATA SET:") {
            run_details.number_data_records = parse_last_int(line);
        } else if line.contains("TOT. NO. OF DATA RECS:") && run_details.number_data_records == 0 {
            // https://github.com/metrumresearchgroup/bbi/issues/227
            run_details.number_data_records = parse_last_int(line);
        } else if line.contains("TOT. NO. OF OBS RECS:") {
            run_details.number_obs = parse_last_int(line);
        } else if line.contains("TOT. NO. OF INDIVIDUALS:") {
            run_details.number_subjects = parse_last_int(line);
        } else if line.contains("NO. OF FUNCTION EVALUATIONS USED") {
            run_details.function_evaluations = parse_last_int(line);
        } else if line.contains("NO. OF SIG. DIGITS IN FINAL EST.:") {
            run_details.significant_digits = parse_last_int(line);
        } else if line.contains("Elapsed estimation") {
            run_details.estimation_time.push(parse_timing(line));
        } else if line.contains("Elapsed covariance") {
            run_details.covariance_time.push(parse_timing(line));
        } else if line.contains("Elapsed postprocess") {
            run_details.postprocess_time = parse_timing(line);
        } else if line.contains("#METH:") {
            run_details
                .estimation_methods
                .push(line.replace("#METH:", "").trim().to_string());
        } else if line.starts_with("$SIM") {
            // we could have ONLY in comments
            if line.split(';').next().unwrap().contains("ONLY") {
                run_details.only_sim = true;
            }
        } else if PROBLEM_RE.is_match(line) {
            run_details.problem = PROBLEM_RE.replace(line, "").to_string();
        }
    }

    run_details
}

fn parse_run_heuristics(content: &str) -> AnyhowResult<RunHeuristics> {
    let model = extract_model_from_contents(content)?;
    let mut run_heuristics = RunHeuristics::from_model(&model);

    for line in content.lines() {
        if line.contains("0MINIMIZATION TERMINATED") {
            run_heuristics.minimization_terminated = Some(true);
        } else if line.contains("RESET HESSIAN") {
            run_heuristics.hessian_reset = Some(true);
        } else if line.contains("PARAMETER ESTIMATE IS NEAR ITS BOUNDARY") {
            run_heuristics.parameter_near_boundary = Some(true);
        } else if line.contains("COVARIANCE STEP ABORTED")
            || line.contains("Forcing positive definiteness")
        {
            run_heuristics.covariance_step_aborted = Some(true);
        }
    }

    Ok(run_heuristics)
}

pub fn parse_lst(content: &str) -> AnyhowResult<LstSummary> {
    let run_heuristics = parse_run_heuristics(content)?;
    let run_details = parse_run_details(content);

    Ok(LstSummary {
        run_details,
        run_heuristics,
    })
}

pub fn extract_model_from_contents(contents: &str) -> AnyhowResult<Model> {
    // lst starts with timestamp, then model content, then NM-TRAN MESSAGES
    let model_content = contents
        .lines()
        .skip(1)
        .take_while(|line| *line != "NM-TRAN MESSAGES")
        .collect::<Vec<_>>()
        .join("\n");

    let model = Model::parse(&model_content)?;
    Ok(model)
}

pub fn extract_model(path: impl AsRef<Path>) -> AnyhowResult<Model> {
    let contents = fs::read_to_string(path)?;
    let model = extract_model_from_contents(&contents)?;

    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_err as fs;
    use insta::{assert_debug_snapshot, glob};

    #[test]
    fn can_parse_lst() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/lst");
        glob!(test_dir, "*.lst", |path| {
            let input = fs::read_to_string(path).unwrap();
            assert_debug_snapshot!(parse_lst(&input).unwrap())
        });
    }

    #[test]
    fn can_extract_model() {
        use std::path::PathBuf;
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/lst");
        // simple extraction of models from all lst files
        glob!(&test_dir, "*.lst", |path| {
            extract_model(path).unwrap();
        });
    }

    #[test]
    fn extracted_model_matches_input_model() {
        use std::path::PathBuf;
        let test_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/run_output/run003");

        let lst_file = test_dir.join("run003.lst");
        let mod_file = test_dir.join("run003.mod");
        let mod_contents = fs::read_to_string(mod_file).unwrap();

        let lst_model = extract_model(lst_file).unwrap();
        let mod_model = Model::parse(&mod_contents).unwrap();

        assert_eq!(lst_model.model_content(), mod_model.model_content());
    }
}
