pub mod slurm;
use slurm::RPartitionTable;

use extendr_api::Result;
use extendr_api::prelude::*;

use std::path::PathBuf;
use which::which;

// pharos scheduler crate
use nonmem::{RunOptions, expand_model_pattern};
use scheduler::{
    SchedulerType,
    sge::SubmitOptions as SgeSubmitOptions,
    slurm::{SubmitOptions as SlurmSubmitOptions, resolve_partition},
};

use hyperion_core::{ResultExt, extendr_err};
use hyperion_nonmem::utils::{load_nonmem_config, path_from_robj};

/// Helper function to process Robj model input and expand patterns
///
/// Takes an Robj that can be either:
/// - A hyperion_nonmem_model object
/// - A single string (e.g., "run001.mod" or "run[001:003].mod")
/// - A character vector of strings
/// - A list of strings or model objects
///
/// Returns a Vec<PathBuf> with all expanded model paths
fn process_model_robj(model: Robj) -> Result<Vec<PathBuf>> {
    let expand = |pattern: &str| {
        expand_model_pattern(pattern).map_to_extendr_err(format!("model pattern '{pattern}'"))
    };

    // Handle hyperion_nonmem_model object
    if model.inherits("hyperion_nonmem_model") {
        let path = path_from_robj(&model, true)?;
        return Ok(vec![path]);
    }

    if let Some(s) = model.as_str() {
        expand(s)
    } else if let Some(strings) = model.as_str_vector() {
        strings
            .into_iter()
            .try_fold(Vec::new(), |mut acc, pattern| {
                acc.extend(expand(pattern)?);
                Ok(acc)
            })
    } else if let Some(list) = model.as_list() {
        // Handle R lists (can contain strings or model objects)
        list.values().try_fold(Vec::new(), |mut acc, item| {
            if item.inherits("hyperion_nonmem_model") {
                let path = path_from_robj(&item, true)?;
                acc.push(path);
                Ok(acc)
            } else if let Some(pattern) = item.as_str() {
                acc.extend(expand(pattern)?);
                Ok(acc)
            } else {
                Err(extendr_err!(
                    "All list elements must be strings or model objects, found: {:?}",
                    item.rtype()
                ))
            }
        })
    } else {
        Err(extendr_err!(
            "model must be a model object, string, character vector, or list"
        ))
    }
}

/// Submits a NONMEM model to SLURM for execution
///
/// This function submits a NONMEM model file to a SLURM cluster for execution,
/// allowing for parallel processing and job queue management. The function handles
/// job configuration, resource allocation, and job submission through pharos
///
/// @param model A hyperion_nonmem_model object, path to the NONMEM model file,
/// or character vector of model paths/patterns (required)
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
/// # Submit using a model object
/// model <- read_model("model.mod")
/// submit_model_to_slurm(model)
///
/// # Dry run to test submission without actually running
/// submit_model_to_slurm("model.mod", dry_run = TRUE)
///
/// # Submit to specific partition with account
/// submit_model_to_slurm("model.mod", partition = "gpu", account = "myproject")
/// }
#[extendr]
#[allow(clippy::too_many_arguments)]
pub fn submit_model_to_slurm(
    model: Robj,
    #[extendr(default = "FALSE")] overwrite: bool,
    #[extendr(default = "FALSE")] dry_run: bool,
    #[extendr(default = "FALSE")] run_in_output_dir: bool,
    #[extendr(default = "1")] ncpu: Option<u8>,
    #[extendr(default = "NULL")] partition: Option<String>,
    #[extendr(default = "1")] clean_level: Option<u8>,
    #[extendr(default = "NULL")] parafile: Option<String>,
    #[extendr(default = "NULL")] template: Option<String>,
    #[extendr(default = "NULL")] account: Option<String>,
) -> Result<()> {
    // Process model input to get list of model files
    let model_files = process_model_robj(model)?;

    let (config_path, nonmem_config) = load_nonmem_config(None)?;

    // check partition and give advice if needed
    let model_count = model_files.len();
    let ncpu_i32 = i32::from(ncpu.unwrap_or(1));
    let table = RPartitionTable::from_slurm()?;
    let partition_name = resolve_partition(
        partition.as_deref(),
        nonmem_config.slurm.partition.as_deref(),
    )
    .map_to_extendr_err("Failed to get requested partition")?;
    let active = table.find_partition(&partition_name);

    if let Some(active) = active {
        if !active.fits(ncpu_i32) {
            let advice = table.partition_advice(ncpu_i32, &partition_name, false);
            call!("stop", advice)?;
        } else if table.is_underutilized(&partition_name, ncpu_i32, model_count) {
            let advice = table.partition_advice(ncpu_i32, &partition_name, true);
            call!("warning", advice)?;
        }
    }

    let submit_options = SlurmSubmitOptions {
        // process_model_robj is handling model paths so SubmitOptions doesn't need it.
        model: String::new(),
        partition,
        account,
        template: template.map(PathBuf::from),
        dry_run,
        ..SlurmSubmitOptions::default()
    };

    let scheduler = SchedulerType::new_slurm(submit_options);
    let parallel = ncpu.is_some_and(|n| n > 1);

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

    let pharos_exe_path =
        which("pharos").map_to_extendr_err("Failed to locate pharos executable")?;

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
        .map_to_extendr_err("Failed to submit job to slurm")?;

    for (p, job_id) in res {
        rprintln!("Model {p:?} -> job ID {job_id}");
    }
    Ok(())
}

/// Submits a NONMEM model to SGE for execution
///
/// This function submits a NONMEM model file to a SGE cluster for execution,
/// allowing for parallel processing and job queue management. The function handles
/// job configuration, resource allocation, and job submission through pharos
///
/// @param model A hyperion_nonmem_model object, path to the NONMEM model file,
/// or character vector of model paths/patterns (required)
/// @param overwrite Whether to overwrite existing output files (default: FALSE)
/// @param dry_run Whether to perform a dry run without actually submitting the job (default: FALSE)
/// @param run_in_output_dir Whether to run the job in the output directory (default: FALSE)
/// @param ncpu Number of CPUs to allocate for the job (default: 1)
/// @param clean_level Level of cleanup to perform after job completion (default: 1)
/// @param parafile Path to parameter file for parallel runs (default: NULL)
/// @param template Path to SGE template file for job submission (default: NULL)
///
/// @return Returns invisibly after printing job submission results. Prints model path and corresponding SGE job ID for each submitted job.
/// @export
///
/// @examples
/// \dontrun{
/// # Submit a basic NONMEM model
/// submit_model_to_sge("model.mod")
///
/// # Submit using a model object
/// model <- read_model("model.mod")
/// submit_model_to_sge(model)
///
/// # Dry run to test submission without actually running
/// submit_model_to_sge("model.mod", dry_run = TRUE)
///}
#[extendr]
#[allow(clippy::too_many_arguments)]
pub fn submit_model_to_sge(
    model: Robj,
    #[extendr(default = "FALSE")] overwrite: bool,
    #[extendr(default = "FALSE")] dry_run: bool,
    #[extendr(default = "FALSE")] run_in_output_dir: bool,
    #[extendr(default = "1")] ncpu: Option<u8>,
    #[extendr(default = "1")] clean_level: Option<u8>,
    #[extendr(default = "NULL")] parafile: Option<String>,
    #[extendr(default = "NULL")] template: Option<String>,
) -> Result<()> {
    // Process model input to get list of model files
    let model_files = process_model_robj(model)?;

    let submit_options = SgeSubmitOptions {
        // process_model_robj is handling model paths so SubmitOptions doesn't need it.
        model: String::new(),
        template: template.map(PathBuf::from),
        dry_run,
        ..SgeSubmitOptions::default()
    };

    let scheduler = SchedulerType::new_sge(submit_options);
    let (config_path, nonmem_config) = load_nonmem_config(None)?;
    let parallel = ncpu.is_some_and(|n| n > 1);

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

    let pharos_exe_path =
        which("pharos").map_to_extendr_err("Failed to locate pharos executable")?;

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
        .map_to_extendr_err("Failed to submit job to sge")?;

    for (p, job_id) in res {
        rprintln!("Model {p:?} submitted: job id {job_id}");
    }

    Ok(())
}

extendr_module! {
    mod hyperion_scheduler;

    use slurm;

    fn submit_model_to_slurm;
    fn submit_model_to_sge;
}
