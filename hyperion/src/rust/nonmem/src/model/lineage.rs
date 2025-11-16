use extendr_api::deserializer::from_robj;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use nonmem::{LineageTree, ModelMetadata, OutputFileHash, RunEndFile, RunStartFile};

use hyperion_core::ResultExt;

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
/// @return hyperion_nonmem_tree S3 object
/// @export
///
/// @examples \dontrun{
/// get_model_lineage("model/nonmem/")
/// }
#[extendr]
pub fn get_model_lineage(model_dir: &str) -> Result<Robj> {
    // Create lineage tree from folder
    let lineage = LineageTree::from_folder(model_dir)
        .map_to_extendr_err("Pharos failed to create lineage tree")?;

    // Convert to R-compatible version (u128 -> f64)
    let r_lineage: RLineageTree = lineage.into();

    // Serialize R-compatible lineage to Robj
    let mut lineage_robj =
        to_robj(&r_lineage).map_to_extendr_err("Failed to create Robj from RLineageTree")?;

    // Set S3 class
    let hyperion_nonmem_tree = lineage_robj
        .set_class(["hyperion_nonmem_tree"])
        .map_to_extendr_err("Failed to set class")?;

    Ok(hyperion_nonmem_tree.to_owned())
}

pub fn robj_to_rlineagetree(r_tree: Robj) -> Result<()> {
    let tree_list = r_tree
        .as_list()
        .ok_or_else(|| Error::Other("Expected list".to_string()))?;

    let nodes_robj = tree_list.dollar("nodes")?;
    let nodes_hash = nodes_robj
        .as_list()
        .ok_or_else(|| Error::Other("Failed to make list for nodes robj".to_string()))?
        .into_hashmap();

    let nodes: HashMap<String, ModelMetadata> = nodes_hash
        .into_iter()
        .map(|(s, m)| {
            let metadata: ModelMetadata =
                from_robj(&m).map_to_extendr_err("Failed to deserialize into ModelMetadata")?;
            Ok((s.to_string(), metadata))
        })
        .collect::<Result<HashMap<String, ModelMetadata>>>()?;

    let metadata_robj = tree_list.dollar("metadata")?;
    let metadata_hash = metadata_robj
        .as_list()
        .ok_or_else(|| Error::Other("Failed to make list for nodes robj".to_string()))?
        .into_hashmap();

    let metadata: HashMap<String, (RunStartFile, Option<RRunEndFile>)> = metadata_hash
        .into_iter()
        .map(|(key, robj)| {
            // Get the tuple components from the R list
            let tuple_list = robj
                .as_list()
                .ok_or_else(|| Error::Other("metadata entry should be a list".to_string()))?;

            // Deserialize RunStartFile (assuming it doesn't have problematic collections)
            let start_file: RunStartFile = from_robj(&tuple_list[0])
                .map_to_extendr_err("Failed to deserialize RunStartFile")?;

            // Manually construct RRunEndFile if present
            let end_file = if tuple_list.len() > 1 && !tuple_list[1].is_null() {
                let end_robj = &tuple_list[1];
                let end_list = end_robj
                    .as_list()
                    .ok_or_else(|| Error::Other("RRunEndFile should be a list".to_string()))?
                    .into_hashmap();

                // Extract fields manually
                let start = end_list
                    .get("start")
                    .ok_or_else(|| Error::Other("Missing start field".to_string()))?
                    .as_str()
                    .ok_or_else(|| Error::Other("start should be string".to_string()))?
                    .to_string();

                let end = end_list
                    .get("end")
                    .ok_or_else(|| Error::Other("Missing end field".to_string()))?
                    .as_str()
                    .ok_or_else(|| Error::Other("end should be string".to_string()))?
                    .to_string();

                let runtime_ms = end_list
                    .get("runtime_ms")
                    .ok_or_else(|| Error::Other("Missing runtime_ms field".to_string()))?
                    .as_real()
                    .ok_or_else(|| Error::Other("runtime_ms should be number".to_string()))?;

                // Convert files_copied from R vector to HashSet
                let files_copied: HashSet<String> =
                    if let Some(files_robj) = end_list.get("files_copied") {
                        files_robj
                            .as_str_vector()
                            .unwrap_or_default()
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        HashSet::new()
                    };

                // Convert output_files_rewrites from R named list to HashMap
                let output_files_rewrites: HashMap<String, String> =
                    if let Some(rewrites_robj) = end_list.get("output_files_rewrites") {
                        rewrites_robj
                            .as_list()
                            .unwrap_or_default()
                            .into_hashmap()
                            .into_iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.to_string(), s.to_string())))
                            .collect()
                    } else {
                        HashMap::new()
                    };

                // Deserialize output_files_hashes Vec normally
                let output_files_hashes: Vec<OutputFileHash> =
                    if let Some(hashes_robj) = end_list.get("output_files_hashes") {
                        from_robj(hashes_robj)
                            .map_to_extendr_err("Failed to deserialize output_files_hashes")?
                    } else {
                        Vec::new()
                    };

                Some(RRunEndFile {
                    start,
                    end,
                    runtime_ms,
                    files_copied,
                    output_files_rewrites,
                    output_files_hashes,
                })
            } else {
                None
            };

            Ok((key.to_string(), (start_file, end_file)))
        })
        .collect::<Result<HashMap<String, (RunStartFile, Option<RRunEndFile>)>>>()?;

    let _tree = RLineageTree { nodes, metadata };

    Ok(())
}

extendr_module! {
    mod lineage;

    fn get_model_lineage;
}
