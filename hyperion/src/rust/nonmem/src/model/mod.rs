use extendr_api::Result;
use extendr_api::deserializer::from_robj;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use fs_err as fs;
use std::path::Path;

//pharos nonmem crate
use nonmem::Model;
use nonmem::output_files::lst;

use crate::model::run_status::determine_run_status;
use crate::utils::{
    find_output_file, get_comment_type, get_model_source_path, resolve_input_model_path,
    resolve_model_source_path,
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

    let comment_type = get_comment_type();
    if let Some(c) = comment_type {
        model.parse_comments(c);
    };

    let mut model_robj = to_robj(&model).map_to_extendr_err("failed to create Robj from Model")?;

    add_filename_attr(&mut model_robj, path)?;
    add_model_source_attr(&mut model_robj, path)?;
    add_run_status_attr(&mut model_robj, path)?;

    set_model_class(&mut model_robj)
}

fn add_filename_attr(model_robj: &mut Robj, path: &Path) -> Result<()> {
    if let Some(n) = path.file_stem().and_then(|name| name.to_str()) {
        model_robj
            .set_attrib("filename", n.into_robj())
            .map_to_extendr_err("Failed to set filename attribute")?;
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
pub fn robj_to_model(model: &Robj) -> Result<Model> {
    from_robj(model).map_to_extendr_err("Failed to create Model from Robj")
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

/// Gets model object from lst file (internal)
///
/// @param path path to lst file, model output directory, or metadata.json file.
///
/// @return hyperion_nonmem_model S3 object with `model_source` attribute for the source file
/// @keywords internal
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
/// @param model hyperion_nonmem_model object from `read_model`
///
/// @return Dataset check results
/// @export
///
/// @examples \dontrun{
/// model <- read_model("model/nonmem/run001.mod")
/// model |> check_dataset()
/// }
#[extendr]
pub fn check_dataset(model: Robj) -> Result<Robj> {
    let source = model
        .get_attrib("model_source")
        .ok_or_extendr_err("Model object is missing model_source attribute")?
        .as_str()
        .ok_or_extendr_err("model_source attribute must be a character")?;
    let model_path = resolve_model_source_path(source)?;
    let model_dir = model_path
        .parent()
        .ok_or_extendr_err("Could not determine model directory")?;

    let model = robj_to_model(&model)?;
    let dataset = model.check_dataset(model_dir).map_to_extendr_err("")?;

    let mut robj = to_robj(&dataset).map_to_extendr_err("Failed to serialize to Robj")?;

    robj.set_class(["hyperion_nonmem_dataset"])
        .map_to_extendr_err("Failed to set class")?;

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
    use run_status;

    fn read_model;
    fn check_dataset;
    fn read_model_from_lst;
}
