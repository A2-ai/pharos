use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use nonmem::LineageTree;


/// Get's model lineage
///
/// @param model_dir path to directory containing all models
///
/// @return lineage tree //todo what is this?
/// @export
///
/// @examples \dontrun{
/// get_model_lineage("model/nonmem/")
/// }
#[extendr]
pub fn get_model_lineage(model_dir: &str) -> Result<Robj> {

    let lineage = LineageTree::from_folder(model_dir)
        .map_err(|e| Error::Other(format!("Failed to create lineage tree: {e}")))?;
    
    let mut lineage = to_robj(&lineage)
        .map_err(|e| Error::Other(format!("Failed to create Robj from LineageTree: {e}")))?;
    
    // Set S3 class
    let hyperion_tree = lineage
        .set_class(["hyperion_tree"])
        .map_err(|e| Error::Other(format!("Failed to set class: {e}")))?;

    Ok(hyperion_tree.to_owned())
}

extendr_module! {
    mod lineage;

    fn get_model_lineage;
}
