use extendr_api::Result;
use extendr_api::prelude::*;

//pharos nonmem crate
use nonmem::update_metadata_file;

use crate::utils::validated_model_from_robj;
use hyperion_core::{ResultExt, extendr_err};

/// Creates a metadata file for a NONMEM model
///
/// This function creates a metadata file that stores information about a NONMEM model,
/// including its description, tags, and lineage information. The metadata is stored
/// in a structured format that can be used for model tracking and documentation.
///
/// @param model_path Path to the NONMEM model file, or a hyperion_nonmem_model object (required)
/// @param description Optional description of the model and its purpose
/// @param tags Character vector of tags to categorize or label the model
/// @param based_on Character vector of model names/paths that this model is based on
///
/// @return Returns invisibly after creating the metadata file
/// @export
///
/// @examples
/// \dontrun{
/// # Create basic metadata for a model
/// set_metadata_file("run001.mod", description = "Base population PK model")
///
/// # Create metadata using a model object
/// model <- read_model("run001.mod")
/// set_metadata_file(model, description = "Base population PK model")
///
/// # Create metadata with tags and lineage
/// set_metadata_file(
///   "run002.mod",
///   description = "PK model with covariate effects",
///   tags = c("population", "pk", "covariates"),
///   based_on = c("run001.mod")
/// )
/// }
#[extendr]
pub fn set_metadata_file(
    model_path: Robj,
    #[extendr(default = "NULL")] description: Option<String>,
    #[extendr(default = "NULL")] tags: Option<Vec<String>>,
    #[extendr(default = "NULL")] based_on: Option<Vec<String>>,
) -> Result<()> {
    if let Some(d) = &description
        && d.trim().is_empty()
    {
        return Err(extendr_err!(
            "Description cannot be empty. Please provide a description for the model."
        ));
    };

    let model_path = validated_model_from_robj(&model_path)?;

    let tags = tags.unwrap_or_default();
    let based_on = based_on.unwrap_or_default();

    update_metadata_file(model_path, description, tags, based_on, true)
        .map_to_extendr_err("Failed to create metadata file")?;

    Ok(())
}

/// Updates a metadatafile
///
/// @param model_path path to model file or metadata file to update, or a hyperion_nonmem_model object
/// @param description Optional description to add to metadata
/// @param tags Optional character vector of tags to add to tags field
/// @param based_on character vector of models to add to based_on field
///
/// @return Invisibly after updaing
/// @export
///
/// @examples \dontrun{
/// update_metadata_file("model/nonmem/run001.mod", tags = "key model")
/// update_metadata_file("model/nonmem/run004.mod", tags = "key model", based_on = "1002")
/// model <- read_model("model/nonmem/run001.mod")
/// update_metadata_file(model, tags = "key model")
/// }
#[extendr(r_name = "update_metadata_file")]
pub fn append_to_metadata_file(
    model_path: Robj,
    #[extendr(default = "NULL")] description: Option<String>,
    #[extendr(default = "NULL")] tags: Option<Vec<String>>,
    #[extendr(default = "NULL")] based_on: Option<Vec<String>>,
) -> Result<()> {
    let path = validated_model_from_robj(&model_path)?;

    let tags = tags.unwrap_or_default();
    let based_on = based_on.unwrap_or_default();

    update_metadata_file(path, description, tags, based_on, false)
        .map_to_extendr_err("Failed to update metadata file")?;

    Ok(())
}

extendr_module! {
    mod metadata;

    fn set_metadata_file;
    fn append_to_metadata_file;
}
