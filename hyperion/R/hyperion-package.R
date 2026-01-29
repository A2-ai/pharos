#' hyperion: Pharmaceutical Model Development and Workflow Tools
#'
#' @description
#' Hyperion is an R interface to the pharos CLI for pharmaceutical model
#' development workflows. It streamlines the complete workflow from development
#' through cluster execution and analysis for NONMEM modeling.
#'
#' @section Model I/O:
#' Functions for reading, writing, and validating NONMEM models:
#' \itemize{
#'   \item [read_model()] - Read a model from a .mod or .ctl file
#'   \item [copy_model()] - Copy a model to a new file with optional parameter updates
#'   \item [check_model()] - Validate model syntax
#'   \item [check_dataset()] - Validate model dataset
#'   \item [get_model_name()] - Get the model name (filename without extension)
#'   \item [get_model_dir()] - Get the model directory path
#'   \item [get_data_path()] - Get the dataset path from the model
#' }
#'
#' @section Model Summaries:
#' Functions for summarizing model runs:
#' \itemize{
#'   \item [summary()] - Get a summary of a completed model run
#'   \item [get_run_info()] - Get run details and heuristics from .lst file
#' }
#'
#' @section Parameter Extraction:
#' Functions for extracting parameter estimates, gradients, and shrinkage:
#' \itemize{
#'   \item [get_parameters()] - Get parameter estimates from .ext file
#'   \item [get_final_estimates()] - Get final parameter estimates
#'   \item [read_ext_file()] - Read .ext file directly
#'   \item [get_gradients()] - Get gradient values
#'   \item [get_eta_shrinkage()] - Get ETA shrinkage
#'   \item [get_eps_shrinkage()] - Get EPS shrinkage
#' }
#'
#' @section Parameter Metadata:
#' S7 classes and functions for structured parameter comments:
#' \itemize{
#'   \item [get_model_parameter_info()] - Extract parameter metadata from a model
#'   \item [ThetaComment], [OmegaComment], [SigmaComment] - Parameter comment classes
#'   \item [ModelComments] - Container for all parameter comments
#'   \item [get_comment()] - Get a comment by NONMEM name
#'   \item [get_parameter_names()] - Get user-defined parameter names from model or ModelComments
#'   \item [get_parameter_transform()] - Get parameterization (transform) type
#'   \item [get_parameter_unit()] - Get parameter units
#'   \item [get_theta_names()] - Get theta parameter names
#'   \item [get_eta_labels()] - Get ETA labels for plots/tables
#'   \item [update_param_info()] - Update parameter metadata
#'   \item [audit_parameter_info()] - Audit provenance of metadata fields
#' }
#'
#' @section Lookup Files:
#' Functions for managing TOML lookup files with parameter metadata:
#' \itemize{
#'   \item [apply_lookup()] - Apply lookup file to fill missing metadata
#'   \item [apply_lookup_defaults()] - Apply default values from lookup
#'   \item [add_parameter_to_lookup()] - Add a parameter to lookup file
#'   \item [remove_parameter_from_lookup()] - Remove a parameter from lookup
#'   \item [list_lookup_parameters()] - List parameters in a lookup file
#' }
#'
#' @section Transform Calculations:
#' Functions for computing derived statistics with transform awareness:
#' \itemize{
#'   \item [compute_cv()] - Compute coefficient of variation
#'   \item [compute_rse()] - Compute relative standard error
#'   \item [compute_ci()] - Compute confidence intervals
#'   \item [transform_value()] - Back-transform estimates to natural scale
#' }
#'
#' @section Model Lineage:
#' Functions for tracking model development history:
#' \itemize{
#'   \item [get_model_lineage()] - Get full lineage tree
#'   \item [get_model_ancestors()] - Get ancestor models
#'   \item [get_model_descendants()] - Get descendant models
#'   \item [are_models_in_lineage()] - Check if models are related
#' }
#'
#' @section Configuration:
#' Functions for pharos configuration:
#' \itemize{
#'   \item [init()] - Initialize pharos with config file path
#'   \item [get_pharos_config()] - Get current pharos configuration
#'   \item [get_comment_type()] - Get comment parsing mode (raw or type1)
#'   \item [use_type1_comments()] - Configure pharos.toml for type1 comment parsing
#' }
#'
#' @section Metadata Files:
#' Functions for managing model metadata JSON files:
#' \itemize{
#'   \item [set_metadata_file()] - Create or update metadata file
#'   \item [update_metadata_file()] - Update existing metadata
#' }
#'
#' @section Job Submission:
#' Functions for submitting models to compute clusters:
#' \itemize{
#'   \item [submit_model_to_slurm()] - Submit to SLURM scheduler
#'   \item [submit_model_to_sge()] - Submit to SGE scheduler
#' }
#'
## usethis namespace: start
#' @importFrom lifecycle deprecated
## usethis namespace: end
#'
#' @keywords internal
"_PACKAGE"
