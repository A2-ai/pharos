use std::path::{Path, PathBuf};

use anyhow::Result;
use config::NonmemConfig;
use rayon::{ThreadPoolBuilder, prelude::*};

use crate::NonmemRunner;
use crate::run::RunOptions;

/// Runs the given models in parallel, printing each one's progress and
/// failure as it happens. Returns each model's NONMEM exit code, or the error
/// that kept it from running, in the order given.
pub fn run_models(
    nonmem_config: &NonmemConfig,
    model_files: &[PathBuf],
    options: &RunOptions,
    config_dir: &Path,
) -> Result<Vec<Result<i32>>> {
    let max_threads = options.num_parallel.unwrap_or_else(num_cpus::get);
    let pool = ThreadPoolBuilder::new().num_threads(max_threads).build()?;

    if model_files.len() > 1 {
        println!(
            "Running {} model(s) using {} threads...",
            model_files.len(),
            max_threads
        );
    }

    let results = pool.install(|| {
        model_files
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
                let result = runner.run();
                match &result {
                    Ok(0) => println!("Model {model_file:?} completed successfully."),
                    Ok(_) => println!("Model {model_file:?} failed."),
                    Err(e) => eprintln!("Model failed: {model_file:?}: {e:#}"),
                }
                result
            })
            .collect::<Vec<_>>()
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use fs_err as fs;

    use super::*;
    use crate::refusal::{Refusal, Step};

    /// A model whose output directory exists is refused unless it is
    /// overwritten; the refusal comes back with its steps.
    #[test]
    fn an_existing_output_dir_is_reported_as_a_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("run001.mod");
        // The output dir is checked before the model is read
        fs::write(&model, "").unwrap();
        fs::create_dir(dir.path().join("run001")).unwrap();

        let results = run_models(
            &NonmemConfig::default(),
            &[model],
            &RunOptions::default(),
            dir.path(),
        )
        .unwrap();
        let [Err(error)] = results.as_slice() else {
            panic!("expected one error, got {results:?}");
        };
        assert_eq!(
            error.downcast_ref::<Refusal>().map(|r| r.steps.clone()),
            Some(vec![Step::Overwrite])
        );
    }
}
