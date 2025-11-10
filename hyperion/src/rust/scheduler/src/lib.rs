use extendr_api::prelude::*;
use std::path::PathBuf;
use which::which;

// pharos scheduler crate
use nonmem::RunOptions;
use scheduler::{SchedulerType, slurm::SubmitOptions};

use hyperion_nonmem::utils::load_nonmem_config;

/// Submits a nonmem job to slurm
///
/// @param model
/// @param job_name
/// @param overwrite
/// @param dry_run
/// @param run_in_output_dir
/// @param num_cpu
/// @param partition
/// @param clean_level
/// @param parafile
/// @param template
///
/// @return
/// @export
///
/// @examples
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
