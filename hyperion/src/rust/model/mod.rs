pub mod check;
pub mod copy;
pub mod lineage;
pub mod summary;

use extendr_api::deserializer::from_robj;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use crate::utils::find_output_file;
use fs_err as fs;
use nonmem::Model;
use std::path::PathBuf;

/// Gets model object
///
/// @param path path to mod file, model output directory, or metadata.json file
///
/// @return hyperion_model S3 object
/// @export
///
/// @examples \dontrun{
/// read_model("model/nonmem/run001")
/// }
#[extendr]
pub fn read_model(path: &str) -> Result<Robj> {
    // Read in mod file and parse into Model
    let path = find_output_file(path, "mod")?;

    let content = fs::read_to_string(&path).map_err(|e| Error::Other(format!("{e}")))?;

    let model = Model::parse(&content)
        .map_err(|e| Error::Other(format!("Failed to read model file: {e}")))?;

    // Convert to List directly
    let model_list = to_robj(&model)
        .map_err(|e| Error::Other(format!("failed to create Robj from Model: {e}")))?
        .as_list()
        .ok_or_else(|| Error::Other("Expected model to be a list".to_string()))?;

    // Save tokens and token_ranges for attributes
    let saved_tokens = model_list.dollar("tokens").ok();
    let saved_token_ranges = model_list.dollar("token_ranges").ok();
    // Rebuild list excluding tokens and token_ranges
    let mut new_pairs: Vec<(&str, Robj)> = Vec::new();
    for (name, value) in model_list.iter() {
        if name != "tokens" && name != "token_ranges" {
            new_pairs.push((name, value));
        }
    }

    // Add filename to model object
    if let Some(n) = path.file_stem().and_then(|name| name.to_str()) {
        new_pairs.push(("filename", n.into_robj()));
    }

    // Convert to Robj only at the end
    let mut model_robj: Robj = List::from_pairs(new_pairs).into();

    // Set hidden attributes
    if let Some(tokens) = saved_tokens {
        model_robj
            .set_attrib("_tokens", tokens)
            .map_err(|e| Error::Other(format!("Failed to set tokens attribute: {e}")))?;
    }
    if let Some(token_ranges) = saved_token_ranges {
        model_robj
            .set_attrib("_token_ranges", token_ranges)
            .map_err(|e| Error::Other(format!("Failed to set token_ranges attribute: {e}")))?;
    }

    // Set S3 class
    let result = model_robj
        .set_class(["hyperion_model"])
        .map_err(|e| Error::Other(format!("Failed to set class: {e}")))?;

    Ok(result.to_owned())
}

/// Checks model dataset
///
/// @param model list of model object from `read_model`
/// @param model_dir directory of model output //TODO check this
///
/// @return nothing //todo maybe a true/false?
/// @export
///
/// @examples \dontrun{
/// model <- read_model("model/nonmem/run001.mod")
/// model |> check_dataset("model/nonmem/run001")
/// }
#[extendr]
pub fn check_dataset(model: Robj, model_dir: &str) -> Result<Robj> {
    // Reconstruct full model object for deserialization
    let model_list = model
        .as_list()
        .ok_or_else(|| Error::Other("Expected model to be a list".to_string()))?;

    // Collect existing elements and add back tokens/token_ranges
    let mut pairs: Vec<(&str, Robj)> = Vec::new();

    // Copy existing elements
    for (name, value) in model_list.iter() {
        pairs.push((name, value));
    }

    // Add back tokens and token_ranges from attributes
    if let Some(tokens) = model.get_attrib("_tokens") {
        pairs.push(("tokens", tokens));
    }
    if let Some(token_ranges) = model.get_attrib("_token_ranges") {
        pairs.push(("token_ranges", token_ranges));
    }

    let full_model: Robj = List::from_pairs(pairs).into();

    let model: Model = from_robj(&full_model)
        .map_err(|e| Error::Other(format!("Failed to create Model from list: {e}")))?;

    let model_dir = PathBuf::from(model_dir);
    let dataset = model
        .check_dataset(&model_dir)
        .map_err(|e| Error::Other(format!("{e}")))?;

    let robj =
        to_robj(&dataset).map_err(|e| Error::Other(format!("Failed to serialize to Robj: {e}")))?;

    Ok(robj)
}

extendr_module! {
    mod model;
    use copy;
    use summary;
    use check;
    use lineage;

    fn read_model;
    fn check_dataset;
}
