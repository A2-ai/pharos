use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use nonmem::{LineageTree, ModelMetadata, OutputFileHash, RunEndFile, RunStartFile};

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
        }
    }
}

/// Get's model lineage
///
/// @param model_dir path to directory containing all models
///
/// @return hyperion_tree S3 object
/// @export
///
/// @examples \dontrun{
/// get_model_lineage("model/nonmem/")
/// }
#[extendr]
pub fn get_model_lineage(model_dir: &str) -> Result<Robj> {
    // Create lineage tree from folder
    let lineage = LineageTree::from_folder(model_dir)
        .map_err(|e| Error::Other(format!("Failed to create lineage tree: {e}")))?;

    // Convert to R-compatible version (u128 -> f64)
    let r_lineage: RLineageTree = lineage.into();

    // Serialize R-compatible lineage to Robj
    let mut lineage_robj = to_robj(&r_lineage)
        .map_err(|e| Error::Other(format!("Failed to create Robj from RLineageTree: {e}")))?;

    // Set S3 class
    let hyperion_tree = lineage_robj
        .set_class(["hyperion_tree"])
        .map_err(|e| Error::Other(format!("Failed to set class: {e}")))?;

    Ok(hyperion_tree.to_owned())
}

extendr_module! {
    mod lineage;

    fn get_model_lineage;
}
