use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use config::{Config, NonmemConfig, render_output_template};
use fs_err as fs;
use nonmem::expand_model_pattern;
use nonmem::output_files::get_summary;
use nonmem::{CopyOptions, LineageTree, RunOptions, check_model, copy_model, run_models};

fn build_lineage_row(
    lineage_tree: &LineageTree,
    model_name: &str,
    model_metadata: &nonmem::ModelMetadata,
) -> Vec<String> {
    // Format parents
    let parents = if model_metadata.based_on.is_empty() {
        "-".to_string()
    } else {
        model_metadata.based_on.join(", ")
    };

    // Get runtime and hashes from metadata
    let (runtime, dataset_hash, model_hash) =
        if let Some((start_file, end_file)) = lineage_tree.get_metadata_for(model_name) {
            let runtime = end_file
                .as_ref()
                .map_or("N/A".to_string(), |e| e.formatted_runtime());
            let dataset_hash = start_file.dataset_hashes.formatted_blake3();
            let model_hash = start_file.model_hashes.formatted_blake3();
            (runtime, dataset_hash, model_hash)
        } else {
            ("N/A".to_string(), "N/A".to_string(), "N/A".to_string())
        };

    // Format description and tags
    let description = if model_metadata.description.is_empty() {
        "-".to_string()
    } else {
        model_metadata.description.clone()
    };

    let tags = if model_metadata.tags.is_empty() {
        "-".to_string()
    } else {
        model_metadata.tags.join(", ")
    };

    vec![
        model_name.to_string(),
        parents,
        runtime,
        dataset_hash,
        model_hash,
        description,
        tags,
    ]
}

// Move this to cli mod?
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(i) {
                *width = (*width).max(cell.len());
            }
        }
    }

    // Print top border
    print!("+");
    for &width in &widths {
        print!("{}", "-".repeat(width + 2));
        print!("+");
    }
    println!();

    // Print header
    print!("|");
    for (i, header) in headers.iter().enumerate() {
        print!(" {:width$} |", header, width = widths[i]);
    }
    println!();

    // Print separator
    print!("+");
    for &width in &widths {
        print!("{}", "-".repeat(width + 2));
        print!("+");
    }
    println!();

    // Print data rows
    for row in rows {
        print!("|");
        for (i, cell) in row.iter().enumerate() {
            let width = widths.get(i).unwrap_or(&0);
            print!(" {:width$} |", cell, width = width);
        }
        println!();
    }

    // Print bottom border
    print!("+");
    for &width in &widths {
        print!("{}", "-".repeat(width + 2));
        print!("+");
    }
    println!();
}

#[derive(Parser)]
#[clap(name = "pharos", version)]
#[command(about = "A CLI tool for pharos operations")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[clap(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Nonmem {
        #[command(subcommand)]
        nonmem_command: NonmemCommands,
    },
}

#[derive(Subcommand)]
pub enum NonmemCommands {
    Init,
    /// Checks the model file with nonmem without running the model.
    /// This will the executables for nonmem version selected in pharos.toml
    Check {
        model: String,
    },
    /// Run the given nonmem model using the pharos config file. You can specify run options that
    /// will override the configuration values.
    Run {
        /// The model to run
        /// It can be a path to .mod file or a pattern like `run[001:003].mod` where pharos will
        /// run the models in parallel
        model: String,
        /// Run options for NONMEM execution
        #[clap(flatten)]
        run_options: RunOptions,
    },
    /// Create a new model from an existing one. All paths specific to a model number in the
    /// model file will be replaced.
    Copy {
        /// Which model to use as base
        #[clap(long)]
        from: PathBuf,
        /// The name of the new model
        #[clap(long)]
        to: PathBuf,
        /// Whether to overwrite an existing model file if it already exists
        #[clap(long)]
        overwrite: bool,
        /// Path to the .ext file containing parameter estimates to use
        /// If not specified, will try {model_name}/{model_name}.ext
        #[clap(long)]
        ext_file: Option<String>,
        /// Copy options including parameter update configuration
        #[clap(flatten)]
        copy_options: CopyOptions,
    },
    /// Generate a summary of NONMEM run results
    Summary {
        /// Directory of the run output
        directory: PathBuf,
        /// Output summary as JSON instead of formatted table
        #[clap(long)]
        json: bool,
    },
    /// Show model lineage and relationships
    Lineage {
        /// Folder containing models and metadata
        folder: PathBuf,
        /// Show lineage tree starting from this model
        #[clap(long)]
        from: Option<String>,
        /// Show lineage tree leading up to this model
        #[clap(long)]
        to: Option<String>,
    },
}

fn find_output_folder(
    config_path: impl AsRef<Path>,
    model_path: impl AsRef<Path>,
) -> Result<Option<PathBuf>> {
    let model_path = model_path.as_ref();
    let config_path = config_path.as_ref();

    let model_name = model_path
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("Could not determine model file stem"))?
        .to_string_lossy();

    let root_folder = model_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not determine parent directory"))?;

    // First look up if there is an output dir
    let mut possible_folders = vec![model_name.as_ref().to_string()];

    if config_path.exists() {
        let config = Config::load(config_path)?;
        if let Some(o) = config.nonmem.and_then(|c| c.output_dir)
            && let Ok(o2) = render_output_template(&o, model_name.as_ref())
        {
            possible_folders.push(o2);
        }
    }

    for f in possible_folders {
        let p = root_folder.join(f).join(format!("{}.ext", model_name));
        if p.exists() {
            return Ok(Some(p));
        }
    }

    Ok(None)
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("pharos.toml");
    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Off
        })
        .init();

    let load_nonmem_config = |run_nonmem_version: Option<&str>| -> Result<NonmemConfig> {
        if !config_path.exists() {
            bail!("pharos config file does not exist");
        }
        log::debug!("Loading pharos config file: {config_path:?}");
        let config = Config::load(&config_path)?;
        let nonmem_config = match config.nonmem {
            Some(config) => config,
            None => bail!("pharos config file does not contain nonmem configuration"),
        };

        if let Some(version) = run_nonmem_version
            && !nonmem_config.versions.contains_key(version)
        {
            bail!("nonmem version {version} not found in config file");
        }

        Ok(nonmem_config)
    };

    match cli.command {
        Commands::Nonmem { nonmem_command } => match nonmem_command {
            NonmemCommands::Init => {
                if config_path.exists() {
                    bail!("pharos config file already exists");
                }

                let mut config_file = fs::File::create(&config_path)?;
                let config = toml::to_string_pretty(&Config::new_nonmem())?;
                config_file.write_all(config.as_bytes())?;
                println!("pharos config file created");
            }
            NonmemCommands::Check { model } => {
                let nonmem_config = load_nonmem_config(None)?;
                check_model(&nonmem_config, Path::new(&model))?;
            }
            NonmemCommands::Run { model, run_options } => {
                let nonmem_config = load_nonmem_config(run_options.nonmem_version.as_deref())?;

                // Expand model pattern to get all model files
                let model_files = expand_model_pattern(&model)?;

                // Validate that all model files exist
                for model_file in &model_files {
                    if !model_file.exists() {
                        bail!("Model file does not exist: {}", model_file.display());
                    }
                }

                log::debug!("Going to run: {model_files:?}");
                run_models(&nonmem_config, &model_files, &run_options)?;
            }
            NonmemCommands::Copy {
                from,
                to,
                overwrite,
                ext_file,
                mut copy_options,
            } => {
                // Validate copy options
                if let Err(e) = copy_options.validate_update() {
                    bail!("{}", e);
                }
                // Validate from file exists
                let from = Path::new(&from);
                if !from.exists() {
                    bail!("Model file does not exist: {}", from.display());
                }
                let original_filename = match from.file_name() {
                    Some(filename) => filename.to_string_lossy().to_string(),
                    None => bail!("`from` model file does not have a file name"),
                };

                // Validate to file doesn't exist or overwrite is allowed
                let to = Path::new(&to);
                if to.exists() && !overwrite {
                    bail!(
                        "Model file {} already exists and the --overwrite flag was not passed",
                        to.display()
                    );
                }
                let new_filename = match to.file_name() {
                    Some(filename) => filename.to_string_lossy().to_string(),
                    None => bail!("`to` model file does not have a file name"),
                };

                // Validate ext file if parameter updates are requested
                if copy_options.is_updating_params() {
                    let ext_path = match &ext_file {
                        Some(path) => PathBuf::from(path),
                        None => find_output_folder(&config_path, from)?.unwrap_or_default(),
                    };

                    if !ext_path.exists() {
                        if ext_file.is_none() {
                            bail!(
                                "Could not find .ext file at expected location: {}\n\
                                 Use --ext-file to specify the correct path to the parameter estimates file",
                                ext_path.display()
                            );
                        } else {
                            bail!("Ext file not found: {}", ext_path.display());
                        }
                    }
                    copy_options.ext_path = Some(ext_path);
                }

                copy_model(from, to, &original_filename, &new_filename, &copy_options)?;
            }
            NonmemCommands::Summary { directory, json } => {
                let comment_type = if config_path.exists() {
                    let config = Config::load(&config_path)?;
                    config.nonmem.and_then(|x| x.comments.r#type)
                } else {
                    None
                };

                let summary = get_summary(&directory, comment_type)?;

                if json {
                    let json_output = serde_json::to_string_pretty(&summary)?;
                    println!("{}", json_output);
                } else {
                    println!("=== Summary ===");
                    println!();
                    println!("Problem: {}", summary.lst.run_details.problem);
                    println!(
                        "Records: {}   Observations: {}  Subjects: {}",
                        summary.lst.run_details.number_data_records,
                        summary.lst.run_details.number_obs,
                        summary.lst.run_details.number_subjects
                    );
                    println!();
                    println!("Estimation Method(s):");
                    for m in &summary.lst.run_details.estimation_methods {
                        println!(" - {}", m);
                    }
                    println!();
                    println!("Objective Function Value:");
                    for m in &summary.lst.run_details.ofv {
                        match m {
                            Some(o) => println!(" - {:.3}", o),
                            None => println!(" - N/A"),
                        }
                    }
                    println!();

                    let h = &summary.lst.run_heuristics;
                    let mut heur: Vec<&str> = Vec::new();
                    if h.minimization_terminated == Some(true) {
                        heur.push("Minimization terminated");
                    }
                    if h.hessian_reset == Some(true) {
                        heur.push("Hessian reset");
                    }
                    if h.parameter_near_boundary == Some(true) {
                        heur.push("Parameter near boundary");
                    }
                    if h.covariance_step_aborted == Some(true) {
                        heur.push("Covariance step aborted");
                    }
                    if h.eigenvalue_issues == Some(true) {
                        heur.push("Eigenvalue issues");
                    }
                    if heur.is_empty() {
                        println!("Heuristic Problems Detected:\n - None\n");
                    } else {
                        println!("Heuristic Problems Detected:");
                        for item in heur {
                            println!(" - {}", item);
                        }
                        println!();
                    }

                    // THETA parameters
                    if !summary.parameters.theta.is_empty() {
                        println!("THETA Parameters:");
                        let theta_rows: Vec<Vec<String>> = summary
                            .parameters
                            .theta
                            .iter()
                            .map(|theta| theta.as_string_pieces())
                            .collect();
                        print_table(
                            &["Parameter", "Estimate", "SE (RSE%)", "Fixed"],
                            &theta_rows,
                        );
                        println!();
                    }

                    // OMEGA parameters
                    let omega_params: Vec<_> = summary
                        .parameters
                        .random_effects
                        .iter()
                        .filter(|p| p.is_omega())
                        .collect();
                    if !omega_params.is_empty() {
                        println!("OMEGA Parameters:");
                        let omega_rows: Vec<Vec<String>> = omega_params
                            .iter()
                            .map(|omega| omega.as_string_pieces())
                            .collect();
                        print_table(
                            &[
                                "Parameter",
                                "ETA",
                                "Estimate",
                                "SE (RSE%)",
                                "Shrinkage (%)",
                                "Fixed",
                            ],
                            &omega_rows,
                        );
                        println!();
                    }

                    // SIGMA parameters
                    let sigma_params: Vec<_> = summary
                        .parameters
                        .random_effects
                        .iter()
                        .filter(|p| p.is_sigma())
                        .collect();
                    if !sigma_params.is_empty() {
                        println!("SIGMA Parameters:");
                        let sigma_rows: Vec<Vec<String>> = sigma_params
                            .iter()
                            .map(|sigma| sigma.as_string_pieces())
                            .collect();
                        print_table(
                            &[
                                "Parameter",
                                "EPS",
                                "Estimate",
                                "SE (RSE%)",
                                "Shrinkage (%)",
                                "Fixed",
                            ],
                            &sigma_rows,
                        );
                    }
                }
            }
            NonmemCommands::Lineage { folder, from, to } => {
                // Validate that only one of from/to is provided
                if from.is_some() && to.is_some() {
                    bail!("Cannot specify both --from and --to flags. Use one or the other.");
                }

                // Load lineage tree from folder
                let lineage_tree = LineageTree::from_folder(&folder)?;

                // Get the models to display based on flags
                let models = if let Some(from_model) = from {
                    lineage_tree.get_tree_from(&from_model)
                } else if let Some(to_model) = to {
                    lineage_tree.get_tree_up_to(&to_model)
                } else {
                    lineage_tree.get_all_models_in_order()
                };

                if models.is_empty() {
                    println!("No models found in lineage tree.");
                    return Ok(());
                }

                // Build table rows
                let mut rows = Vec::new();
                for (model_name, model_metadata) in &models {
                    let row = build_lineage_row(&lineage_tree, model_name, model_metadata);
                    rows.push(row);
                }

                // Print table
                print_table(
                    &[
                        "Model",
                        "Parents",
                        "Runtime",
                        "Dataset Hash",
                        "Model Hash",
                        "Description",
                        "Tags",
                    ],
                    &rows,
                );
            }
        },
    }

    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e:?}");
        std::process::exit(1)
    }
}
