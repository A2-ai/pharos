use extendr_api::prelude::*;
use std::path::PathBuf;
use which::which;

// pharos scheduler crate
use nonmem::RunOptions;
use scheduler::{SchedulerType, slurm::SubmitOptions};

use hyperion_nonmem::utils::load_nonmem_config;

/// Submits a NONMEM model to SLURM for execution
///
/// This function submits a NONMEM model file to a SLURM cluster for execution,
/// allowing for parallel processing and job queue management. The function handles
/// job configuration, resource allocation, and job submission through pharos
///
/// @param model Path to the NONMEM model file (required)
/// @param job_name Optional name for the SLURM job. If not provided, a default name will be generated
/// @param overwrite Whether to overwrite existing output files (default: FALSE)
/// @param dry_run Whether to perform a dry run without actually submitting the job (default: FALSE)
/// @param run_in_output_dir Whether to run the job in the output directory (default: FALSE)
/// @param num_cpu Number of CPUs to allocate for the job (default: 1)
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
/// submit_slurm_job("model.mod")
///
/// # Submit with custom job name and multiple CPUs
/// submit_slurm_job("model.mod", job_name = "my_analysis", num_cpu = 4)
///
/// # Dry run to test submission without actually running
/// submit_slurm_job("model.mod", dry_run = TRUE)
///
/// # Submit to specific partition with account
/// submit_slurm_job("model.mod", partition = "gpu", account = "myproject")
/// }
#[extendr]
pub fn submit_slurm_job(
    model: String,
    #[default = "NULL"] job_name: Option<String>,
    #[default = "FALSE"] overwrite: bool,
    #[default = "FALSE"] dry_run: bool,
    #[default = "FALSE"] run_in_output_dir: bool,
    #[default = "1"] num_cpu: Option<u8>,
    #[default = "NULL"] partition: Option<String>,
    #[default = "1"] clean_level: Option<u8>,
    #[default = "NULL"] parafile: Option<String>,
    #[default = "NULL"] template: Option<String>,
    #[default = "NULL"] account: Option<String>,
) -> Result<()> {
    let submit_options = SubmitOptions {
        model: model.clone(),
        job_name,
        partition,
        account,
        template: template.map(PathBuf::from),
        dry_run,
    };

    let scheduler = SchedulerType::new_slurm(submit_options);
    let (config_path, nonmem_config) = load_nonmem_config(None)?;

    // take Robj that can be character vector of models or a single model
    let model_files = vec![PathBuf::from(&model)];
    let parallel = num_cpu.map_or(false, |n| n > 1);

    let run_options = RunOptions {
        run_in_output_dir,
        overwrite,
        clean_level,
        parallel,
        num_mpi_cpus: num_cpu,
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

    fn submit_slurm_job;
}
