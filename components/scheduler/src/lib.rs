use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as AnyhowContext, Result, anyhow, bail};
use config::NonmemConfig;
use fs_err as fs;
use nonmem::{ModelLayout, RunOptions, check_model};
use serde_json::json;
use tera::{Context, Kwargs, State, Tera, TeraResult, Value};

pub mod sge;
pub mod slurm;

const SUBMISSIONS_DIR: &str = "submission-log";
const GITIGNORE: &[u8] = b"*\n!.gitignore";

/// Tera filter that shell-quotes its input so interpolated paths/flags cannot
/// word-split or inject shell metacharacters in the generated submission script.
pub(crate) fn shquote_filter(value: &Value, _kwargs: Kwargs, _state: &State) -> TeraResult<Value> {
    fn quote(s: &str) -> TeraResult<String> {
        shlex::try_quote(s)
            .map(|c| c.into_owned())
            .map_err(|e| tera::Error::message(format!("failed to shell-quote {s:?}: {e}")))
    }

    if let Some(s) = value.as_str() {
        Ok(Value::safe_string(&quote(s)?))
    } else if let Some(items) = value.as_array() {
        let mut parts = Vec::with_capacity(items.len());
        for item in items {
            let s = item
                .as_str()
                .ok_or_else(|| tera::Error::message("shquote array elements must be strings"))?;
            parts.push(quote(s)?);
        }
        Ok(Value::safe_string(&parts.join(" ")))
    } else {
        Err(tera::Error::message(
            "shquote expects a string or an array of strings",
        ))
    }
}

/// Render a user-provided custom template with the same filters available as the
/// built-in templates (notably `shquote`).
pub(crate) fn render_custom_template(content: &str, context: &Context) -> TeraResult<String> {
    let mut tera = Tera::default();
    tera.register_filter("shquote", shquote_filter);
    tera.add_raw_template("job", content)?;
    tera.render("job", context)
}

pub(crate) fn get_or_create_gitignore(dir: impl AsRef<Path>) -> Result<PathBuf> {
    let gitignore = dir.as_ref().join(".gitignore");

    if !gitignore.exists() {
        let mut f = fs::File::create(&gitignore)?;
        f.write_all(GITIGNORE)?;
    }

    Ok(gitignore.canonicalize()?)
}

pub(crate) fn get_or_create_submissions_dir(parent: impl AsRef<Path>) -> Result<PathBuf> {
    let dir = parent.as_ref().join(SUBMISSIONS_DIR);
    fs::create_dir_all(&dir)?;
    get_or_create_gitignore(&dir)?;
    Ok(dir.canonicalize()?)
}

pub(crate) fn get_or_create_logs_dir(
    parent: impl AsRef<Path>,
    passed_log_dir: Option<PathBuf>,
    default_logs_dir: &str,
) -> Result<PathBuf> {
    let dir = if let Some(d) = passed_log_dir {
        d
    } else {
        parent.as_ref().join(default_logs_dir)
    };

    if dir.exists() {
        return Ok(dir);
    }

    fs::create_dir_all(&dir)?;
    get_or_create_gitignore(&dir)?;

    Ok(dir.canonicalize()?)
}

#[derive(Debug, PartialEq)]
pub enum SchedulerType {
    Slurm(slurm::SubmitOptions),
    Sge(sge::SubmitOptions),
}

impl SchedulerType {
    pub fn new_sge(options: sge::SubmitOptions) -> Self {
        SchedulerType::Sge(options)
    }

    pub fn new_slurm(options: slurm::SubmitOptions) -> Self {
        SchedulerType::Slurm(options)
    }

    fn kind(&self) -> &'static str {
        match self {
            SchedulerType::Slurm(_) => "slurm",
            SchedulerType::Sge(_) => "sge",
        }
    }

    fn get_logs_dir(&self, config_dir: &Path, config: &NonmemConfig) -> Result<PathBuf> {
        match self {
            SchedulerType::Slurm(_) => get_or_create_logs_dir(
                config_dir,
                config.slurm.log_folder(config_dir),
                slurm::SLURM_LOGS_DIR,
            ),
            SchedulerType::Sge(_) => get_or_create_logs_dir(
                config_dir,
                config.sge.log_folder(config_dir),
                sge::SGE_LOGS_DIR,
            ),
        }
    }

    fn job_name(&self) -> Option<&str> {
        match self {
            SchedulerType::Slurm(s) => s.job_name.as_deref(),
            SchedulerType::Sge(s) => s.job_name.as_deref(),
        }
    }

    fn submit_command_name(&self) -> &'static str {
        match self {
            SchedulerType::Slurm(_) => "sbatch",
            SchedulerType::Sge(_) => "qsub",
        }
    }

    fn template(&self, config: &NonmemConfig, config_dir: &Path) -> Option<PathBuf> {
        let (cli_val, config_val) = match self {
            SchedulerType::Slurm(s) => (s.template.clone(), config.slurm.template(config_dir)),
            SchedulerType::Sge(s) => (s.template.clone(), config.sge.template(config_dir)),
        };

        cli_val.or(config_val)
    }

    fn is_dry_run(&self) -> bool {
        match self {
            SchedulerType::Slurm(s) => s.dry_run,
            SchedulerType::Sge(s) => s.dry_run,
        }
    }

    pub fn submit(
        &self,
        config_path: &Path,
        models: Vec<PathBuf>,
        run_options: RunOptions,
        mut config: NonmemConfig,
        pharos_exe_path: PathBuf,
    ) -> Result<Vec<(PathBuf, usize)>> {
        let config_dir = config_path
            .parent()
            .expect("config file to have a parent dir");
        run_options.update_config_from_options(&mut config);
        let log_dir = self.get_logs_dir(config_dir, &config)?;
        log::debug!("Log dir: {log_dir:?}");
        let submission_dir = get_or_create_submissions_dir(config_dir)?;
        log::debug!("Submission dir: {submission_dir:?}");
        let num_cpus = config.parallel.resolve_num_cpus(config_dir);

        // We do 2 loops: one to get all the info and generate the script and another one to actually
        // run them. Split so an error in one model doesn't result in a batch partially sent
        let nmtran_available = config.get_nmtrans_executable_path(None).is_ok();

        let mut jobs = vec![];
        for m in models {
            let m = m.canonicalize()?;

            if nmtran_available {
                let check_result = check_model(&config, &m)?;
                if !check_result.success {
                    bail!(
                        "Model check failed for {}: {}",
                        m.display(),
                        check_result.stdout.trim()
                    );
                }
            }

            let model_name = m.file_stem().unwrap().to_str().unwrap().to_string();
            let job_name = if let Some(job_name) = self.job_name() {
                job_name.to_string()
            } else {
                model_name.clone()
            };

            let output_template = run_options
                .output_dir
                .clone()
                .or_else(|| config.output_dir.clone());
            let output_dir =
                ModelLayout::from_model_file(&m)?.resolve_output_dir(output_template.as_deref())?;
            let parent_dir = m.parent().expect("to have a parent");

            if output_dir.is_dir() {
                if !run_options.overwrite {
                    bail!(
                        "Output directory already exists: {:?} and --overwrite not given for {m:?}",
                        output_dir
                    );
                }
                if !self.is_dry_run() {
                    if !output_dir.starts_with(parent_dir) || output_dir == parent_dir {
                        bail!(
                            "Cannot overwrite {output_dir:?}: outside the model directory or is the parent directory."
                        );
                    }
                    fs::remove_dir_all(&output_dir)?;
                }
            }

            log::debug!(
                "Model name: {model_name}, job name: {job_name}, output dir: {output_dir:?}"
            );

            // Pin the child to the exact directory we resolved (and
            // overwrite-checked) above.
            let resolved_output_name = output_dir
                .strip_prefix(parent_dir)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| output_dir.to_string_lossy().into_owned());
            let mut run_flags = run_options.run_flags();
            if let Some(pos) = run_flags.iter().position(|f| f == "--output-dir") {
                run_flags[pos + 1] = resolved_output_name;
            } else {
                run_flags.push("--output-dir".to_string());
                run_flags.push(resolved_output_name);
            }

            let mut vars = json!({
                "config_dir": config_dir,
                "job_name": job_name,
                "model_path": m,
                "model_name": model_name,
                "num_mpi_cpus": num_cpus,
                "pharos_exe_path": pharos_exe_path,
                "parallel": config.parallel.enabled,
                "run_flags": run_flags,
                "output_dir": output_dir,
                "config_path": config_path,
            });
            let obj = vars.as_object_mut().unwrap();

            let default_tera_instance = match self {
                SchedulerType::Slurm(s) => {
                    obj.insert("account".into(), json!(s.account));
                    obj.insert(
                        "log_path".into(),
                        json!(log_dir.join("%x_%j.out").to_str().unwrap()),
                    );

                    let actual_partition = slurm::resolve_partition(
                        s.partition.as_deref(),
                        config.slurm.partition.as_deref(),
                    )?;

                    log::debug!("Will use SLURM partition '{actual_partition}'");
                    obj.insert("partition".into(), json!(actual_partition));
                    &slurm::TERA
                }
                SchedulerType::Sge(_) => {
                    obj.insert(
                        "log_path".into(),
                        json!(log_dir.join(format!("{job_name}.log"))),
                    );
                    &sge::TERA
                }
            };

            let context = Context::from_serialize(&vars)
                .map_err(|e| anyhow!("failed to build template context for {m:?}: {e}"))?;

            let script = if let Some(tpl) = self.template(&config, config_dir) {
                let tpl_content = fs::read_to_string(&tpl)
                    .with_context(|| format!("failed to read template file {tpl:?}"))?;
                render_custom_template(&tpl_content, &context)
                    .with_context(|| format!("failed to render custom template {tpl:?}"))?
            } else {
                default_tera_instance
                    .render("job", &context)
                    .with_context(|| {
                        format!("failed to render default template for {}", self.kind())
                    })?
            };

            jobs.push((m, job_name, script, vars));
        }

        if self.is_dry_run() {
            for (m, _, script, vars) in jobs {
                println!("===");
                println!("Model: {m:?}");
                println!("Generated {} script:", self.kind());
                println!("```");
                println!("{script}");
                println!("```");
                println!("---");
                println!("Available variables:");
                for (key, val) in vars.as_object().expect("json! produces an object") {
                    println!("  -  {{{{ {key} }}}}: {val}");
                }
            }

            return Ok(vec![]);
        }

        let mut out = vec![];
        for (m, job_name, script, _) in jobs {
            let script_path = submission_dir.join(format!("{}_{job_name}.sh", self.kind()));
            fs::write(&script_path, &script)
                .with_context(|| format!("failed to write script to {script_path:?}",))?;

            let cmd_name = self.submit_command_name();
            log::debug!("Running {cmd_name} for {m:?}");
            let output = Command::new(cmd_name)
                .arg(script_path)
                .output()
                .with_context(
                    || format!("failed to execute {cmd_name} command for model {m:?}",),
                )?;

            // If SGE does not have a compute node ready during job submission
            // qsub fails and gives this error:
            //
            // Unable to run job: warning: <your-user-name's> job is not allowed to run in any queue
            // Your job <number> ("<model-name>") has been submitted
            // Exiting.
            //
            // Checking stderr for this message to prevent bail! on non-zero exit code from qsub
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let sge_queuing_warning = stderr.contains("Unable to run job: warning")
                    && stderr.contains("job is not allowed to run in any queue")
                    && stderr.contains("has been submitted");
                if !sge_queuing_warning {
                    bail!("{cmd_name} failed: {stderr}");
                }
                log::warn!("{cmd_name} reported a warning but the job was submitted: {stderr}");
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let job_id = match self {
                SchedulerType::Slurm(_) => {
                    let num = stdout.trim().replace("Submitted batch job ", "");
                    num.parse()
                        .map_err(|e| anyhow!("Failed to parse job ID '{stdout}': {e}"))?
                }
                SchedulerType::Sge(_) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    sge::parse_job_id(&stdout, &stderr)?
                }
            };

            out.push((m, job_id));
        }

        Ok(out)
    }
}
