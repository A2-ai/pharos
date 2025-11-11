use extendr_api::prelude::*;
use std::path::PathBuf;
use which::which;

// pharos scheduler crate
use nonmem::{RunOptions, expand_model_pattern};
use scheduler::{SchedulerType, slurm::SubmitOptions};

use hyperion_nonmem::utils::load_nonmem_config;

/// Helper function to process Robj model input and expand patterns
///
/// Takes an Robj that can be either:
/// - A single string (e.g., "run001.mod" or "run[001:003].mod")
/// - A character vector of strings
/// - A list of strings using values of the list for paths.
///
/// Returns a Vec<PathBuf> with all expanded model paths
fn process_model_robj(model: Robj) -> Result<Vec<PathBuf>> {
    let expand = |pattern: &str| {
        expand_model_pattern(pattern)
            .map_err(|e| Error::Other(format!("model pattern '{}': {e}", pattern)))
    };

    if let Some(s) = model.as_str() {
        expand(s)
    } else if let Some(strings) = model.as_str_vector() {
        strings
            .into_iter()
            .try_fold(Vec::new(), |mut acc, pattern| {
                acc.extend(expand(&pattern)?);
                Ok(acc)
            })
    } else if let Some(list) = model.as_list() {
        // Handle R lists
        list.values().try_fold(Vec::new(), |mut acc, item| {
            if let Some(pattern) = item.as_str() {
                acc.extend(expand(pattern)?);
                Ok(acc)
            } else {
                Err(Error::Other(format!(
                    "All list elements must be strings, found: {:?}",
                    item.rtype()
                )))
            }
        })
    } else {
        Err(Error::Other(
            "model must be a single string, character vector, or list of strings".to_string(),
        ))
    }
}

/// Submits a NONMEM model to SLURM for execution
///
/// This function submits a NONMEM model file to a SLURM cluster for execution,
/// allowing for parallel processing and job queue management. The function handles
/// job configuration, resource allocation, and job submission through pharos
///
/// @param model Path to the NONMEM model file, or character vector of model paths/patterns (required)
/// @param overwrite Whether to overwrite existing output files (default: FALSE)
/// @param dry_run Whether to perform a dry run without actually submitting the job (default: FALSE)
/// @param run_in_output_dir Whether to run the job in the output directory (default: FALSE)
/// @param ncpu Number of CPUs to allocate for the job (default: 1)
/// @param partition SLURM partition to submit the job to (default: NULL, uses cluster default)
/// @param clean_level Level of cleanup to perform after job completion (default: 1)
/// @param parafile Path to parameter file for parallel runs (default: NULL)
/// @param template Path to SLURM template file for job submission (default: NULL)
/// @param account SLURM account to charge the job to (default: NULL)
///
/// @return Returns invisibly after printing job submission results. Prints model path and corresponding SLURM job ID for each submitted job.
/// @export
///
/// @examples
/// \dontrun{
/// # Submit a basic NONMEM model
/// submit_model_to_slurm("model.mod")
///
/// # Submit with custom job name and multiple CPUs
/// submit_model_to_slurm("model.mod", job_name = "my_analysis", ncpu = 4)
///
/// # Dry run to test submission without actually running
/// submit_model_to_slurm("model.mod", dry_run = TRUE)
///
/// # Submit to specific partition with account
/// submit_model_to_slurm("model.mod", partition = "gpu", account = "myproject")
/// }
#[extendr]
pub fn submit_model_to_slurm(
    model: Robj,
    #[default = "FALSE"] overwrite: bool,
    #[default = "FALSE"] dry_run: bool,
    #[default = "FALSE"] run_in_output_dir: bool,
    #[default = "1"] ncpu: Option<u8>,
    #[default = "NULL"] partition: Option<String>,
    #[default = "1"] clean_level: Option<u8>,
    #[default = "NULL"] parafile: Option<String>,
    #[default = "NULL"] template: Option<String>,
    #[default = "NULL"] account: Option<String>,
) -> Result<()> {
    // Process model input to get list of model files
    let model_files = process_model_robj(model)?;

    let submit_options = SubmitOptions {
        // process_model_robj is handling model paths so SubmitOptions doesn't need it.
        model: String::new(),
        partition,
        account,
        template: template.map(PathBuf::from),
        dry_run,
        ..SubmitOptions::default()
    };

    let scheduler = SchedulerType::new_slurm(submit_options);
    let (config_path, nonmem_config) = load_nonmem_config(None)?;
    let parallel = ncpu.map_or(false, |n| n > 1);

    let run_options = RunOptions {
        run_in_output_dir,
        overwrite,
        clean_level,
        parallel,
        num_mpi_cpus: ncpu,
        parafile: parafile.map(PathBuf::from),
        ..RunOptions::default() // nonmem_version: (),
                                // output_dir: (),
                                // num_parallel: (),
                                // extra_files: (),
                                // mpi_timeout: (),
    };

    let pharos_exe_path = which("pharos")
        .map_err(|e| Error::Other(format!("Failed to locate pharos executable: {e}")))?;

    let res = scheduler
        .submit(
            config_path
                .parent()
                .expect("config file to have a parent dir"),
            model_files,
            run_options,
            nonmem_config,
            pharos_exe_path,
        )
        .map_err(|e| Error::Other(format!("Failed to submit job to slurm: {e}")))?;

    for (p, job_id) in res {
        println!("Model {p:?} -> job ID {job_id}");
    }
    Ok(())
}

extendr_module! {
    mod hyperion_scheduler;

    fn submit_model_to_slurm;
}
