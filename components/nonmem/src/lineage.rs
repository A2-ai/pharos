use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use config::{find_config_dir, to_root_relative};
use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::model_metadata::{METADATA_FILENAME_SUFFIX, ModelMetadata};
use crate::model_resolution::ModelLayout;
use crate::run::metadata::{
    RUN_END_FILENAME, RunEndFile, RunStartFile, SKIP_DIRS, walk_run_start_files,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineageTree {
    pub nodes: HashMap<String, ModelMetadata>,
    pub metadata: HashMap<String, (RunStartFile, Option<RunEndFile>)>,
    /// Canonical project root the tree was built against. Used by
    /// path-based queries (e.g. `model_identity_for`) to strip prefixes
    /// without consulting global config state. Not serialized: a path is
    /// machine-local and meaningless on the consumer side. A deserialized
    /// tree therefore supports only identity-based queries (string keys),
    /// not path-based ones.
    #[serde(skip)]
    project_root: PathBuf,
}

enum Direction {
    Descendants,
    Ancestors,
}

impl LineageTree {
    /// Build a LineageTree by recursively scanning the current pharos
    /// project (located via `find_config_dir`). Each model is keyed by its
    /// project-relative path with forward slashes (e.g.
    /// `"model/nonmem/struct/1001.mod"`).
    pub fn from_project() -> Result<Self> {
        let project_root =
            fs::canonicalize(find_config_dir()?.ok_or_else(|| {
                anyhow!("No pharos.toml found in this directory or any parent.")
            })?)?;
        Self::from_project_root(project_root)
    }

    pub fn from_project_root(project_root: PathBuf) -> Result<Self> {
        let mut tree = Self {
            project_root,
            ..Default::default()
        };
        let root = tree.project_root.clone();
        tree.extend_model_nodes(&root, &root)?;
        tree.load_run_metadata(&root)?;
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

            let Some(layout) = ModelLayout::try_locate(&base_name, dir)? else {
                continue;
            };
            let key = to_root_relative(layout.model_path(), project_root)?;

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
    fn load_run_metadata(&mut self, project_root: &Path) -> Result<()> {
        for (dir, run_start) in walk_run_start_files(project_root)? {
            let Ok(key) = to_root_relative(&run_start.model_canonical_path, project_root) else {
                continue;
            };
            if !self.nodes.contains_key(&key) {
                continue;
            }

            // Only use the latest timestamp if we have several models with timestamps in them.
            // They are UTC so sortable
            if let Some((existing, _)) = self.metadata.get(&key)
                && existing.start >= run_start.start
            {
                continue;
            }

            let end_path = dir.join(RUN_END_FILENAME);
            let run_end = if end_path.exists() {
                match RunEndFile::load(&end_path) {
                    Ok(re) => Some(re),
                    Err(e) => {
                        // Visible without --verbose (see load note above).
                        eprintln!(
                            "Warning: failed to load {}: {e}; treating run as incomplete",
                            end_path.display()
                        );
                        None
                    }
                }
            } else {
                None
            };
            self.metadata.insert(key, (run_start, run_end));
        }
        Ok(())
    }

    pub fn topological_order(
        &self,
        nodes: HashSet<String>,
    ) -> Result<Vec<(String, ModelMetadata)>> {
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

        let mut processed = 0usize;
        while let Some(Reverse(current)) = heap.pop() {
            processed += 1;
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

        // Any node never reaching in-degree 0 sits in (or downstream of) a
        // `based_on` cycle. Report it instead of silently dropping it.
        if processed < nodes.len() {
            let mut cyclic: Vec<String> = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(node, _)| node)
                .collect();
            cyclic.sort();
            bail!(
                "based_on cycle detected in model lineage involving: {}",
                cyclic.join(", ")
            );
        }

        Ok(result)
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
        self.topological_order(visited)
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
        self.topological_order(set)
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
        let key = to_root_relative(&canonical, &self.project_root)?;
        if !self.nodes.contains_key(&key) {
            bail!(
                "'{}' has no metadata; lineage requires a *_metadata.json next to the model file",
                input.display()
            );
        }
        Ok(key)
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
    fn test_slice_descendants() {
        let test_tree = create_test_tree();
        let diamond = create_diamond_tree();
        let cases: &[(&str, &LineageTree, &str, &[&str])] = &[
            (
                "linear_from_root",
                &test_tree,
                "a/base.mod",
                &["a/base.mod", "a/model1.mod", "a/model2.mod"],
            ),
            (
                "linear_from_leaf",
                &test_tree,
                "a/model2.mod",
                &["a/model2.mod"],
            ),
            (
                "linear_from_middle",
                &test_tree,
                "a/model1.mod",
                &["a/model1.mod", "a/model2.mod"],
            ),
            (
                "diamond_from_root",
                &diamond,
                "a/base.mod",
                &[
                    "a/base.mod",
                    "a/branch1.mod",
                    "a/branch2.mod",
                    "a/final.mod",
                ],
            ),
            (
                "diamond_from_branch",
                &diamond,
                "a/branch1.mod",
                &["a/branch1.mod", "a/final.mod"],
            ),
        ];
        for (name, tree, from, expected) in cases {
            let result = tree.slice(Some(*from), Option::<&str>::None).unwrap();
            let got: Vec<&str> = result.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(&got[..], *expected, "case: {name}");
        }
    }

    #[test]
    fn test_slice_descendants_errors_on_unknown_input() {
        let test_tree = create_test_tree();
        let empty = LineageTree::default();
        let cases: &[(&str, &LineageTree, &str)] = &[
            ("nonexistent_in_filled_tree", &test_tree, "nonexistent"),
            ("empty_input_in_filled_tree", &test_tree, ""),
            ("any_query_in_empty_tree", &empty, "any"),
        ];
        for (name, tree, from) in cases {
            assert!(
                tree.slice(Some(*from), Option::<&str>::None).is_err(),
                "case: {name}"
            );
        }
    }

    #[test]
    fn test_slice_ancestors() {
        let test_tree = create_test_tree();
        let diamond = create_diamond_tree();
        let cases: &[(&str, &LineageTree, &str, &[&str])] = &[
            (
                "linear_to_leaf",
                &test_tree,
                "a/model2.mod",
                &["a/base.mod", "a/model1.mod", "a/model2.mod"],
            ),
            ("linear_to_root", &test_tree, "a/base.mod", &["a/base.mod"]),
            (
                "linear_to_middle",
                &test_tree,
                "a/model1.mod",
                &["a/base.mod", "a/model1.mod"],
            ),
            (
                "diamond_to_final",
                &diamond,
                "a/final.mod",
                &[
                    "a/base.mod",
                    "a/branch1.mod",
                    "a/branch2.mod",
                    "a/final.mod",
                ],
            ),
            (
                "diamond_to_branch",
                &diamond,
                "a/branch1.mod",
                &["a/base.mod", "a/branch1.mod"],
            ),
        ];
        for (name, tree, to, expected) in cases {
            let result = tree.slice(Option::<&str>::None, Some(*to)).unwrap();
            let got: Vec<&str> = result.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(&got[..], *expected, "case: {name}");
        }
    }

    #[test]
    fn test_slice_ancestors_errors_on_unknown_input() {
        let test_tree = create_test_tree();
        let empty = LineageTree::default();
        let cases: &[(&str, &LineageTree, &str)] = &[
            ("nonexistent_in_filled_tree", &test_tree, "nonexistent"),
            ("empty_input_in_filled_tree", &test_tree, ""),
            ("any_query_in_empty_tree", &empty, "any"),
        ];
        for (name, tree, to) in cases {
            assert!(
                tree.slice(Option::<&str>::None, Some(*to)).is_err(),
                "case: {name}"
            );
        }
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
        let result = tree.topological_order(nodes).unwrap();
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
        let result = tree.topological_order(all).unwrap();
        assert_models_in_order(&result, &["a", "b", "aaa", "zzz"]);
    }

    #[test]
    fn test_topological_order_reports_cycle() {
        // a -> b -> a is a based_on cycle; neither node can reach in-degree 0,
        // so it must be reported rather than silently dropped from the output.
        let tree = tree_from_deps(&[("a", &["b"]), ("b", &["a"])]);
        let all: HashSet<String> = tree.nodes.keys().cloned().collect();
        let err = tree.topological_order(all).unwrap_err().to_string();
        assert!(err.contains("cycle"), "unexpected error: {err}");
        assert!(err.contains('a') && err.contains('b'), "error: {err}");
    }

    #[test]
    fn test_slice_between() {
        // Slice between m1 and m2 returns descendants(m1) ∩ ancestors(m2),
        // so siblings of m1 and descendants of m2 fall out.
        let chain = tree_from_deps(&[
            ("base.mod", &[]),
            ("a.mod", &["base.mod"]),
            ("b.mod", &["base.mod"]),
            ("a-cov.mod", &["a.mod"]),
            ("a-leaf.mod", &["a-cov.mod"]),
        ]);
        let disjoint = tree_from_deps(&[("a.mod", &[]), ("b.mod", &[])]);
        let test_tree = create_test_tree();

        let cases: &[(&str, &LineageTree, &str, &str, &[&str])] = &[
            (
                "basic_chain",
                &chain,
                "a.mod",
                "a-cov.mod",
                &["a.mod", "a-cov.mod"],
            ),
            ("disjoint_branches_empty", &disjoint, "a.mod", "b.mod", &[]),
            (
                "same_model_both_sides",
                &test_tree,
                "a/model1.mod",
                "a/model1.mod",
                &["a/model1.mod"],
            ),
        ];
        for (name, tree, from, to, expected) in cases {
            let result = tree.slice(Some(*from), Some(*to)).unwrap();
            let got: Vec<&str> = result.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(&got[..], *expected, "case: {name}");
        }
    }

    #[test]
    fn test_lineage_of() {
        let tree = tree_from_deps(&[
            ("root.mod", &[]),
            ("mid.mod", &["root.mod"]),
            ("leaf.mod", &["mid.mod"]),
            ("unrelated.mod", &[]),
        ]);

        let chain = tree.lineage_of("mid.mod").unwrap();
        assert_models_in_order(&chain, &["root.mod", "mid.mod", "leaf.mod"]);
    }

    fn setup_project(deps: &[(&str, &[&str])]) -> (tempfile::TempDir, LineageTree) {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs_err::canonicalize(tmp.path()).unwrap();

        for (rel_path, parents) in deps {
            let full = root.join(rel_path);
            fs_err::create_dir_all(full.parent().unwrap()).unwrap();
            fs_err::write(&full, "dummy").unwrap();
            let stem = full.file_stem().unwrap().to_string_lossy().to_string();
            let dir = full.parent().unwrap();
            let metadata = ModelMetadata {
                based_on: parents.iter().map(|s| s.to_string()).collect(),
                copied_from: String::new(),
                description: format!("{rel_path} model"),
                tags: vec![],
            };
            metadata.save(&stem, dir).unwrap();
        }

        let tree = LineageTree::from_project_root(root).unwrap();
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
    fn test_model_identity_for_existing() {
        let (tmp, tree) = setup_project(&[("model/base.mod", &[])]);
        let file = tmp.path().join("model/base.mod");
        let id = tree.model_identity_for(&file).unwrap();
        assert_eq!(id, "model/base.mod");
    }

    #[test]
    fn test_model_identity_for_no_metadata() {
        let (tmp, tree) = setup_project(&[("model/base.mod", &[])]);
        let unregistered = tmp.path().join("model/other.mod");
        fs_err::write(&unregistered, "dummy").unwrap();
        let err = tree
            .model_identity_for(&unregistered)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no metadata"), "{err}");
    }

    #[test]
    fn test_model_identity_for_not_found() {
        let (tmp, tree) = setup_project(&[("model/base.mod", &[])]);
        let missing = tmp.path().join("nonexistent.mod");
        let err = tree.model_identity_for(&missing).unwrap_err().to_string();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn test_model_identity_for_outside_root() {
        let (_tmp, tree) = setup_project(&[("model/base.mod", &[])]);
        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join("foo.mod");
        fs_err::write(&file, "dummy").unwrap();
        let err = tree.model_identity_for(&file).unwrap_err().to_string();
        assert!(err.contains("outside the project root"), "{err}");
    }

    #[test]
    fn test_from_project_skips_noise_dirs() {
        let (tmp, _tree) = setup_project(&[("model/base.mod", &[])]);
        let root = fs_err::canonicalize(tmp.path()).unwrap();

        // Plant a model + metadata pair inside each skip dir. They should
        // all be ignored.
        for skip in SKIP_DIRS {
            let noise_dir = root.join(skip).join("nested");
            fs_err::create_dir_all(&noise_dir).unwrap();
            let model = noise_dir.join("hidden.mod");
            fs_err::write(&model, "dummy").unwrap();
            ModelMetadata {
                based_on: vec![],
                copied_from: String::new(),
                description: format!("hidden in {skip}"),
                tags: vec![],
            }
            .save("hidden", &noise_dir)
            .unwrap();
        }

        let tree = LineageTree::from_project_root(root).unwrap();
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes.contains_key("model/base.mod"));
    }
}
