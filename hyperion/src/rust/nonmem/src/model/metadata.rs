use extendr_api::prelude::*;
use std::path::PathBuf;

//pharos nonmem crate
use nonmem::create_metadata_file;

/// Creates a metadata file for a NONMEM model
///
/// This function creates a metadata file that stores information about a NONMEM model,
/// including its description, tags, and lineage information. The metadata is stored
/// in a structured format that can be used for model tracking and documentation.
///
/// @param model_path Path to the NONMEM model file (required)
/// @param description Optional description of the model and its purpose
/// @param tags Character vector of tags to categorize or label the model
/// @param based_on Character vector of model names/paths that this model is based on
/// @param overwrite Whether to overwrite an existing metadata file (default: FALSE)
///
/// @return Returns invisibly after creating the metadata file
/// @export
///
/// @examples
/// \dontrun{
/// # Create basic metadata for a model
/// create_metadata_file("run001.mod", description = "Base population PK model")
///
/// # Create metadata with tags and lineage
/// create_metadata_file(
///   "run002.mod",
///   description = "PK model with covariate effects",
///   tags = c("population", "pk", "covariates"),
///   based_on = c("run001.mod")
/// )
///
/// # Overwrite existing metadata
/// create_metadata_file(
///   "run001.mod",
///   description = "Updated base model",
///   overwrite = TRUE
/// )
/// }
#[extendr(r_name = "create_metadata_file")]
pub fn create_metadata_file_wrap(
    model_path: String,
    description: String,
    #[default = "NULL"] tags: Option<Vec<String>>,
    #[default = "NULL"] based_on: Option<Vec<String>>,
    #[default = "FALSE"] overwrite: bool,
) -> Result<()> {
    let model_path = PathBuf::from(model_path);

    if description == String::new() {
        return Err(Error::Other(
            "Please provide a description for this model".to_string(),
        ));
    }

    let tags = tags.unwrap_or(Vec::new());
    let based_on = based_on.unwrap_or(Vec::new());

    create_metadata_file(model_path, Some(description), tags, based_on, overwrite)
        .map_err(|e| Error::Other(format!("Failed to create metadata file: {e}")))?;
    Ok(())
}

extendr_module! {
    mod metadata;

    fn create_metadata_file_wrap;
}
