use extendr_api::Result;
use extendr_api::deserializer::from_robj;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use fs_err as fs;
use std::path::{Path, PathBuf};

//pharos nonmem crate
use nonmem::Model;
use nonmem::output_files::lst;

use crate::model::run_status::determine_run_status;
use crate::utils::{
    find_output_file, get_comment_type, get_model_source_path, resolve_input_model_path,
};
use hyperion_core::{OptionExt, ResultExt};

pub mod check;
pub mod copy;
pub mod lineage;
pub mod metadata;
pub mod parameters;
pub mod run_status;
pub mod summary;

/// Helper to convert Model to Robj for read_model and read_model_from_lst
///
/// This handles comment parsing and Model -> Robj + S3 setting
fn model_to_robj(model: &mut Model, path: impl AsRef<Path>) -> Result<Robj> {
    let path = path.as_ref();

    // Load config and extract comment type
    let comment_type = get_comment_type();
    if let Some(c) = comment_type {
        model.parse_comments(c);
    };

    // Convert to List directly
    let model_list = to_robj(&model)
        .map_to_extendr_err("failed to create Robj from Model")?
        .as_list()
        .ok_or_extendr_err("Expected model to be a list")?;

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

    // Add attributes to model
    add_tokens_attrs(&mut model_robj, saved_tokens, saved_token_ranges)?;
    add_model_source_attr(&mut model_robj, path)?;
    add_run_status_attr(&mut model_robj, path)?;

    // Set S3 class
    set_model_class(&mut model_robj)
}

fn add_tokens_attrs(
    model_robj: &mut Robj,
    saved_tokens: Option<Robj>,
    saved_token_ranges: Option<Robj>,
) -> Result<()> {
    if let Some(tokens) = saved_tokens {
        model_robj
            .set_attrib("_tokens", tokens)
            .map_to_extendr_err("Failed to set tokens attribute")?;
    }
    if let Some(token_ranges) = saved_token_ranges {
        model_robj
            .set_attrib("_token_ranges", token_ranges)
            .map_to_extendr_err("Failed to set token_ranges attribute")?;
    }

    Ok(())
}

fn add_model_source_attr(model_robj: &mut Robj, path: &Path) -> Result<()> {
    let source_path = get_model_source_path(path)?;
    model_robj
        .set_attrib("model_source", source_path.into_robj())
        .map_to_extendr_err("Failed to set model source attribute")?;

    Ok(())
}

fn add_run_status_attr(model_robj: &mut Robj, path: &Path) -> Result<()> {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext == "mod" || ext == "ctl" || ext == "lst" {
            let run_status = determine_run_status(path)?;
            model_robj
                .set_attrib("run_status", run_status.to_string().into_robj())
                .map_to_extendr_err("Failed to set run_status attribute")?;
        }
    }

    Ok(())
}

fn set_model_class(model_robj: &mut Robj) -> Result<Robj> {
    let result = model_robj
        .set_class(["hyperion_nonmem_model"])
        .map_to_extendr_err("Failed to set class")?;

    Ok(result.to_owned())
}

/// Helper function to reconstruct a pharos Model from hyperion_nonmem_model Robj
///
/// This handles the conversion from the R model object back to the full pharos Model
/// by adding back the tokens and token_ranges from attributes.
pub fn robj_to_model(model: &Robj) -> Result<Model> {
    // Reconstruct full model object for deserialization
    let model_list = model
        .as_list()
        .ok_or_extendr_err("Expected model to be a list")?;

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

    let model: Model =
        from_robj(&full_model).map_to_extendr_err("Failed to create Model from Robj")?;

    Ok(model)
}

/// Gets model object
///
/// @param path path to mod or ctl file.
///
/// @return hyperion_nonmem_model S3 object with `model_source` and `run_status` attributes
/// @export
///
/// @examples \dontrun{
/// read_model("model/nonmem/run001.mod")
/// }
#[extendr]
pub fn read_model(path: &str) -> Result<Robj> {
    let mod_path = resolve_input_model_path(&path)?;
    let content = fs::read_to_string(&mod_path).map_to_extendr_err("")?;

    let mut model = Model::parse(&content).map_to_extendr_err("Failed to read model file")?;
    let robj_model = model_to_robj(&mut model, mod_path)?;
    Ok(robj_model)
}

/// Gets model object from lst file
///
/// @param path path to lst file, model output directory, or metadata.json file.
///
/// @return hyperion_nonmem_model S3 object with `model_source` attribute for the source file
/// @export
///
/// @examples \dontrun{
/// read_model_from_lst("model/nonmem/run001/run001.lst")
/// }
#[extendr]
pub fn read_model_from_lst(path: &str) -> Result<Robj> {
    let path = find_output_file(path, "lst")?;
    let mut model =
        lst::extract_model(&path).map_to_extendr_err("Failed to extract Model from lst file")?;
    let robj_model = model_to_robj(&mut model, path)?;

    Ok(robj_model)
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
    let model = robj_to_model(&model)?;

    let model_dir = PathBuf::from(model_dir);
    let dataset = model.check_dataset(&model_dir).map_to_extendr_err("")?;

    let robj = to_robj(&dataset).map_to_extendr_err("Failed to serialize to Robj")?;

    Ok(robj)
}

extendr_module! {
    mod model;
    use copy;
    use summary;
    use check;
    use lineage;
    use parameters;
    use metadata;

    fn read_model;
    fn check_dataset;
    fn read_model_from_lst;
}
