mod check;
pub mod copy;
pub mod estimation;
mod files;
mod lineage;
mod metadata;
pub mod output_files;
mod parsing;
mod pattern;
mod prepare_model;
mod run_metadata;
pub mod runner;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Serialize;
use tempfile::{TempDir, tempdir, tempdir_in};
use utils::write_json_to_file;

use crate::files::{FileCopier, cleanup_unwanted_files};
use crate::prepare_model::prepare_model;
use crate::run_metadata::{OutputHashes, RUN_CONFIG_FILENAME};

pub use crate::pattern::expand_model_pattern;
pub use crate::run_metadata::{OutputFileHash, RunEndFile, RunStartFile};
pub use check::check_model;
pub use copy::{CopyOptions, copy_model};
pub use lineage::LineageTree;
pub use metadata::{ModelMetadata, clear_metadata_file, update_metadata_file, validate_model_path};
pub use parsing::{Dataset, Model};
pub use runner::{RunOptions, run_models};

fn get_utc_now() -> String {
    let now_utc = Timestamp::now().to_zoned(TimeZone::UTC);
    now_utc.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

const BASE_GITIGNORE: &str = r#"background.set
compile.lnk
FCON
FDATA
FDATA.csv
FMSG
FREPORT
FSIZES
FSTREAM
FSUBS
FSUBS.0
FSUBS.o
FSUBS_MU.F90
FSUBS.f90
fsubs.f90
FSUBS2
gfortran.txt
GFCOMPILE.BAT
INTER
licfile.set
linkc.lnk
LINK.LNK
LINKC.LNK
locfile.set
maxlim.set
newline
nmexec.set
nmpathlist.txt
nmprd4p.mod
nobuild.set
parafile.set
parafprint.set
prcompile.set
prdefault.set
prsame.set
PRSIZES.f90
rundir.set
runpdir.set
simparon.set
temp_dir
tprdefault.set
trskip.set
worker.set
xmloff.set
fort.2001
fort.2002
flushtime.set
nonmem
FPWARN
condorarguments.set
condoropenmpiscript.set
condor.set
mpiloc
nmmpi.sh
temp.out
trashfile.xxx
WK_[0-9]*
*.pnm
"#;

fn get_final_gitignore(model_name: &str) -> String {
    let mut output = BASE_GITIGNORE.to_string();

    output.push_str(&format!("\n{model_name}"));
    output.push_str(&format!("\n{model_name}_ETAS"));
    output.push_str(&format!("\n{model_name}_RMAT"));
    output.push_str(&format!("\n{model_name}_SMAT"));
    output.push_str(&format!("\n{model_name}.msf"));
    output.push_str(&format!("\n{model_name}_ETAS.msf"));
    output.push_str(&format!("\n{model_name}_RMAT.msf"));
    output.push_str(&format!("\n{model_name}_SMAT.msf"));

    output
}

#[derive(Debug, Serialize)]
pub struct NonmemRunner {
    model: PathBuf,
    overwrite: bool,
    run_in_output_dir: bool,
    config: NonmemConfig,
    nonmem_version: Option<String>,
    output_dir: Option<String>,
    #[serde(skip)]
    tempdir: Option<TempDir>,
    extra_files: Vec<String>,
}

fn generate_parafile(mpi_exec_path: &Path, total_nodes: u8, timeout: usize) -> Result<String> {
    // We will have validated that we have at least 2 nodes
    let worker_nodes = total_nodes - 1;
    // Parse type 2 refers to evenly load balanced work
    // Transfer Type 1 refers to MPI
    // TIMEOUTI 100 means wait 100 seconds for node to become available
    let parafile_content = format!(
        r#"$GENERAL
NODES={total_nodes} PARSE_TYPE=2 TIMEOUTI=100 TIMEOUT={timeout} PARAPRINT=0 TRANSFER_TYPE=1
$COMMANDS
1: {mpi_exec_path:?} -wdir "$PWD" -n 1 ./nonmem $*
2:-wdir "$PWD" -n {worker_nodes} ./nonmem -wnf
$DIRECTORIES
1:NONE
2-[nodes]:worker{{#-1}}
"#
    );

    Ok(parafile_content)
}

impl NonmemRunner {
    pub fn new(
        model: impl AsRef<Path>,
        overwrite: bool,
        config: NonmemConfig,
        nonmem_version: Option<String>,
        output_dir: Option<String>,
        extra_files: Vec<String>,
    ) -> NonmemRunner {
        Self {
            model: model.as_ref().to_owned(),
            overwrite,
            run_in_output_dir: false,
            config,
            nonmem_version,
            output_dir,
            extra_files,
            tempdir: None,
        }
    }

    pub fn run_in_output_dir(&mut self) {
        self.run_in_output_dir = true;
    }

    fn calculate_output_files_hashes(
        &self,
        output_dir: &Path,
        model_name: &str,
        output_files_rewrites: &HashMap<String, String>,
    ) -> Vec<OutputFileHash> {
        let mut files_to_hash = Vec::new();
        for ext in [
            ".ext", ".lst", ".grd", ".shk", ".cor", ".cov", ".coi", ".xml", ".clt", ".phi", ".msf",
            ".mod", ".ctl",
        ] {
            let filename = format!("{}{}", model_name, ext);
            let file_path = output_dir.join(&filename);
            if file_path.exists() {
                files_to_hash.push((filename, file_path));
            }
        }

        for rewritten_filename in output_files_rewrites.values() {
            let file_path = output_dir.join(rewritten_filename);
            if file_path.exists() {
                files_to_hash.push((rewritten_filename.clone(), file_path));
            }
        }

        files_to_hash.sort_by(|a, b| a.0.cmp(&b.0));

        let mut hashes = Vec::new();
        for (filename, p) in files_to_hash {
            match fs::read(&p) {
                Ok(data) => {
                    let blake3_hash = format!("{}", blake3::hash(&data));
                    hashes.push(OutputFileHash {
                        filename,
                        hashes: OutputHashes {
                            blake3: blake3_hash,
                        },
                    });
                }
                Err(e) => {
                    eprintln!("Warning: Could not read {}: {}", filename, e);
                }
            }
        }

        hashes
    }

    fn generate_script(&self, model_name: &str) -> Result<String> {
        let nonmem_path = self
            .config
            .get_nonmem_executable_path(self.nonmem_version.as_deref())?;
        let mut elems = vec![nonmem_path.to_string_lossy().into_owned()];
        elems.push(format!("{model_name}.mod"));
        elems.push(format!("{model_name}.lst"));

        // Add parafile flag if parallel execution is enabled
        if self.config.parallel.enabled {
            elems.push(format!("-parafile={model_name}.pnm"));
        }

        elems.extend(self.config.options.as_flags());

        Ok(elems.join(" "))
    }

    fn validate_parallel_config(&self) -> Result<()> {
        if !self.config.parallel.enabled {
            log::debug!("Parallel execution disabled");
            return Ok(());
        }

        // Check that MPI executable exists and is executable
        if let Some(mpiexec_path) = &self.config.parallel.mpiexec_path {
            if !mpiexec_path.exists() {
                bail!("MPI executable not found: {}", mpiexec_path.display());
            }
        } else {
            bail!("MPI executable not set in config file");
        }

        // Check that threads is at least 2
        if self.config.parallel.num_cpus < 2 {
            bail!(
                "Parallel execution requires at least 2 threads, got {}",
                self.config.parallel.num_cpus
            );
        }

        if let Some(ref p) = self.config.parallel.parafile
            && !p.exists()
        {
            bail!("Parafile {p:?} does not exist.",);
        }

        log::debug!("Parallel config is ok!");

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        // 0. Validate parallel configuration if enabled
        self.validate_parallel_config()?;

        // 1. We ensure the model and dataset exist, hashing it as well.
        let model_setup = prepare_model(
            &self.model,
            self.overwrite,
            self.output_dir.clone(),
            &self.config.comments,
        )?;
        log::debug!("Model output dir will be {:?}", model_setup.output_dir);

        // 2. We get started, finding where we will run things
        let running_dir = if self.run_in_output_dir {
            model_setup.output_dir.clone()
        } else {
            let tmp = if cfg!(target_os = "linux") {
                tempdir_in("/dev/shm")?
            } else {
                tempdir()?
            };
            let p = tmp.path().to_path_buf();
            self.tempdir = Some(tmp);
            p
        };
        log::debug!("Model {:?} will be running in {running_dir:?}", self.model);

        fs::create_dir_all(&model_setup.output_dir)?;
        fs::create_dir_all(&running_dir)?;
        // This will contain the canonicalized path to the dataset
        let mut f = fs::File::create(running_dir.join(format!("{}.mod", model_setup.name)))?;
        f.write_all(model_setup.model_content.as_bytes())?;
        // Create a .gitignore that ignores everything
        let mut f = fs::File::create(running_dir.join(".gitignore"))?;
        f.write_all("*".as_bytes())?;
        f.write_all("!.gitignore".as_bytes())?;
        // Add all extra files in it
        for extra in &self.extra_files {
            fs::copy(extra, running_dir.join(extra))?;
        }
        // Create the config snapshot
        write_json_to_file(&self, running_dir.join(RUN_CONFIG_FILENAME))?;

        // Create the run start dump
        let model_canonical_path = self.model.canonicalize()?;
        let start_file = RunStartFile::new(&model_setup, &model_canonical_path);
        let start = start_file.start.clone();
        start_file.save(&running_dir)?;

        // 3. Generate parafile if parallel execution is enabled
        let parallel = &self.config.parallel;
        if parallel.enabled {
            let parafile_path = running_dir.join(format!("{}.pnm", model_setup.name));
            if let Some(ref existing) = parallel.parafile {
                fs::copy(existing, parafile_path)?;
            } else {
                let parafile_content = generate_parafile(
                    &parallel.mpiexec_path.as_ref().unwrap(),
                    parallel.num_cpus,
                    parallel.timeout,
                )?;
                let mut f = fs::File::create(&parafile_path)?;
                f.write_all(parafile_content.as_bytes())?;
            }
        }

        // 4. We generate the script to run nonmem
        let script = self.generate_script(&model_setup.name)?;
        let script_path = running_dir.join(format!("{}.sh", model_setup.name));
        let mut f = fs::File::create(&script_path)?;
        f.write_all(script.as_bytes())?;

        // 5. Setup file copying if running in a different directory
        let need_file_copying = running_dir != model_setup.output_dir;

        // Combine configured patterns with output files found in model
        let mut all_patterns = self.config.files_to_copy().to_vec();
        for filename in model_setup.output_files.values() {
            if let Ok(pattern) = glob::Pattern::new(filename) {
                all_patterns.push(pattern);
            }
        }

        let mut file_copier = Some(FileCopier::default());
        let (copy_handle, shutdown_flag) = if need_file_copying {
            let running_dir_clone = running_dir.clone();
            let output_dir_clone = model_setup.output_dir.clone();
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_clone = Arc::clone(&shutdown);
            let mut copier = file_copier.take().unwrap();

            let handle = thread::spawn(move || {
                while !shutdown_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_secs(5));
                    if let Err(e) = copier.copy_changed_files(&running_dir_clone, &output_dir_clone)
                    {
                        eprintln!("Error copying files: {e}");
                    }
                }
                copier
            });
            (Some(handle), Some(shutdown))
        } else {
            (None, None)
        };

        // 6. Execute the script
        let script_start = Instant::now();
        let (mut recv, send) = std::io::pipe()?;

        let mut command = Command::new("sh")
            .arg(script_path.file_name().unwrap())
            .stdout(send.try_clone()?)
            .stderr(send)
            .current_dir(&running_dir)
            .spawn()?;

        let mut output = Vec::new();
        recv.read_to_end(&mut output)?;
        let status = command.wait()?;

        // 7. Stop background file copying and do final copy
        if need_file_copying {
            // Signal the background thread to stop
            if let Some(flag) = shutdown_flag {
                flag.store(true, Ordering::Relaxed);
            }
            // Wait for the background thread to finish and get the FileCopier
            if let Some(handle) = copy_handle {
                match handle.join() {
                    Ok(mut copier) => {
                        // Do final file copy with the same FileCopier instance
                        if let Err(e) =
                            copier.copy_changed_files(&running_dir, &model_setup.output_dir)
                        {
                            eprintln!("Error in final file copy: {e}");
                        }
                        file_copier = Some(copier);
                    }
                    Err(_) => {
                        eprintln!("Background file copying thread panicked");
                    }
                }
            }
        }
        let script_end = Instant::now();

        // Calculate Blake3 hashes for output files
        let output_files_hashes = self.calculate_output_files_hashes(
            &model_setup.output_dir,
            &model_setup.name,
            &model_setup.output_files,
        );

        // Create the run end dump
        let end_dump = RunEndFile {
            start,
            end: get_utc_now(),
            exit_code: status.code().unwrap_or_default(),
            runtime_ms: script_end.duration_since(script_start).as_millis(),
            files_copied: file_copier
                .as_ref()
                .map_or_else(HashSet::new, |fc| fc.copied_files.clone()),
            output_files_rewrites: model_setup.output_files,
            output_files_hashes,
        };
        end_dump.save(&model_setup.output_dir)?;

        if !status.success() {
            let mut error_msg = format!(
                "Script execution failed with exit code: {}\n",
                status.code().unwrap_or_default()
            );
            error_msg.push_str(std::str::from_utf8(&output)?);
            bail!("{}", error_msg);
        }

        // 8. Clean up unwanted files from output directory and update .gitignore
        if let Err(e) = cleanup_unwanted_files(
            &model_setup.output_dir,
            self.config.clean_level,
            &all_patterns,
            &model_setup.name,
        ) {
            eprintln!("Error during cleanup: {e}");
        }

        let mut f = fs::File::create(model_setup.output_dir.join(".gitignore"))?;
        f.write_all(get_final_gitignore(&model_setup.name).as_bytes())?;

        Ok(())
    }
}
