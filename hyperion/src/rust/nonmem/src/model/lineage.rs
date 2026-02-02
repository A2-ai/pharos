use extendr_api::Result;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use nonmem::{LineageTree, ModelMetadata, OutputFileHash, RunEndFile, RunStartFile};

use crate::utils::{path_from_robj, to_config_relative};
use hyperion_core::{OptionExt, ResultExt};

/// R-compatible version of RunEndFile with u128 -> f64 conversion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RRunEndFile {
    pub start: String,
    pub end: String,
    pub runtime_ms: f64, // Changed from u128 to f64 for R compatibility
    pub files_copied: HashSet<String>,
    pub output_files_rewrites: HashMap<String, String>,
    pub output_files_hashes: Vec<OutputFileHash>,
}

/// R-compatible version of LineageTree
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RLineageTree {
    pub nodes: HashMap<String, ModelMetadata>,
    pub metadata: HashMap<String, (RunStartFile, Option<RRunEndFile>)>,
    pub source_dir: String,
}

impl RLineageTree {
    /// Set the source directory for this lineage tree
    pub fn with_source_dir(mut self, source_dir: String) -> Self {
        self.source_dir = source_dir;
        self
    }
}

impl From<RunEndFile> for RRunEndFile {
    fn from(run_end: RunEndFile) -> Self {
        RRunEndFile {
            start: run_end.start,
            end: run_end.end,
            runtime_ms: run_end.runtime_ms as f64, // Convert u128 to f64
            files_copied: run_end.files_copied,
            output_files_rewrites: run_end.output_files_rewrites,
            output_files_hashes: run_end.output_files_hashes,
        }
    }
}

impl From<LineageTree> for RLineageTree {
    fn from(lineage: LineageTree) -> Self {
        let r_metadata = lineage
            .metadata
            .into_iter()
            .map(|(key, (start_file, opt_end_file))| {
                let r_end_file = opt_end_file.map(|end_file| end_file.into());
                (key, (start_file, r_end_file))
            })
            .collect();

        RLineageTree {
            nodes: lineage.nodes,
            metadata: r_metadata,
            source_dir: String::new(), // Set by caller via with_source_dir()
        }
    }
}

/// Get's model lineage
///
/// @param model_dir path to directory containing all models, or a hyperion_nonmem_model object
/// (uses the model's parent directory)
///
/// @return hyperion_nonmem_tree S3 object
/// @export
///
/// @examples \dontrun{
/// get_model_lineage("model/nonmem/")
/// model <- read_model("model/nonmem/run001.mod")
/// get_model_lineage(model)
/// }
#[extendr]
pub fn get_model_lineage(model_dir: Robj) -> Result<Robj> {
    let path = path_from_robj(&model_dir, false)?;
    // If it's a file, use parent directory; if directory, use as-is
    let model_dir = if path.is_file() {
        path.parent()
            .ok_or_extendr_err("Could not determine model directory")?
            .to_path_buf()
    } else {
        path
    };

    // Create lineage tree from folder
    let lineage = LineageTree::from_folder(&model_dir)
        .map_to_extendr_err("Pharos failed to create lineage tree")?;

    // Convert to R-compatible version (u128 -> f64) and attach source directory (relative to pharos.toml)
    let source_dir = to_config_relative(&model_dir)?;
    let r_lineage: RLineageTree = RLineageTree::from(lineage).with_source_dir(source_dir);

    // Serialize R-compatible lineage to Robj
    let mut lineage_robj =
        to_robj(&r_lineage).map_to_extendr_err("Failed to create Robj from RLineageTree")?;

    // Set S3 class
    let hyperion_nonmem_tree = lineage_robj
        .set_class(["hyperion_nonmem_tree"])
        .map_to_extendr_err("Failed to set class")?;

    Ok(hyperion_nonmem_tree.to_owned())
}

extendr_module! {
    mod lineage;

    fn get_model_lineage;
}
