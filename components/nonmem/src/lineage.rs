use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::model_metadata::{METADATA_FILENAME_SUFFIX, ModelMetadata};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME, RunEndFile, RunStartFile};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineageTree {
    pub nodes: HashMap<String, ModelMetadata>,
    pub metadata: HashMap<String, (RunStartFile, Option<RunEndFile>)>,
}

impl LineageTree {
    pub fn from_folder(folder: impl AsRef<Path>) -> Result<Self> {
        let folder = folder.as_ref();

        let mut nodes = HashMap::new();
        let mut metadata_files: HashMap<PathBuf, (PathBuf, Option<PathBuf>)> = HashMap::new();

        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(METADATA_FILENAME_SUFFIX)
            {
                let path = entry.path();
                let file_stem = path.file_stem().unwrap().to_string_lossy();
                // Remove the "_metadata" suffix to get the base model name
                let base_name = file_stem
                    .strip_suffix("_metadata")
                    .unwrap_or(&file_stem)
                    .to_string();

                // Look for corresponding .mod or .ctl file in the same folder
                let mod_file = folder.join(format!("{}.mod", base_name));
                let ctl_file = folder.join(format!("{}.ctl", base_name));

                let actual_model_name = if mod_file.exists() {
                    format!("{}.mod", base_name)
                } else if ctl_file.exists() {
                    format!("{}.ctl", base_name)
                } else {
                    // No corresponding model file found, skip this metadata file
                    continue;
                };

                nodes.insert(actual_model_name, ModelMetadata::load(&path)?);
                continue;
            }

            // look for any dir and try to find the files in it. Use the dirname as key for the hashmap
            if entry.file_type()?.is_dir() {
                let dir_path = entry.path();
                let mut paths = (None, None);

                // Search within the directory for pharos JSON files
                if let Ok(dir_entries) = fs::read_dir(&dir_path) {
                    for dir_entry in dir_entries {
                        if let Ok(dir_entry) = dir_entry
                            && dir_entry
                                .file_type()
                                .map(|ft| ft.is_file())
                                .unwrap_or(false)
                        {
                            let file_name = dir_entry.file_name().to_string_lossy().to_string();
                            let file_path = dir_entry.path();
                            match file_name.as_str() {
                                RUN_START_FILENAME => paths.0 = Some(file_path),
                                RUN_END_FILENAME => paths.1 = Some(file_path),
                                _ => {} // Ignore other files
                            }
                        }
                    }
                }

                if paths.0.is_some() {
                    metadata_files.insert(dir_path, (paths.0.unwrap(), paths.1));
                }
            }
        }

        // Once we have found all metadata paths, we try to load them and store them in a hashmap
        // keyed by the model name
        let mut metadata = HashMap::new();

        for (run_start_p, run_end_p) in metadata_files.values() {
            let run_start = RunStartFile::load(run_start_p)?;
            let run_end = if let Some(run_end_p) = run_end_p {
                Some(RunEndFile::load(run_end_p)?)
            } else {
                None
            };
            let possible_names = vec![
                format!("{}.mod", run_start.model_name),
                format!("{}.ctl", run_start.model_name),
            ];
            for name in possible_names {
                if nodes.contains_key(&name) {
                    metadata.insert(name, (run_start, run_end));
                    break;
                }
            }
        }

        Ok(Self { nodes, metadata })
    }

    pub fn topological_order(&self, nodes: HashSet<String>) -> Vec<(String, ModelMetadata)> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Calculate in-degree for each node (how many parents it has within our set)
        for node in &nodes {
            in_degree.insert(node.clone(), 0);
        }

        for node in &nodes {
            if let Some(metadata) = self.nodes.get(node) {
                for parent in &metadata.based_on {
                    if nodes.contains(parent) {
                        *in_degree.get_mut(node).unwrap() += 1;
                    }
                }
            }
        }

        let mut initial_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(node, _)| node.clone())
            .collect();
        initial_nodes.sort();
        let mut queue: VecDeque<String> = initial_nodes.into();

        // Process nodes level by level
        while let Some(current) = queue.pop_front() {
            if let Some(metadata) = self.nodes.get(&current) {
                result.push((current.clone(), metadata.clone()));

                // Decrease in-degree for all children
                let mut nodes_to_add = Vec::new();
                for node in &nodes {
                    if let Some(node_metadata) = self.nodes.get(node)
                        && node_metadata.based_on.contains(&current)
                    {
                        let degree = in_degree.get_mut(node).unwrap();
                        *degree -= 1;
                        if *degree == 0 {
                            nodes_to_add.push(node.clone());
                        }
                    }
                }

                // Sort nodes that reached in-degree 0 and add them to queue
                nodes_to_add.sort();
                for node in nodes_to_add {
                    queue.push_back(node);
                }
            }
        }

        result
    }

    pub fn get_metadata_for(
        &self,
        model_name: &str,
    ) -> Option<&(RunStartFile, Option<RunEndFile>)> {
        self.metadata.get(model_name)
    }

    pub fn get_all_models_in_order(&self) -> Vec<(String, ModelMetadata)> {
        self.topological_order(self.nodes.keys().cloned().collect())
    }

    pub fn get_tree_from(&self, model_name: &str) -> Vec<(String, ModelMetadata)> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![model_name.to_string()];
        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            for (child_name, child_meta) in &self.nodes {
                if child_meta.based_on.contains(&current) && !visited.contains(child_name) {
                    to_visit.push(child_name.clone());
                }
            }
        }

        self.topological_order(visited)
    }

    pub fn get_tree_up_to(&self, model_name: &str) -> Vec<(String, ModelMetadata)> {
        let mut to_visit = vec![model_name.to_string()];
        let mut visited = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(metadata) = self.nodes.get(&current) {
                for parent in &metadata.based_on {
                    if !visited.contains(parent) {
                        to_visit.push(parent.clone());
                    }
                }
            }
        }
        self.topological_order(visited)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_from_deps(deps: &[(&str, &[&str])]) -> LineageTree {
        let mut tree = LineageTree::default();
        for (name, parents) in deps {
            let based_on = parents.iter().map(|s| s.to_string()).collect();
            let description = format!("{} model", name);
            let tags = vec![name.to_string()];
            tree.nodes.insert(
                name.to_string(),
                ModelMetadata {
                    based_on,
                    description,
                    tags,
                },
            );
        }
        tree
    }

    fn create_test_tree() -> LineageTree {
        tree_from_deps(&[
            ("base", &[]),
            ("model1", &["base"]),
            ("model2", &["model1"]),
        ])
    }

    fn create_diamond_tree() -> LineageTree {
        tree_from_deps(&[
            ("base", &[]),
            ("branch1", &["base"]),
            ("branch2", &["base"]),
            ("final", &["branch1", "branch2"]),
        ])
    }

    fn assert_models_in_order(result: &[(String, ModelMetadata)], expected: &[&str]) {
        let names: Vec<&str> = result.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn test_get_tree_from_basic() {
        let tree = create_test_tree();
        let result = tree.get_tree_from("base");
        assert_eq!(result.len(), 3);
        assert_models_in_order(&result, &["base", "model1", "model2"]);
    }

    #[test]
    fn test_get_tree_from_positions() {
        let tree = create_test_tree();

        let leaf_result = tree.get_tree_from("model2");
        assert_eq!(leaf_result.len(), 1);
        assert_eq!(leaf_result[0].0, "model2");

        let middle_result = tree.get_tree_from("model1");
        assert_eq!(middle_result.len(), 2);
        assert_models_in_order(&middle_result, &["model1", "model2"]);
    }

    #[test]
    fn test_get_tree_from_diamond() {
        let tree = create_diamond_tree();
        let result = tree.get_tree_from("base");
        assert_eq!(result.len(), 4);
        assert_models_in_order(&result, &["base", "branch1", "branch2", "final"]);

        let branch_result = tree.get_tree_from("branch1");
        assert_eq!(branch_result.len(), 2);
        assert_models_in_order(&branch_result, &["branch1", "final"]);
    }

    #[test]
    fn test_get_tree_from_edge_cases() {
        let tree = create_test_tree();
        assert!(tree.get_tree_from("nonexistent").is_empty());
        assert!(tree.get_tree_from("").is_empty());

        let empty_tree = LineageTree::default();
        assert!(empty_tree.get_tree_from("any").is_empty());
    }

    #[test]
    fn test_get_tree_up_to_basic() {
        let tree = create_test_tree();
        let result = tree.get_tree_up_to("model2");
        assert_eq!(result.len(), 3);
        assert_models_in_order(&result, &["base", "model1", "model2"]);
    }

    #[test]
    fn test_get_tree_up_to_positions() {
        let tree = create_test_tree();

        let root_result = tree.get_tree_up_to("base");
        assert_eq!(root_result.len(), 1);
        assert_eq!(root_result[0].0, "base");

        let middle_result = tree.get_tree_up_to("model1");
        assert_eq!(middle_result.len(), 2);
        assert_models_in_order(&middle_result, &["base", "model1"]);
    }

    #[test]
    fn test_get_tree_up_to_diamond() {
        let tree = create_diamond_tree();
        let result = tree.get_tree_up_to("final");
        assert_eq!(result.len(), 4);
        assert_models_in_order(&result, &["base", "branch1", "branch2", "final"]);

        let branch_result = tree.get_tree_up_to("branch1");
        assert_eq!(branch_result.len(), 2);
        assert_models_in_order(&branch_result, &["base", "branch1"]);
    }

    #[test]
    fn test_get_tree_up_to_edge_cases() {
        let tree = create_test_tree();
        assert!(tree.get_tree_up_to("nonexistent").is_empty());
        assert!(tree.get_tree_up_to("").is_empty());

        let empty_tree = LineageTree::default();
        assert!(empty_tree.get_tree_up_to("any").is_empty());
    }

    #[test]
    fn test_from_folder() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a test tree with the correct .mod-style references
        let test_tree = tree_from_deps(&[
            ("base.mod", &[]),
            ("model1.mod", &["base.mod"]),
            ("model2.mod", &["model1.mod"]),
        ]);

        for (name, metadata) in &test_tree.nodes {
            // Extract base name from the full model name (e.g., "base.mod" -> "base")
            let base_name = name.strip_suffix(".mod").unwrap_or(name);
            metadata.save(base_name, temp_dir.path()).unwrap();
            // Create dummy model files that the from_folder method expects
            let model_file_path = temp_dir.path().join(name);
            fs_err::write(model_file_path, "dummy model content").unwrap();
        }

        let loaded = LineageTree::from_folder(temp_dir.path()).unwrap();
        assert_eq!(loaded.nodes.len(), 3);

        for (name, original_meta) in &test_tree.nodes {
            let loaded_meta = &loaded.nodes[name];
            assert_eq!(loaded_meta.based_on, original_meta.based_on);
            assert_eq!(loaded_meta.description, original_meta.description);
        }

        let result = loaded.get_tree_from("base.mod");
        assert_eq!(result.len(), 3);
        assert_models_in_order(&result, &["base.mod", "model1.mod", "model2.mod"]);
    }
}
