use std::collections::HashMap;
use std::path::Path;

use crate::ast::Subroutine;
use crate::cst::CstChild;
use crate::lexer::Token;
use crate::model::Model;

/// Extract the filename from a path string.
fn flatten_path(path: &str) -> String {
    let p = Path::new(path);
    p.file_name()
        .unwrap_or(p.as_os_str())
        .to_string_lossy()
        .to_string()
}

fn replace_stem_in_path(path: &str, original_stem: &str, new_stem: &str) -> Option<String> {
    if !path.contains(original_stem) {
        return None;
    }
    Some(path.replace(original_stem, new_stem))
}

impl Model {
    pub fn paths_to_replace(&self) -> HashMap<String, String> {
        let mut output = HashMap::new();
        let mut paths: Vec<&str> = vec![];

        for est in &self.estimations {
            if let Some(p) = &est.msfo {
                paths.push(p.to_str().unwrap_or_default());
            }
            if let Some(p) = &est.file {
                paths.push(p.to_str().unwrap_or_default());
            }
        }
        for table in &self.tables {
            if let Some(f) = &table.file {
                paths.push(f);
            }
        }
        if let Some(subs) = &self.subroutines {
            for sub in &subs.entries {
                if let Subroutine::Other { path, .. } = sub {
                    paths.push(path);
                }
            }
        }

        for p in paths {
            let path = Path::new(p);
            let filename = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .to_string();
            output.insert(p.to_string(), filename);
        }

        output
    }

    pub fn model_content(&self) -> String {
        self.cst.text(&self.tokens)
    }

    pub fn with_modified_paths(&self, dataset_path: &Path) -> String {
        let mut replacements: HashMap<usize, String> = HashMap::new();

        // Data path
        if let Some(idx) = self.data.path_idx {
            replacements.insert(idx, dataset_path.to_string_lossy().to_string());
        }

        // Table FILE= paths
        for table in &self.tables {
            if let (Some(idx), Some(file)) = (table.file_idx, &table.file) {
                replacements.insert(idx, flatten_path(file));
            }
        }

        // Estimation FILE= and MSFO= paths
        for est in &self.estimations {
            if let (Some(idx), Some(file)) = (est.file_idx, &est.file) {
                replacements.insert(idx, flatten_path(&file.to_string_lossy()));
            }
            if let (Some(idx), Some(msfo)) = (est.msfo_idx, &est.msfo) {
                replacements.insert(idx, flatten_path(&msfo.to_string_lossy()));
            }
        }

        // Subroutine OTHER= paths
        if let Some(subs) = &self.subroutines {
            for sub in &subs.entries {
                if let Subroutine::Other { path, path_idx } = sub {
                    replacements.insert(*path_idx, flatten_path(path));
                }
            }
        }

        self.cst.text_with_replacements(&self.tokens, &replacements)
    }

    pub fn copy(&self, original_filename: &str, new_filename: &str) -> Model {
        let mut new_model = self.clone();

        let original_stem = Path::new(original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original_filename);

        let new_stem = Path::new(new_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(new_filename);

        if original_stem == new_stem {
            return new_model;
        }

        // Update table paths
        let table_updates: Vec<_> = new_model
            .tables
            .iter()
            .enumerate()
            .filter_map(|(idx, table)| {
                let path = table.file.as_ref()?;
                replace_stem_in_path(path, original_stem, new_stem).map(|new_name| (idx, new_name))
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
            .filter_map(|(idx, est)| {
                let new_file = est.file.as_ref().and_then(|f| {
                    replace_stem_in_path(&f.to_string_lossy(), original_stem, new_stem)
                });
                let new_msfo = est.msfo.as_ref().and_then(|f| {
                    replace_stem_in_path(&f.to_string_lossy(), original_stem, new_stem)
                });
                if new_file.is_some() || new_msfo.is_some() {
                    Some((idx, new_file, new_msfo))
                } else {
                    None
                }
            })
            .collect();

        for (idx, file_path, msfo_path) in est_updates {
            new_model.update_estimation_paths(idx, file_path.as_deref(), msfo_path.as_deref());
        }

        new_model.update_problem_statement(new_stem, original_stem);

        new_model
    }

    pub fn update_table_path(&mut self, index: usize, new_path: &str) {
        if let Some(table) = self.tables.get_mut(index) {
            if let Some(idx) = table.file_idx {
                self.tokens[idx].text = new_path.to_string();
            }
            table.file = Some(new_path.to_string());
        }
    }

    pub fn update_estimation_paths(
        &mut self,
        index: usize,
        new_file_path: Option<&str>,
        new_msfo_path: Option<&str>,
    ) {
        if let Some(est) = self.estimations.get_mut(index) {
            if let Some(new_path) = new_file_path {
                if let Some(idx) = est.file_idx {
                    self.tokens[idx].text = new_path.to_string();
                }
                est.file = Some(new_path.into());
            }
            if let Some(new_path) = new_msfo_path {
                if let Some(idx) = est.msfo_idx {
                    self.tokens[idx].text = new_path.to_string();
                }
                est.msfo = Some(new_path.into());
            }
        }
    }

    pub fn update_problem_statement(&mut self, new_stem: &str, original_stem: &str) {
        let metadata_pattern = " created from pharos see ";
        let metadata_suffix = "_metadata.json for details.";
        let new_metadata_ref = format!("{metadata_pattern}{new_stem}{metadata_suffix}");

        if self.problem.text.contains(metadata_pattern) {
            self.problem.text = self.problem.text.replace(original_stem, new_stem);
        } else {
            self.problem.text.push_str(&new_metadata_ref);
        }

        // Update the CST tokens for the problem record
        let CstChild::Node(node) = &self.cst.children[self.problem.record_idx] else {
            return;
        };

        // Collect content token indices: everything after the $PROBLEM keyword, excluding
        // trailing newline. We'll set the first content token to the new text and blank the rest.
        let mut content_indices = Vec::new();
        let mut found_keyword = false;
        for child in &node.children {
            match child {
                CstChild::Token(idx) => {
                    if !found_keyword {
                        // The first token is the $PROBLEM keyword
                        found_keyword = true;
                        continue;
                    }
                    // Skip trailing newline
                    if self.tokens[*idx].token == Token::Newline {
                        continue;
                    }
                    content_indices.push(*idx);
                }
                _ => {}
            }
        }

        if content_indices.is_empty() {
            return;
        }

        // Reconstruct the problem text with original leading whitespace
        let leading_ws: String = self.tokens[content_indices[0]]
            .text
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        self.tokens[content_indices[0]].text = format!("{leading_ws}{}", self.problem.text);

        for &idx in &content_indices[1..] {
            self.tokens[idx].text = String::new();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_model(input: &str) -> Model {
        let (model, diagnostics) = Model::parse(input).unwrap();
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        model
    }

    #[test]
    fn model_content_round_trips() {
        let inputs = ["everything.mod", "nmexample.mod", "ar1mod.mod"];
        for name in inputs {
            let path = format!("{}/test_data/{name}", env!("CARGO_MANIFEST_DIR"));
            let input = fs_err::read_to_string(&path).unwrap();
            let model = parse_model(&input);
            assert_eq!(model.model_content(), input, "round-trip failed for {name}");
        }
    }

    #[test]
    fn copy_replaces_stems() {
        let input = "\
$PROBLEM test run001
$INPUT ID TIME DV
$DATA data.csv
$THETA 1.5
$EST METHOD=0 FILE=run001.ext MSFO=run001.msf
$TABLE ID TIME FILE=run001.tab
";
        let model = parse_model(input);
        let copied = model.copy("run001.mod", "run002.mod");

        assert!(copied.problem.text.contains("run002"));
        assert_eq!(copied.tables[0].file.as_deref(), Some("run002.tab"));
        assert_eq!(
            copied.estimations[0].file.as_ref().unwrap().to_str(),
            Some("run002.ext")
        );
        assert_eq!(
            copied.estimations[0].msfo.as_ref().unwrap().to_str(),
            Some("run002.msf")
        );

        let content = copied.model_content();
        assert!(content.contains("run002.tab"));
        assert!(content.contains("run002.ext"));
        assert!(content.contains("run002.msf"));
        assert!(!content.contains("run001.tab"));
        assert!(!content.contains("run001.ext"));
        assert!(!content.contains("run001.msf"));
    }

    #[test]
    fn copy_same_stem_is_noop() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA data.csv
$THETA 1
";
        let model = parse_model(input);
        let copied = model.copy("run001.mod", "run001.mod");
        assert_eq!(copied.model_content(), input);
    }

    #[test]
    fn with_modified_paths_replaces_data_and_flattens_outputs() {
        let input = "\
$PROBLEM test
$INPUT ID
$DATA ../data/file.csv
$THETA 1
$EST METHOD=0 FILE=../output/run001.ext
$TABLE ID FILE=../output/run001.tab
";
        let model = parse_model(input);
        let modified = model.with_modified_paths(Path::new("/absolute/data/file.csv"));

        assert!(modified.contains("/absolute/data/file.csv"));
        assert!(modified.contains("FILE=run001.ext"));
        assert!(modified.contains("FILE=run001.tab"));
        assert!(!modified.contains("../output/"));
        assert!(!modified.contains("../data/"));
    }

    #[test]
    fn update_problem_statement_appends_metadata() {
        let input = "\
$PROBLEM my cool model
$INPUT ID
$DATA data.csv
$THETA 1
";
        let mut model = parse_model(input);
        model.update_problem_statement("run002", "run001");
        assert_eq!(
            model.problem.text,
            "my cool model created from pharos see run002_metadata.json for details."
        );
        let content = model.model_content();
        let problem = content.trim().split('\n').next().unwrap();
        assert_eq!(
            problem,
            "$PROBLEM my cool model created from pharos see run002_metadata.json for details."
        );
    }
}
