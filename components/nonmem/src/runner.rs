use std::path::{Path, PathBuf};

use anyhow::Result;
use config::NonmemConfig;
use rayon::{ThreadPoolBuilder, prelude::*};

use crate::NonmemRunner;
use crate::run::RunOptions;

pub fn run_models(
    nonmem_config: &NonmemConfig,
    model_files: &[PathBuf],
    options: &RunOptions,
    config_dir: &Path,
) -> Result<()> {
    let max_threads = options.num_parallel.unwrap_or_else(num_cpus::get);
    let pool = ThreadPoolBuilder::new().num_threads(max_threads).build()?;

    if model_files.len() > 1 {
        println!(
            "Running {} model(s) using {} threads...",
            model_files.len(),
            max_threads
        );
    }

    pool.install(|| {
        let results = model_files
            .par_iter()
            .map(|model_file| {
                let mut nonmem_config = nonmem_config.clone();
                options.update_config_from_options(&mut nonmem_config);
                let nonmem_version_clone = options.nonmem_version.clone();

                // Then we figure out the output dir based on cli flag + config
                let output_dir_final = options
                    .output_dir
                    .clone()
                    .or_else(|| nonmem_config.output_dir.clone());

                println!("Running model: {model_file:?}");
                let mut runner = NonmemRunner::new(
                    model_file,
                    options.overwrite,
                    nonmem_config,
                    nonmem_version_clone,
                    output_dir_final,
                    options.extra_files.clone(),
                    config_dir,
                );
                if options.run_in_output_dir {
                    runner.run_in_output_dir();
                }
                match runner.run() {
                    Ok(exit_code) => {
                        if exit_code == 0 {
                            println!("Model {model_file:?} completed successfully.");
                        } else {
                            println!("Model {model_file:?} failed.");
                        }
                        Ok(exit_code)
                    }
                    Err(e) => {
                        eprintln!("Model failed: {model_file:?}: {e:?}");
                        Err(e)
                    }
                }
            })
            .collect::<Vec<_>>();

        if results.len() == 1 {
            match results[0] {
                Ok(exit_code) => {
                    if exit_code != 0 {
                        std::process::exit(exit_code);
                    }
                }
                Err(_) => std::process::exit(1),
            }
        } else {
            for result in results {
                match result {
                    Ok(exit_code) => {
                        if exit_code != 0 {
                            std::process::exit(1);
                        }
                    }
                    Err(_) => std::process::exit(1),
                }
            }
        }
    });

    Ok(())
}
