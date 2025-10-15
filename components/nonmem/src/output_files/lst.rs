//! Parses .lst output file

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

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

/// RunDetails contains key information about logistics of the model run
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RunDetails {
    pub problem: String,
    pub number_data_records: usize,
    pub number_subjects: usize,
    pub number_obs: usize,
    pub ofv: Vec<Option<f64>>,
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

fn parse_ofv_value(line: &str, estimation_methods: &[String]) -> Option<f64> {
    if let Some(current_method) = estimation_methods.last()
        && current_method.contains("Stochastic Approximation Expectation-Maximization")
    {
        return None;
    }

    // unwrapping here becuase if we find this line we should always get a number
    SIGNED_NUMBER_RE
        .find(line)
        .map(|captures| captures.as_str().trim().parse::<f64>().unwrap())
}

fn parse_run_details(content: &str) -> RunDetails {
    let mut run_details = RunDetails::default();
    let mut n_log_2pi_line_found = false;

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
        } else if line.contains("N*LOG(2PI) CONSTANT TO OBJECTIVE FUNCTION:") {
            n_log_2pi_line_found = true;
        } else if line.contains("OBJECTIVE FUNCTION VALUE WITHOUT CONSTANT:")
            && n_log_2pi_line_found
        {
            let ofv = parse_ofv_value(line, &run_details.estimation_methods);
            run_details.ofv.push(ofv);
            n_log_2pi_line_found = false;
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

fn parse_run_heuristics(content: &str) -> RunHeuristics {
    let mut run_heuristics = RunHeuristics::default();

    for line in content.lines() {
        if line.contains("0MINIMIZATION TERMINATED") {
            run_heuristics.minimization_terminated = Some(true);
        } else if line.contains("RESET HESSIAN") {
            run_heuristics.hessian_reset = Some(true);
        } else if line.contains("PARAMETER ESTIMATE IS NEAR ITS BOUNDARY") {
            run_heuristics.parameter_near_boundary = Some(true);
        } else if line.contains("COVARIANCE STEP ABORTED") {
            run_heuristics.covariance_step_aborted = Some(true);
        } else if line.contains("Forcing positive definiteness") {
            run_heuristics.covariance_step_aborted = Some(true);
        }
    }

    run_heuristics
}

pub fn parse_lst(content: &str) -> LstSummary {
    // This way we read the file multiple times but it's tiny and easier to understand for the dev
    let run_heuristics = parse_run_heuristics(content);
    let run_details = parse_run_details(content);

    LstSummary {
        run_details,
        run_heuristics,
    }
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
            assert_debug_snapshot!(parse_lst(&input));
        });
    }
}
