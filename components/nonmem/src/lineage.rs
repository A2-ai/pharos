use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use config::to_config_relative;
use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::model_metadata::{METADATA_FILENAME_SUFFIX, ModelMetadata};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME, RunEndFile, RunStartFile};

/// Directory names skipped by the project walkers. These never contain
/// pharos model or run files and are common enough that walking through
/// them on every `lineage` invocation is wasteful.
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "build", "dist"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineageTree {
    pub nodes: HashMap<String, ModelMetadata>,
    pub metadata: HashMap<String, (RunStartFile, Option<RunEndFile>)>,
}

enum Direction {
    Descendants,
    Ancestors,
}

impl LineageTree {
    /// Build a LineageTree by recursively scanning the project rooted at `project_root`.
    /// Each model is keyed by its project-relative path with forward slashes
    /// (e.g. `"model/nonmem/struct/1001.mod"`).
    pub fn from_project(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = fs::canonicalize(project_root.as_ref())?;
        let mut tree = Self::default();
        tree.extend_model_nodes(&project_root, &project_root)?;
        tree.load_run_metadata(&project_root, &project_root)?;
        Ok(tree)
    }

    /// Recursively walk `dir`. For each `<stem>_metadata.json` file found,
    /// look for a sibling `<stem>.mod` or `<stem>.ctl`; if one exists,
    /// register the model in `self.nodes`. The key is the model file's
    /// project-relative path with forward slashes, so identities are stable
    /// across platforms.
    fn extend_model_nodes(&mut self, project_root: &Path, dir: &Path) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                self.extend_model_nodes(project_root, &entry.path())?;
                continue;
            }

            if !file_type.is_file() || !name.ends_with(METADATA_FILENAME_SUFFIX) {
                continue;
            }

            let base_name = name
                .strip_suffix(METADATA_FILENAME_SUFFIX)
                .unwrap()
                .to_string();
            let ext = if dir.join(format!("{base_name}.mod")).exists() {
                "mod"
            } else if dir.join(format!("{base_name}.ctl")).exists() {
                "ctl"
            } else {
                continue;
            };

            let model_file = dir.join(format!("{base_name}.{ext}"));
            let key = model_file
                .strip_prefix(project_root)
                .map(path_to_forward_slash)
                .map_err(|_| {
                    anyhow!(
                        "model file {} is outside project root",
                        model_file.display()
                    )
                })?;

            let metadata = ModelMetadata::load(entry.path())?;
            self.nodes.insert(key, metadata);
        }
        Ok(())
    }

    /// Recursively walk `dir`. For each `pharos_start.json` file found,
    /// look up the model it belongs to via the file's stored
    /// `model_canonical_path`; if that model is already registered in
    /// `self.nodes`, record the run in `self.metadata` (along with the
    /// optional sibling `pharos_end.json`). Run-output directories can
    /// live anywhere under `project_root` — the canonical model path
    /// inside each start file is what associates the run with its model,
    /// so any user-configured `output_dir` template is honored.
    ///
    /// `project_root` must already be canonical; otherwise the strip-prefix
    /// check against `model_canonical_path` silently fails for every entry
    /// and no run metadata is loaded.
    fn load_run_metadata(&mut self, project_root: &Path, dir: &Path) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                self.load_run_metadata(project_root, &entry.path())?;
                continue;
            }

            if !file_type.is_file() || name != RUN_START_FILENAME {
                continue;
            }

            let start_path = entry.path();
            let run_start = RunStartFile::load(&start_path)?;
            let Ok(rel) = run_start.model_canonical_path.strip_prefix(project_root) else {
                continue;
            };
            let key = path_to_forward_slash(rel);
            if !self.nodes.contains_key(&key) {
                continue;
            }

            let run_dir = start_path.parent().unwrap_or(dir);
            let end_path = run_dir.join(RUN_END_FILENAME);
            let run_end = if end_path.exists() {
                Some(RunEndFile::load(end_path)?)
            } else {
                None
            };
            self.metadata.insert(key, (run_start, run_end));
        }
        Ok(())
    }

    pub fn topological_order(&self, nodes: HashSet<String>) -> Vec<(String, ModelMetadata)> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for node in &nodes {
            in_degree.insert(node.clone(), 0);
        }

        for node in &nodes {
            if let Some(meta) = self.nodes.get(node) {
                for parent in &meta.based_on {
                    if nodes.contains(parent) {
                        *in_degree.get_mut(node).unwrap() += 1;
                    }
                }
            }
        }

        // Use a min-heap so ties are broken strictly alphabetically across the
        // entire ready set, not just per batch.
        let mut heap: BinaryHeap<Reverse<String>> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(node, _)| Reverse(node.clone()))
            .collect();

        while let Some(Reverse(current)) = heap.pop() {
            if let Some(meta) = self.nodes.get(&current) {
                result.push((current.clone(), meta.clone()));
            }

            // Decrement the in-degree of every child still in the input set.
            // We do this even when `current` is not in `self.nodes` so that
            // orphan parents don't strand their descendants.
            for node in &nodes {
                if let Some(node_meta) = self.nodes.get(node)
                    && node_meta.based_on.contains(&current)
                {
                    let degree = in_degree.get_mut(node).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        heap.push(Reverse(node.clone()));
                    }
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

    /// Returns the full lineage of the model at `input`: the union of its
    /// ancestors and descendants (plus the model itself), topo-sorted.
    pub fn lineage_of(&self, input: impl AsRef<Path>) -> Result<Vec<(String, ModelMetadata)>> {
        let id = self.model_identity_for(input)?;
        let mut visited = self.reachable(&id, Direction::Descendants);
        visited.extend(self.reachable(&id, Direction::Ancestors));
        Ok(self.topological_order(visited))
    }

    /// Topo-sorted slice of the tree.
    ///
    /// - `slice(None, None)` returns every model in the project.
    /// - `slice(Some(m), None)` returns m and its descendants.
    /// - `slice(None, Some(m))` returns m and its ancestors.
    /// - `slice(Some(f), Some(t))` returns descendants(f) ∩ ancestors(t).
    pub fn slice<F: AsRef<Path>, T: AsRef<Path>>(
        &self,
        from: Option<F>,
        to: Option<T>,
    ) -> Result<Vec<(String, ModelMetadata)>> {
        let from_id = from.map(|p| self.model_identity_for(p)).transpose()?;
        let to_id = to.map(|p| self.model_identity_for(p)).transpose()?;

        let descendants = match from_id.as_deref() {
            Some(f) => self.reachable(f, Direction::Descendants),
            None => self.nodes.keys().cloned().collect(),
        };
        let ancestors = match to_id.as_deref() {
            Some(t) => self.reachable(t, Direction::Ancestors),
            None => self.nodes.keys().cloned().collect(),
        };
        let set: HashSet<String> = descendants.intersection(&ancestors).cloned().collect();
        Ok(self.topological_order(set))
    }

    fn reachable(&self, start: &str, direction: Direction) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![start.to_string()];
        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            match direction {
                Direction::Descendants => {
                    for (child_name, child_meta) in &self.nodes {
                        if child_meta.based_on.contains(&current) && !visited.contains(child_name) {
                            to_visit.push(child_name.clone());
                        }
                    }
                }
                Direction::Ancestors => {
                    if let Some(meta) = self.nodes.get(&current) {
                        for parent in &meta.based_on {
                            if !visited.contains(parent) {
                                to_visit.push(parent.clone());
                            }
                        }
                    }
                }
            }
        }
        visited
    }

    /// Resolve `input` to a tree key.
    ///
    /// If `input` is already a known key (its string representation matches
    /// an entry in `self.nodes`), it's returned directly without filesystem
    /// I/O — this lets tests query synthetic trees with bare key strings.
    /// Otherwise `input` is treated as a filesystem path, canonicalized,
    /// stripped against the project root, and validated against `self.nodes`.
    /// Errors if the file resolves outside the project root or if no
    /// metadata is registered for it.
    fn model_identity_for(&self, input: impl AsRef<Path>) -> Result<String> {
        let input = input.as_ref();
        let s = input.to_string_lossy().into_owned();
        if self.nodes.contains_key(&s) {
            return Ok(s);
        }
        if !input.exists() {
            bail!("model file not found: {}", input.display());
        }
        let canonical = fs::canonicalize(input)?;
        let key = to_config_relative(&canonical)?;
        if !self.nodes.contains_key(&key) {
            bail!(
                "'{}' has no metadata; lineage requires a *_metadata.json next to the model file",
                input.display()
            );
        }
        Ok(key)
    }
}

/// Convert a `Path` to a forward-slash string (for use as a map key).
fn path_to_forward_slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
                    copied_from: String::new(),
                    description,
                    tags,
                },
            );
        }
        tree
    }

    fn create_test_tree() -> LineageTree {
        tree_from_deps(&[
            ("a/base.mod", &[]),
            ("a/model1.mod", &["a/base.mod"]),
            ("a/model2.mod", &["a/model1.mod"]),
        ])
    }

    fn create_diamond_tree() -> LineageTree {
        tree_from_deps(&[
            ("a/base.mod", &[]),
            ("a/branch1.mod", &["a/base.mod"]),
            ("a/branch2.mod", &["a/base.mod"]),
            ("a/final.mod", &["a/branch1.mod", "a/branch2.mod"]),
        ])
    }

    fn assert_models_in_order(result: &[(String, ModelMetadata)], expected: &[&str]) {
        let names: Vec<&str> = result.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn test_get_tree_from_basic() {
        let tree = create_test_tree();
        let result = tree
            .slice(Some("a/base.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_models_in_order(&result, &["a/base.mod", "a/model1.mod", "a/model2.mod"]);
    }

    #[test]
    fn test_get_tree_from_positions() {
        let tree = create_test_tree();

        let leaf_result = tree
            .slice(Some("a/model2.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(leaf_result.len(), 1);
        assert_eq!(leaf_result[0].0, "a/model2.mod");

        let middle_result = tree
            .slice(Some("a/model1.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(middle_result.len(), 2);
        assert_models_in_order(&middle_result, &["a/model1.mod", "a/model2.mod"]);
    }

    #[test]
    fn test_get_tree_from_diamond() {
        let tree = create_diamond_tree();
        let result = tree
            .slice(Some("a/base.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(result.len(), 4);
        assert_models_in_order(
            &result,
            &[
                "a/base.mod",
                "a/branch1.mod",
                "a/branch2.mod",
                "a/final.mod",
            ],
        );

        let branch_result = tree
            .slice(Some("a/branch1.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(branch_result.len(), 2);
        assert_models_in_order(&branch_result, &["a/branch1.mod", "a/final.mod"]);
    }

    #[test]
    fn test_get_tree_from_edge_cases() {
        let tree = create_test_tree();
        // Keys not in the tree and not on disk produce an error under the new API.
        assert!(
            tree.slice(Some("nonexistent"), Option::<&str>::None)
                .is_err()
        );
        assert!(tree.slice(Some(""), Option::<&str>::None).is_err());

        let empty_tree = LineageTree::default();
        assert!(empty_tree.slice(Some("any"), Option::<&str>::None).is_err());
    }

    #[test]
    fn test_get_tree_up_to_basic() {
        let tree = create_test_tree();
        let result = tree
            .slice(Option::<&str>::None, Some("a/model2.mod"))
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_models_in_order(&result, &["a/base.mod", "a/model1.mod", "a/model2.mod"]);
    }

    #[test]
    fn test_get_tree_up_to_positions() {
        let tree = create_test_tree();

        let root_result = tree
            .slice(Option::<&str>::None, Some("a/base.mod"))
            .unwrap();
        assert_eq!(root_result.len(), 1);
        assert_eq!(root_result[0].0, "a/base.mod");

        let middle_result = tree
            .slice(Option::<&str>::None, Some("a/model1.mod"))
            .unwrap();
        assert_eq!(middle_result.len(), 2);
        assert_models_in_order(&middle_result, &["a/base.mod", "a/model1.mod"]);
    }

    #[test]
    fn test_get_tree_up_to_diamond() {
        let tree = create_diamond_tree();
        let result = tree
            .slice(Option::<&str>::None, Some("a/final.mod"))
            .unwrap();
        assert_eq!(result.len(), 4);
        assert_models_in_order(
            &result,
            &[
                "a/base.mod",
                "a/branch1.mod",
                "a/branch2.mod",
                "a/final.mod",
            ],
        );

        let branch_result = tree
            .slice(Option::<&str>::None, Some("a/branch1.mod"))
            .unwrap();
        assert_eq!(branch_result.len(), 2);
        assert_models_in_order(&branch_result, &["a/base.mod", "a/branch1.mod"]);
    }

    #[test]
    fn test_get_tree_up_to_edge_cases() {
        let tree = create_test_tree();
        // Keys not in the tree and not on disk produce an error under the new API.
        assert!(
            tree.slice(Option::<&str>::None, Some("nonexistent"))
                .is_err()
        );
        assert!(tree.slice(Option::<&str>::None, Some("")).is_err());

        let empty_tree = LineageTree::default();
        assert!(empty_tree.slice(Option::<&str>::None, Some("any")).is_err());
    }

    /// If the input set contains a key that has no entry in `self.nodes`
    /// (e.g. a stale identity passed in by a caller) it must still
    /// decrement its descendants' in-degrees so they don't strand at the
    /// top of the heap. Without that, `child` here would never be emitted.
    #[test]
    fn test_topological_order_tolerates_orphan_in_input() {
        let tree = tree_from_deps(&[("child", &["orphan"])]);
        let mut nodes = HashSet::new();
        nodes.insert("orphan".to_string());
        nodes.insert("child".to_string());
        let result = tree.topological_order(nodes);
        // `orphan` has no metadata, so it is not emitted; `child` must still
        // appear.
        assert_models_in_order(&result, &["child"]);
    }

    /// Demonstrates that ties are broken strictly alphabetically across levels,
    /// not just within a single batch. With a FIFO VecDeque the order would be
    /// a, b, zzz, aaa; with the min-heap it must be a, b, aaa, zzz.
    #[test]
    fn test_topological_order_cross_level_tie_breaking() {
        let tree = tree_from_deps(&[("a", &[]), ("b", &[]), ("zzz", &["a"]), ("aaa", &["b"])]);
        let all: HashSet<String> = tree.nodes.keys().cloned().collect();
        let result = tree.topological_order(all);
        assert_models_in_order(&result, &["a", "b", "aaa", "zzz"]);
    }

    fn setup_project(deps: &[(&str, &[&str])]) -> (TempDir, LineageTree) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let metadata_map: HashMap<&str, ModelMetadata> = deps
            .iter()
            .map(|(name, parents)| {
                let based_on = parents.iter().map(|s| s.to_string()).collect();
                (
                    *name,
                    ModelMetadata {
                        based_on,
                        copied_from: String::new(),
                        description: format!("{name} model"),
                        tags: vec![],
                    },
                )
            })
            .collect();

        for (rel_path, metadata) in &metadata_map {
            let full = root.join(rel_path);
            fs_err::create_dir_all(full.parent().unwrap()).unwrap();
            // Create the .mod file.
            fs_err::write(&full, "dummy").unwrap();
            // Save metadata next to it.
            let stem = full.file_stem().unwrap().to_string_lossy().to_string();
            let dir = full.parent().unwrap();
            metadata.save(&stem, dir).unwrap();
        }

        let tree = LineageTree::from_project(root).unwrap();
        (tmp, tree)
    }

    #[test]
    fn test_from_project_basic() {
        let (_tmp, tree) = setup_project(&[
            ("model/nonmem/base/base.mod", &[]),
            (
                "model/nonmem/struct/model1.mod",
                &["model/nonmem/base/base.mod"],
            ),
            (
                "model/nonmem/struct/cov/model2.mod",
                &["model/nonmem/struct/model1.mod"],
            ),
        ]);

        assert_eq!(tree.nodes.len(), 3);
        assert!(tree.nodes.contains_key("model/nonmem/base/base.mod"));
        assert!(tree.nodes.contains_key("model/nonmem/struct/model1.mod"));
        assert!(
            tree.nodes
                .contains_key("model/nonmem/struct/cov/model2.mod")
        );

        let result = tree
            .slice(Some("model/nonmem/base/base.mod"), Option::<&str>::None)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_models_in_order(
            &result,
            &[
                "model/nonmem/base/base.mod",
                "model/nonmem/struct/model1.mod",
                "model/nonmem/struct/cov/model2.mod",
            ],
        );
    }

    #[test]
    fn test_get_tree_between_basic() {
        let tree = tree_from_deps(&[
            ("base.mod", &[]),
            ("a.mod", &["base.mod"]),
            ("b.mod", &["base.mod"]),
            ("a-cov.mod", &["a.mod"]),
            ("a-leaf.mod", &["a-cov.mod"]),
        ]);

        // Slice from `a.mod` to `a-cov.mod` includes only those two; `b.mod`
        // is not a descendant of `a.mod`, and `a-leaf.mod` is not an
        // ancestor of `a-cov.mod`.
        let slice = tree.slice(Some("a.mod"), Some("a-cov.mod")).unwrap();
        assert_models_in_order(&slice, &["a.mod", "a-cov.mod"]);
    }

    #[test]
    fn test_get_tree_between_disjoint() {
        let tree = tree_from_deps(&[("a.mod", &[]), ("b.mod", &[])]);

        // `a.mod` is not an ancestor of `b.mod`; the slice is empty.
        let slice = tree.slice(Some("a.mod"), Some("b.mod")).unwrap();
        assert!(slice.is_empty());
    }

    #[test]
    fn test_get_tree_between_same_model() {
        let tree = create_test_tree();
        let slice = tree
            .slice(Some("a/model1.mod"), Some("a/model1.mod"))
            .unwrap();
        assert_models_in_order(&slice, &["a/model1.mod"]);
    }

    #[test]
    fn test_full() {
        let tree = tree_from_deps(&[
            ("root.mod", &[]),
            ("mid.mod", &["root.mod"]),
            ("leaf.mod", &["mid.mod"]),
            ("unrelated.mod", &[]),
        ]);

        let chain = tree.lineage_of("mid.mod").unwrap();
        assert_models_in_order(&chain, &["root.mod", "mid.mod", "leaf.mod"]);
    }
}
