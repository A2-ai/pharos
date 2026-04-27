use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use config::{CONFIG_FILENAME, Config, NonmemConfig, find_config_dir, render_output_dir_template};
use fs_err as fs;
use nonmem::expand_model_pattern;
use nonmem::output_files::ext::ParameterType;
use nonmem::output_files::get_summary;
use nonmem::{CopyOptions, LineageTree, RunOptions, check_model, copy_model, run_models};
use scheduler::{SchedulerType, sge, slurm};
use serde_json::json;

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
    /// Whether to enable logging
    #[clap(long, global = true)]
    verbose: bool,
    /// Path to a specific pharos.toml config file. By default we'll search
    /// from the current directory and upwards until we find it or a .git folder
    #[clap(long, global = true)]
    config_file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    Nonmem {
        #[command(subcommand)]
        nonmem_command: NonmemCommands,
    },
}

#[derive(Subcommand)]
pub enum NonmemSlurm {
    /// Submits the given model to SLURM
    Submit {
        /// Submit options for NONMEM execution
        #[clap(flatten)]
        submit_options: slurm::SubmitOptions,
        #[clap(flatten)]
        run_options: RunOptions,
    },
}

#[derive(Subcommand)]
pub enum NonmemSge {
    /// Submits the given model to SGE
    Submit {
        /// Submit options for NONMEM execution
        #[clap(flatten)]
        submit_options: sge::SubmitOptions,
        #[clap(flatten)]
        run_options: RunOptions,
    },
}

#[derive(Subcommand)]
pub enum NonmemMetadata {
    /// Create new metadata or completely replace existing metadata
    Set {
        /// Path to the model file (.mod or .ctl)
        model_path: PathBuf,
        /// Description of the model
        #[clap(long)]
        description: Option<String>,
        /// Comma-separated list of tags
        #[clap(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Comma-separated list of model paths this one is based on (relative to model directory)
        #[clap(long, value_delimiter = ',')]
        based_on: Vec<String>,
    },
    /// Append to existing metadata (file must already exist)
    Append {
        /// Path to the model file (.mod/.ctl) or metadata file (_metadata.json)
        input: PathBuf,
        /// Description to append
        #[clap(long)]
        description: Option<String>,
        /// Comma-separated list of tags to add
        #[clap(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Comma-separated list of models to add to based_on
        #[clap(long, value_delimiter = ',')]
        based_on: Vec<String>,
    },
    /// Clear specified metadata fields
    Clear {
        /// Path to the model file (.mod or .ctl)
        model_path: PathBuf,
        /// Clear the based_on field
        #[clap(long)]
        based_on: bool,
        /// Clear the copied_from field
        #[clap(long)]
        copied_from: bool,
        /// Clear the tags field
        #[clap(long)]
        tags: bool,
    },
}

#[derive(Subcommand)]
pub enum NonmemCommands {
    /// Creates a pharos.toml file for nonmem models
    Init,
    /// Checks the model file with nonmem without running the model.
    /// This will the executables for nonmem version selected in pharos.toml
    Check { model: String },
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
        /// Hide off-diagonal omega/sigma estimates (shown by default if not fixed)
        #[clap(long)]
        hide_off_diagonals: bool,
        /// Highlight parameters that have a correlation higher than that threshold.
        /// If not set will pick the value from the pharos.toml file which defaults to 0.95
        #[clap(long)]
        correlation_threshold: Option<f64>,
        /// How many significant digits should we show for the numbers in the summary
        /// Defaults to the max number of sig digits found in the summary
        #[clap(long)]
        significant_digits: Option<usize>,
        /// When using --json, include all correlations instead of filtering by threshold
        #[clap(long, requires = "json")]
        include_all_correlations_json: bool,
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
    /// All commands to interact with slurm for nonmem runs
    Slurm {
        #[command(subcommand)]
        slurm_nonmem: NonmemSlurm,
    },
    /// All commands to interact with SGE for nonmem runs
    Sge {
        #[command(subcommand)]
        sge_nonmem: NonmemSge,
    },
    /// Manage model metadata
    Metadata {
        #[command(subcommand)]
        command: NonmemMetadata,
    },
    /// Checks the status of the current setup
    Sitrep,
}

fn find_output_folder(
    config: &NonmemConfig,
    model_path: impl AsRef<Path>,
) -> Result<Option<PathBuf>> {
    let model_path = model_path.as_ref();

    let model_name = model_path
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("Could not determine model file stem"))?
        .to_string_lossy();

    let root_folder = model_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not determine parent directory"))?;

    // First look up if there is an output dir
    let mut possible_folders = vec![model_name.as_ref().to_string()];

    if let Some(o) = &config.output_dir
        && let Ok(o2) = render_output_dir_template(o, model_name.as_ref())
    {
        possible_folders.push(o2);
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

    env_logger::Builder::new()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Off
        })
        .init();

    let load_nonmem_config = |run_nonmem_version: Option<&str>| -> Result<(PathBuf, NonmemConfig)> {
        let p = if let Some(config_path) = cli.config_file {
            config_path
        } else if let Some(root_dir) = find_config_dir()? {
            root_dir.join(CONFIG_FILENAME)
        } else {
            std::env::current_dir()?.join(CONFIG_FILENAME)
        };

        if !p.exists() {
            bail!("pharos config file not found in current or parent directories");
        }

        let config = Config::load(&p)?;
        let nonmem_config = match config.nonmem {
            Some(config) => config,
            None => bail!("pharos config file does not contain nonmem configuration"),
        };

        if let Some(version) = run_nonmem_version
            && !nonmem_config.versions.contains_key(version)
        {
            bail!("nonmem version {version} not found in config file");
        }

        Ok((p, nonmem_config))
    };

    match cli.command {
        Commands::Nonmem { nonmem_command } => match nonmem_command {
            NonmemCommands::Init => {
                if let Some(p) = find_config_dir()? {
                    bail!("Config file already exists in {p:?}");
                }

                let mut config_file = fs::File::create(CONFIG_FILENAME)?;
                let config = toml::to_string_pretty(&Config::new_nonmem()?)?;
                config_file.write_all(config.as_bytes())?;
                println!("pharos config file created");
            }
            NonmemCommands::Check { model } => {
                let (_, nonmem_config) = load_nonmem_config(None)?;

                match check_model(&nonmem_config, Path::new(&model)) {
                    Err(e) => eprintln!("{e:#}"),
                    Ok(res) if res.success => {
                        println!("{}", res.stdout);
                    }
                    Ok(res) => {
                        eprintln!(
                            "{}\nnmtran failed with exit code {:?}",
                            res.stdout, res.exit_code
                        );
                    }
                }
            }
            NonmemCommands::Run { model, run_options } => {
                let (config_path, nonmem_config) =
                    load_nonmem_config(run_options.nonmem_version.as_deref())?;

                // Expand model pattern to get all model files
                let model_files = expand_model_pattern(&model)?;
                for model_file in &model_files {
                    if !model_file.exists() {
                        bail!("Model file does not exist: {}", model_file.display());
                    }
                }
                log::debug!("Going to run: {model_files:?}");
                let config_dir = config_path
                    .parent()
                    .expect("config file to have a parent dir")
                    .canonicalize()?;

                run_models(&nonmem_config, &model_files, &run_options, &config_dir)?;
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
                let (_, config) = load_nonmem_config(None)?;

                // Validate ext file if parameter updates are requested
                if copy_options.is_updating_params() {
                    let ext_path = match &ext_file {
                        Some(path) => PathBuf::from(path),
                        None => find_output_folder(&config, from)?.unwrap_or_default(),
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
            NonmemCommands::Summary {
                directory,
                json,
                significant_digits,
                hide_off_diagonals,
                correlation_threshold,
                include_all_correlations_json,
            } => {
                let (_, config) = load_nonmem_config(None)?;

                let comment_type = config.comments.r#type;
                let correlation_threshold = if let Some(c) = correlation_threshold {
                    c
                } else {
                    config.summary.high_correlation_threshold
                };

                let mut summary = match get_summary(&directory, comment_type, hide_off_diagonals) {
                    Ok(s) => s,
                    Err(e) => {
                        if json {
                            let json_output = json!({"error": e.to_string()});
                            println!("{}", json_output);
                            std::process::exit(1);
                        } else {
                            return Err(e);
                        }
                    }
                };

                if json {
                    if let Some(ref mut correlation_matrix) = summary.correlation_matrix {
                        if include_all_correlations_json {
                            // Generate full matrix with all parameter pairs (including 0.0 correlations)
                            correlation_matrix.fill_missing_correlations();
                        } else {
                            // Filter correlations by threshold
                            correlation_matrix
                                .correlations
                                .retain(|entry| entry.value.abs() >= correlation_threshold);
                        }
                    }

                    let json_output = serde_json::to_string_pretty(&summary)?;
                    println!("{}", json_output);
                } else {
                    println!("=== {} Summary ===", summary.run_name);
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
                    for m in &summary.minimization_results {
                        match m.ofv {
                            Some(o) => println!(" - {:.3}", o),
                            None => println!(" - N/A"),
                        }
                    }
                    println!();
                    if summary
                        .minimization_results
                        .iter()
                        .any(|m| m.condition_number.is_some())
                    {
                        println!("Condition Number:");
                        for m in &summary.minimization_results {
                            match m.condition_number {
                                Some(o) => println!(" - {:.3}", o),
                                None => println!(" - N/A"),
                            }
                        }
                        println!();
                    }
                    if summary
                        .minimization_results
                        .iter()
                        .any(|m| m.termination_code.is_some())
                    {
                        println!("Termination Code:");
                        for m in &summary.minimization_results {
                            match m.termination_code {
                                Some(c) => println!(" - {c}"),
                                None => println!(" - None"),
                            }
                        }
                        println!();
                    }

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
                        let sig_dig = significant_digits.unwrap_or_else(|| {
                            summary.get_num_significant_digits(ParameterType::Theta)
                        });

                        println!("THETA Parameters:");
                        let theta_rows: Vec<Vec<String>> = summary
                            .parameters
                            .theta
                            .iter()
                            .map(|theta| theta.as_string_pieces(sig_dig))
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
                        let sig_dig = significant_digits.unwrap_or_else(|| {
                            summary.get_num_significant_digits(ParameterType::Omega)
                        });
                        println!("OMEGA Parameters:");
                        let omega_rows: Vec<Vec<String>> = omega_params
                            .iter()
                            .map(|omega| omega.as_string_pieces(sig_dig))
                            .collect();
                        print_table(
                            &[
                                "Parameter",
                                "ETA",
                                "Estimate",
                                "SE (RSE%)",
                                "Shrinkage (%)",
                                "Fixed",
                                "Diagonal",
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
                        let sig_dig = significant_digits.unwrap_or_else(|| {
                            summary.get_num_significant_digits(ParameterType::Sigma)
                        });
                        println!("SIGMA Parameters:");
                        let sigma_rows: Vec<Vec<String>> = sigma_params
                            .iter()
                            .map(|sigma| sigma.as_string_pieces(sig_dig))
                            .collect();
                        print_table(
                            &[
                                "Parameter",
                                "EPS",
                                "Estimate",
                                "SE (RSE%)",
                                "Shrinkage (%)",
                                "Fixed",
                                "Diagonal",
                            ],
                            &sigma_rows,
                        );
                        println!();
                    }

                    let high_correlation_params = if let Some(cm) = &summary.correlation_matrix {
                        cm.get_parameters_over_threshold(correlation_threshold)
                    } else {
                        Vec::new()
                    };

                    if !high_correlation_params.is_empty() {
                        println!("High Correlation Parameters:");
                        let rows: Vec<_> = high_correlation_params
                            .iter()
                            .map(|((p1, p2), val)| vec![format!("{p1}-{p2}"), val.to_string()])
                            .collect();
                        print_table(&["Parameters", "Correlation"], &rows);
                        println!();
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
            NonmemCommands::Slurm { slurm_nonmem } => match slurm_nonmem {
                NonmemSlurm::Submit {
                    submit_options,
                    run_options,
                } => {
                    let model_files = expand_model_pattern(&submit_options.model)?;
                    for model_file in &model_files {
                        if !model_file.exists() {
                            bail!("Model file does not exist: {}", model_file.display());
                        }
                    }

                    // Grab cli --verbose flag for RunOptions
                    let run_options = RunOptions {
                        verbose: cli.verbose,
                        ..run_options
                    };

                    log::debug!("Going to submit to slurm: {model_files:?}");
                    let (config_path, nonmem_config) = load_nonmem_config(None)?;
                    let pharos_exe_path = std::env::current_exe()?;
                    let scheduler = SchedulerType::new_slurm(submit_options);
                    let res = scheduler.submit(
                        &config_path
                            .parent()
                            .expect("config file to have a parent dir")
                            .canonicalize()?,
                        model_files,
                        run_options,
                        nonmem_config,
                        pharos_exe_path,
                    )?;

                    for (p, job_id) in res {
                        println!("Model {p:?} -> job ID {job_id}");
                    }
                }
            },
            NonmemCommands::Sge { sge_nonmem } => match sge_nonmem {
                NonmemSge::Submit {
                    submit_options,
                    run_options,
                } => {
                    // Expand model pattern to get all model files
                    let model_files = expand_model_pattern(&submit_options.model)?;
                    for model_file in &model_files {
                        if !model_file.exists() {
                            bail!("Model file does not exist: {}", model_file.display());
                        }
                    }

                    // Grab cli --verbose flag for RunOptions
                    let run_options = RunOptions {
                        verbose: cli.verbose,
                        ..run_options
                    };

                    log::debug!("Going to submit to sge: {model_files:?}");
                    let (config_path, nonmem_config) = load_nonmem_config(None)?;
                    let pharos_exe_path = std::env::current_exe()?;

                    let scheduler = SchedulerType::new_sge(submit_options);
                    let res = scheduler.submit(
                        &config_path
                            .parent()
                            .expect("config file to have a parent dir")
                            .canonicalize()?,
                        model_files,
                        run_options,
                        nonmem_config,
                        pharos_exe_path,
                    )?;

                    for (p, job_id) in res {
                        println!("Model {p:?} submitted: job id {job_id}");
                    }
                }
            },
            NonmemCommands::Metadata { command } => match command {
                NonmemMetadata::Set {
                    model_path,
                    description,
                    tags,
                    based_on,
                } => {
                    if let Some(d) = &description
                        && d.trim().is_empty()
                    {
                        bail!("Description cannot be empty.")
                    };

                    let path = nonmem::update_metadata_file(
                        model_path,
                        description,
                        tags,
                        based_on,
                        true, // Use overwrite=true for 'set' command
                    )?;
                    println!("Metadata file set at {path:?}");
                }
                NonmemMetadata::Append {
                    input,
                    description,
                    tags,
                    based_on,
                } => {
                    let path = nonmem::update_metadata_file(
                        input,
                        description,
                        tags,
                        based_on,
                        false, // Use overwrite=false for 'append' command
                    )?;
                    println!("Metadata file updated at {path:?}");
                }
                NonmemMetadata::Clear {
                    model_path,
                    based_on,
                    copied_from,
                    tags,
                } => {
                    let (model_name, model_dir) = nonmem::validate_model_path(&model_path)?;
                    let metadata_path = model_dir.join(format!("{model_name}_metadata.json"));

                    // Check if metadata file exists
                    if !metadata_path.exists() {
                        bail!(
                            "Metadata file does not exist: {}. Use 'set' or 'append' to create metadata first.",
                            metadata_path.display()
                        );
                    }

                    let path = nonmem::clear_metadata_file(
                        model_name,
                        model_dir,
                        metadata_path,
                        based_on,
                        copied_from,
                        tags,
                    )?;
                    println!("Metadata fields cleared at {path:?}");
                }
            },
            NonmemCommands::Sitrep => {
                let (config_path, nonmem_config) = load_nonmem_config(None)?;
                let sitrep_results = nonmem_config.validate();

                // Configuration Section
                println!("### Configuration\n");
                println!("Valid config loaded from: {config_path:?}");
                // Helper functions for sitrep output formatting

                fn print_status_line(check: bool, message: &str) {
                    let icon = if check { "✅" } else { "❌" };
                    println!("{} {}", icon, message);
                }

                // Check default version
                // Check for custom templates
                if let Some(slurm_template) = &sitrep_results.slurm_template {
                    let slurm_msg =
                        format!("SLURM template found at {}", slurm_template.path.display());
                    print_status_line(slurm_template.found, &slurm_msg);
                }

                if let Some(sge_template) = &sitrep_results.sge_template {
                    let sge_msg = format!("SGE template found at {}", sge_template.path.display());
                    print_status_line(sge_template.found, &sge_msg);
                }
                println!();

                println!("### NONMEM Installations\n");

                if sitrep_results.default_version.defined && sitrep_results.default_version.valid {
                    print_status_line(
                        true,
                        "Default NONMEM version was found in the config and executable was found",
                    )
                } else if sitrep_results.default_version.defined
                    && !sitrep_results.default_version.valid
                {
                    print_status_line(
                        false,
                        &format!(
                            "Default NONMEM version {} is defined in pharos.toml but we could not find the executable",
                            sitrep_results.default_version.name
                        ),
                    )
                } else {
                    print_status_line(
                        false,
                        "Default NONMEM version is not defined in pharos.toml",
                    )
                }
                println!();

                if sitrep_results.nonmem_installations.is_empty() {
                    println!("❌ No NONMEM installations configured");
                } else {
                    for installation in &sitrep_results.nonmem_installations {
                        println!("NONMEM version: {}", installation.name);

                        if let Some(nmfe_path) = &installation.nmfe {
                            print_status_line(
                                true,
                                &format!("nmfe found at {}", nmfe_path.display()),
                            );
                        } else {
                            print_status_line(false, "nmfe executable not found");
                        }

                        if let Some(nmtran_path) = &installation.nmtran {
                            print_status_line(
                                true,
                                &format!("nmtran found at {}", nmtran_path.display()),
                            );
                        } else {
                            print_status_line(false, "nmtran executable not found");
                        }
                        println!();
                    }
                }

                if let Some(mpi_info) = &sitrep_results.mpi_info {
                    println!("### MPI Configuration\n");
                    if mpi_info.mpi.found {
                        print_status_line(
                            true,
                            &format!("mpiexec located at {}", mpi_info.mpi.path.display()),
                        );
                    } else {
                        print_status_line(
                            false,
                            &format!("mpiexec not found at {}", mpi_info.mpi.path.display()),
                        );
                    }

                    if let Some(version_output) = &mpi_info.version_output {
                        println!("MPI version details");
                        println!();
                        println!("{version_output}");
                        println!();
                    }
                }

                println!();
                if sitrep_results.has_errors() {
                    println!("⚠️  Some issues detected - please review the status above");
                } else {
                    println!("🎉 All systems operational!");
                }
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
