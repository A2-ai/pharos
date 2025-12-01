use anyhow::{Result, bail};
use regex::Regex;
use std::path::PathBuf;
use std::sync::LazyLock;

static MODEL_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[(\d+):(\d+)\](.*)$").unwrap());

/// Expands a model pattern like "run[001:003].mod" into individual model paths
pub fn expand_model_pattern(pattern: &str) -> Result<Vec<PathBuf>> {
    // If there's no brackets, that's a normal path
    if !pattern.contains('[') || !pattern.contains(']') {
        return Ok(vec![PathBuf::from(pattern)]);
    }

    // Parse pattern like "run[001:003].mod"
    let caps = MODEL_PATTERN_RE
        .captures(pattern)
        .ok_or_else(|| anyhow::anyhow!("Invalid pattern format: {}", pattern))?;

    let prefix = caps.get(1).unwrap().as_str();
    let start_str = caps.get(2).unwrap().as_str();
    let end_str = caps.get(3).unwrap().as_str();
    let suffix = caps.get(4).unwrap().as_str();

    let start: u32 = start_str.parse()?;
    let end: u32 = end_str.parse()?;

    if start > end {
        bail!(
            "Start number {} cannot be greater than end number {}",
            start,
            end
        );
    }

    // Determine zero-padding from the original numbers
    let width = start_str.len().max(end_str.len());

    let mut models = Vec::new();
    for i in start..=end {
        let filename = format!("{prefix}{i:0width$}{suffix}");
        models.push(PathBuf::from(filename));
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_expand_patterns() {
        let test_cases = vec![
            // Single file cases
            ("run001.mod", Ok(vec!["run001.mod"])),
            // Simple range cases
            ("run[1:3].mod", Ok(vec!["run1.mod", "run2.mod", "run3.mod"])),
            // Zero-padded range cases
            (
                "run[001:003].mod",
                Ok(vec!["run001.mod", "run002.mod", "run003.mod"]),
            ),
            // Mixed padding cases
            (
                "run[01:003].mod",
                Ok(vec!["run001.mod", "run002.mod", "run003.mod"]),
            ),
            // No extension cases
            ("model[1:2]", Ok(vec!["model1", "model2"])),
            // Path with directory cases
            (
                "models/run[01:02].mod",
                Ok(vec!["models/run01.mod", "models/run02.mod"]),
            ),
            // Error cases
            ("run[3:1].mod", Err("Invalid range")),
            ("run[invalid].mod", Err("Invalid pattern")),
        ];

        for (pattern, expected) in test_cases {
            let result = expand_model_pattern(pattern);

            match expected {
                Ok(expected_paths) => {
                    let result = result.unwrap_or_else(|e| {
                        panic!("Pattern '{}' should succeed but failed: {}", pattern, e)
                    });
                    let expected: Vec<PathBuf> =
                        expected_paths.into_iter().map(PathBuf::from).collect();
                    assert_eq!(
                        result, expected,
                        "Pattern '{}' produced incorrect result",
                        pattern
                    );
                }
                Err(_) => {
                    assert!(
                        result.is_err(),
                        "Pattern '{}' should fail but succeeded with: {:?}",
                        pattern,
                        result
                    );
                }
            }
        }
    }
}
