use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use config::{CONFIG_FILENAME, Config, NonmemConfig, find_config_dir};
use fs_err as fs;
use nonmem::expand_model_pattern;
use nonmem::output_files::ext::ParameterType;
use nonmem::output_files::{get_summary, resolve_estimation_files};
use nonmem::scm;
use nonmem::{
    CopyOptions, LineageTree, Model, ModelLayout, RUN_END_FILENAME, RunOptions,
    TERMINATION_FILENAME, Termination, check_model, copy_model, run_models,
    validate_model_extension,
};
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
    #[clap(long, global = true, help_heading = "Global Options")]
    verbose: bool,
    /// Path to a specific pharos.toml config file. By default we'll search
    /// from the current directory and upwards until we find it or a .git folder
    #[clap(long, global = true, help_heading = "Global Options")]
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
        /// Path of the model this was copied from (relative to model directory)
        #[clap(long)]
        copied_from: Option<String>,
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
pub enum NonmemScm {
    /// Discover and validate covariate candidates, write plan.json. Runs nothing.
    Plan {
        /// The template control stream: candidate effects written into $PK
        /// and `(0 FIX)`'d in $THETA. Candidate names come from each theta's
        /// comment (`THETA<n>` if it has none)
        model: PathBuf,
        /// 1-based THETA numbers of the candidate covariate effects, e.g. 6,7,8
        #[clap(long, value_delimiter = ',', required = true)]
        covariates: Vec<usize>,
        /// Which phases to run: forward, backward, or forward,backward
        #[clap(long, value_delimiter = ',', required = true)]
        direction: Vec<scm::Direction>,
        /// Significance level for adding a covariate in forward selection
        #[clap(long, default_value_t = 0.05)]
        forward_alpha: f64,
        /// Significance level for keeping a covariate in backward elimination
        #[clap(long, default_value_t = 0.001)]
        backward_alpha: f64,
        /// Pause after this many rounds per invocation (the search is resumable)
        #[clap(long)]
        num_rounds: Option<usize>,
        /// Retries per failed fit; each retry starts from the previous
        /// attempt's estimates (never jittered)
        #[clap(long, default_value_t = 3)]
        max_retries: usize,
        /// Initial estimate a covariate theta is released at
        #[clap(long, default_value_t = 0.1)]
        release_init: f64,
        /// Whether generated models run the covariance step
        #[clap(long, default_value_t = true, action = clap::ArgAction::Set)]
        cov_step: bool,
        /// Replace existing SCM output from a different plan in out_dir
        #[clap(long)]
        overwrite: bool,
        /// Output directory (defaults to scm/<model name> beside the model)
        #[clap(long)]
        out_dir: Option<PathBuf>,
        /// Print the plan as JSON instead of the human-readable rendering
        #[clap(long)]
        json: bool,
    },
    /// Run (or resume) the search described by a plan.json
    Run {
        /// Path to the plan.json written by `scm plan`
        #[clap(long)]
        plan: PathBuf,
        /// Fit rounds on the cluster instead of locally
        #[clap(long)]
        slurm: bool,
        /// Slurm partition (defaults to the pharos.toml / cluster default)
        #[clap(long)]
        partition: Option<String>,
        /// Slurm account
        #[clap(long)]
        account: Option<String>,
        /// How many models to fit in parallel when running locally
        #[clap(long)]
        num_parallel: Option<usize>,
        /// Cap on slurm jobs in flight at once (0 = no cap); further models
        /// are submitted as earlier ones finish
        #[clap(long, default_value_t = 6)]
        max_concurrent: usize,
        /// Override the plan's num_rounds for this invocation only: a number
        /// pauses after that many rounds now, `all` runs to completion.
        /// The plan file is not modified.
        #[clap(long)]
        num_rounds: Option<RoundsArg>,
    },
    /// Report where a search stands (rounds done, models running, retries used)
    Status {
        /// The SCM output directory, or its plan.json
        path: PathBuf,
        /// Print the status as JSON
        #[clap(long)]
        json: bool,
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
    /// Show model lineage and relationships.
    ///
    /// With no arguments, prints the full project lineage tree. Supplying a
    /// model path prints that model's full lineage (ancestors and
    /// descendants). The `--from` and `--to` flags filter the tree from a
    /// model downward, up to a model, or to the slice between two models.
    Lineage {
        /// Optional model file. Shows the model's full lineage (ancestors
        /// and descendants). Conflicts with --from/--to.
        #[clap(conflicts_with_all = ["from", "to"])]
        path: Option<PathBuf>,

        /// Filter the tree to this model and everything downstream.
        #[clap(long)]
        from: Option<PathBuf>,

        /// Filter the tree to this model and everything upstream.
        #[clap(long)]
        to: Option<PathBuf>,
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
    /// Stepwise covariate modeling: plan, run, and inspect an SCM search
    Scm {
        #[command(subcommand)]
        command: NonmemScm,
    },
    /// Manage model metadata
    Metadata {
        #[command(subcommand)]
        command: NonmemMetadata,
    },
    /// Migrate run start files (pharos_start.json) written by older pharos
    /// versions from an absolute model_canonical_path to a model_path
    /// relative to the project root
    MigrateRunStart {
        /// Project root the runs were originally recorded under, used when
        /// the recorded absolute paths are not under the current project
        /// root (e.g. /data/user-homes/analyst1/Projects/project-root)
        #[clap(long)]
        base_path: Option<PathBuf>,
    },
    /// Checks the status of the current setup
    Sitrep,
}

/// `--num-rounds` argument for `scm run`: a per-invocation cap, or `all` to
/// run to completion regardless of the plan's num_rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundsArg {
    All,
    Limit(usize),
}

impl std::str::FromStr for RoundsArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("all") {
            return Ok(RoundsArg::All);
        }
        match s.parse::<usize>() {
            Ok(n) if n >= 1 => Ok(RoundsArg::Limit(n)),
            _ => Err(format!(
                "expected a round count of at least 1, or 'all'; got '{s}'"
            )),
        }
    }
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

    if let Some(p) = cli.config_file.as_ref()
        && let Some(parent) = p.parent()
    {
        config::set_config_dir(parent.to_path_buf());
    }

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
                let model_path = Path::new(&model);
                validate_model_extension(model_path)?;
                let (_, nonmem_config) = load_nonmem_config(None)?;

                let res = check_model(&nonmem_config, model_path)?;
                if res.success {
                    println!("{}", res.stdout);
                } else {
                    eprintln!(
                        "{}\nnmtran failed with exit code {:?}",
                        res.stdout, res.exit_code
                    );
                    std::process::exit(if res.exit_code != 0 { res.exit_code } else { 1 });
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
                    validate_model_extension(model_file)?;
                }
                log::debug!("Going to run: {model_files:?}");
                let config_dir = config_path
                    .parent()
                    .expect("config file to have a parent dir")
                    .canonicalize()?;

                let exit_code =
                    run_models(&nonmem_config, &model_files, &run_options, &config_dir)?;
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
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
                // Validate from file exists and has a supported extension
                let from = Path::new(&from);
                if !from.exists() {
                    bail!("Model file does not exist: {}", from.display());
                }
                let from_ext = validate_model_extension(from)?;
                let original_filename = match from.file_name() {
                    Some(filename) => filename.to_string_lossy().to_string(),
                    None => bail!("`from` model file does not have a file name"),
                };

                // If `to` lacks an extension, inherit it from `from`.
                let to = if to.extension().is_none() {
                    to.with_extension(from_ext)
                } else {
                    to
                };
                let to = to.as_path();
                validate_model_extension(to)?;

                // Validate to file doesn't exist or overwrite is allowed
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
                // Ensure copy runs inside a pharos project (matches prior behavior).
                let _ = load_nonmem_config(None)?;

                // Validate ext file if parameter updates are requested
                if copy_options.is_updating_params() {
                    let ext_path = match &ext_file {
                        Some(path) => PathBuf::from(path),
                        None => {
                            // Discover the model's actual run output from the pharos metadata
                            let layout = ModelLayout::from_model_file(from)?;
                            let project_root =
                                fs::canonicalize(find_config_dir()?.ok_or_else(|| {
                                    anyhow!("No pharos.toml found in this directory or any parent.")
                                })?)?;
                            let run_dir = layout
                                .discover_output_dir(&project_root)?
                                .ok_or_else(|| {
                                    anyhow!(
                                        "Could not find any pharos run output for {}. \
                                         Use --ext-file to point at the parameter estimates file directly.",
                                        from.display()
                                    )
                                })?;
                            // Parse the source model so we can honor `$EST FILE=` overrides when
                            // discovering the .ext file. If parsing fails, fall through to the default
                            // name — the parse error will resurface in copy_model.
                            let default = layout.output_file(&run_dir, "ext");
                            let model = fs::read_to_string(from)
                                .ok()
                                .and_then(|s| Model::parse(from, &s).ok());
                            match &model {
                                Some(m) => resolve_estimation_files(m, &run_dir, &default)
                                    .last()
                                    .cloned()
                                    .unwrap_or(default),
                                None => default,
                            }
                        }
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

                    // Do not allow copying a run that failed or didn't finish
                    if let Some(run_dir) = ext_path.parent() {
                        let terminated = run_dir.join(TERMINATION_FILENAME);
                        if terminated.exists() {
                            let termination: Termination =
                                serde_json::from_reader(fs::File::open(&terminated)?)?;
                            bail!("Cannot copy estimates: the run was terminated.\n{termination}");
                        }

                        if ext_file.is_none() && !run_dir.join(RUN_END_FILENAME).exists() {
                            bail!(
                                "Run does not seem finished, no {RUN_END_FILENAME} found next to {}. If it's complete, pass --ext-file to point at the file directly",
                                ext_path.display()
                            );
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
                    let methods = &summary.lst.run_details.estimation_methods;
                    let any_cond = summary
                        .minimization_results
                        .iter()
                        .any(|m| m.condition_number.is_some());
                    let any_term = summary
                        .minimization_results
                        .iter()
                        .any(|m| m.termination_code.is_some());

                    let n = methods.len().max(summary.minimization_results.len());
                    for i in 0..n {
                        let method = methods.get(i).map(|s| s.as_str()).unwrap_or("Unknown");
                        println!("Method: {}", method);
                        if let Some(m) = summary.minimization_results.get(i) {
                            match m.ofv {
                                Some(o) => println!(" - OFV: {:.3}", o),
                                None => println!(" - OFV: N/A"),
                            }
                            if any_cond {
                                match m.condition_number {
                                    Some(c) => println!(" - Condition Number: {:.3}", c),
                                    None => println!(" - Condition Number: N/A"),
                                }
                            }
                            if any_term {
                                match m.termination_code {
                                    Some(c) => println!(" - Termination Code: {c}"),
                                    None => println!(" - Termination Code: None"),
                                }
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
            NonmemCommands::Lineage { path, from, to } => {
                let lineage_tree = LineageTree::from_project()?;

                let models = if let Some(p) = path {
                    lineage_tree.lineage_of(&p)?
                } else {
                    lineage_tree.slice(from.as_deref(), to.as_deref())?
                };

                if models.is_empty() {
                    println!("No models found in lineage tree.");
                    return Ok(());
                }

                let mut rows = Vec::new();
                for (model_name, model_metadata) in &models {
                    let row = build_lineage_row(&lineage_tree, model_name, model_metadata);
                    rows.push(row);
                }

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
                        validate_model_extension(model_file)?;
                    }

                    // Grab cli --verbose flag for RunOptions
                    let run_options = RunOptions {
                        verbose: cli.verbose,
                        ..run_options
                    };

                    log::debug!("Going to submit to slurm: {model_files:?}");
                    let (config_path, nonmem_config) = load_nonmem_config(None)?;
                    let config_path = config_path.canonicalize()?;
                    let pharos_exe_path = std::env::current_exe()?;
                    let scheduler = SchedulerType::new_slurm(submit_options);
                    let res = scheduler.submit(
                        &config_path,
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
                        validate_model_extension(model_file)?;
                    }

                    // Grab cli --verbose flag for RunOptions
                    let run_options = RunOptions {
                        verbose: cli.verbose,
                        ..run_options
                    };

                    log::debug!("Going to submit to sge: {model_files:?}");
                    let (config_path, nonmem_config) = load_nonmem_config(None)?;
                    let config_path = config_path.canonicalize()?;
                    let pharos_exe_path = std::env::current_exe()?;

                    let scheduler = SchedulerType::new_sge(submit_options);
                    let res = scheduler.submit(
                        &config_path,
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
            NonmemCommands::Scm { command } => match command {
                NonmemScm::Plan {
                    model,
                    covariates,
                    direction,
                    forward_alpha,
                    backward_alpha,
                    num_rounds,
                    max_retries,
                    release_init,
                    cov_step,
                    overwrite,
                    out_dir,
                    json,
                } => {
                    let options = scm::ScmOptions {
                        direction,
                        forward_alpha,
                        backward_alpha,
                        num_rounds,
                        max_retries,
                        release_init,
                        cov_step,
                        overwrite,
                    };

                    let built = scm::build_plan(
                        &model,
                        &covariates,
                        out_dir.as_deref(),
                        options,
                        env!("CARGO_PKG_VERSION"),
                    )?;
                    let plan_path = built.plan.save()?;

                    for w in &built.warnings {
                        eprintln!("warning: {w}");
                    }
                    if json {
                        println!("{}", built.plan.to_json()?);
                    } else {
                        print!("{}", built.plan.render_text());
                        println!("\nplan written to {}", plan_path.display());
                    }
                }
                NonmemScm::Run {
                    plan,
                    slurm,
                    partition,
                    account,
                    num_parallel,
                    max_concurrent,
                    num_rounds,
                } => {
                    let mut plan = scm::ScmPlan::load(&plan)?;
                    // Per-invocation run control: num_rounds is excluded from
                    // the plan digest, so overriding it here never invalidates
                    // on-disk state, and the plan file itself is untouched.
                    if let Some(cap) = num_rounds {
                        plan.options.num_rounds = match cap {
                            RoundsArg::All => None,
                            RoundsArg::Limit(n) => Some(n),
                        };
                        eprintln!(
                            "num_rounds for this invocation: {}",
                            match cap {
                                RoundsArg::All => "uncapped (run to completion)".to_string(),
                                RoundsArg::Limit(n) => format!("pause after {n}"),
                            }
                        );
                    }
                    let (config_path, nonmem_config) = load_nonmem_config(None)?;

                    let executor: Box<dyn scm::FitExecutor> = if slurm {
                        Box::new(scheduler::ScmSlurmExecutor {
                            config_path: config_path.canonicalize()?,
                            nonmem_config,
                            pharos_exe: std::env::current_exe()?,
                            partition,
                            account,
                            max_concurrent,
                        })
                    } else {
                        let config_dir = config_path
                            .parent()
                            .expect("config file to have a parent dir")
                            .to_path_buf();
                        Box::new(scm::LocalExecutor {
                            nonmem_config,
                            config_dir,
                            num_parallel,
                        })
                    };

                    let outcome = scm::run_scm(&plan, executor.as_ref())?;
                    let status = scm::read_status(&plan.out_dir_path())?;
                    print!("{}", status.render_text());

                    match outcome.state.status {
                        scm::ScmRunStatus::Completed if outcome.state.had_unusable => {
                            eprintln!(
                                "\nsearch completed with unusable candidates — see the decision log"
                            );
                            std::process::exit(2);
                        }
                        scm::ScmRunStatus::Completed | scm::ScmRunStatus::Paused => {}
                        other => {
                            bail!("SCM search ended in unexpected state: {other}");
                        }
                    }
                }
                NonmemScm::Status { path, json } => {
                    let out_dir = if path.is_file() {
                        path.parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| PathBuf::from("."))
                    } else {
                        path
                    };
                    let status = scm::read_status(&out_dir)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&status)?);
                    } else {
                        print!("{}", status.render_text());
                    }
                }
            },
            NonmemCommands::Metadata { command } => match command {
                NonmemMetadata::Set {
                    model_path,
                    description,
                    tags,
                    based_on,
                    copied_from,
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
                        copied_from,
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
                        None,
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
                    validate_model_extension(&model_path)?;
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
            NonmemCommands::MigrateRunStart { base_path } => {
                let (config_path, _) = load_nonmem_config(None)?;
                let project_root = config_path
                    .parent()
                    .expect("config file to have a parent dir")
                    .canonicalize()?;

                let report = nonmem::MigrationReport::migrate_run_start_files(
                    &project_root,
                    base_path.as_deref(),
                )?;
                println!(
                    "{} file(s) migrated, {} already migrated",
                    report.migrated, report.skipped
                );
                if !report.failed.is_empty() {
                    eprintln!("Failed to migrate {} file(s):", report.failed.len());
                    for (path, reason) in &report.failed {
                        eprintln!(" - {}: {reason}", path.display());
                    }
                    std::process::exit(1);
                }
            }
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
