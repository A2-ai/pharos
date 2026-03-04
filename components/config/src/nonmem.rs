use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow, bail};
use fs_err as fs;
use glob::Pattern;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use which::which;

const KNOWN_NONMEM_FOLDERS: [&str; 2] = ["/opt/nonmem", "/opt/NONMEM"];

fn parse_nonmem_version_name(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("nm")?;
    let digit_count = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }

    rest[..digit_count].parse::<u32>().ok()
}

fn cmp_nonmem_version_names(a: &str, b: &str) -> Ordering {
    let a_num = parse_nonmem_version_name(a);
    let b_num = parse_nonmem_version_name(b);

    match (a_num, b_num) {
        (Some(a_num), Some(b_num)) => b_num.cmp(&a_num).then_with(|| a.cmp(b)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    }
}

fn resolve_path_from_config_dir(path: Option<&PathBuf>, config_dir: &Path) -> Option<PathBuf> {
    path.map(|p| {
        if p.is_relative() {
            config_dir.join(p)
        } else {
            p.clone()
        }
    })
}

fn find_mpiexec_path() -> PathBuf {
    which("mpiexec").unwrap_or_else(|_| PathBuf::from("/opt/bin/mpich/bin/mpiexec"))
}

fn find_nonmem_versions() -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();

    for folder in KNOWN_NONMEM_FOLDERS {
        let dir = PathBuf::from(&folder);
        if !dir.is_dir() {
            continue;
        }

        for folder in fs::read_dir(dir)?
            .filter_map(|f| f.ok())
            .filter(|f| f.path().is_dir())
        {
            let p = folder.path();
            let name = folder.file_name().to_string_lossy().into_owned();
            // And then check if it looks like actual nonmem files
            if !p.join("license").join("nonmem.lic").exists() || !p.join("run").is_dir() {
                continue;
            }

            log::debug!("Found nonmem {name} in {p:?}");
            out.insert(name, p);
        }
    }

    if out.is_empty() {
        bail!("Failed to find any nonmem versions");
    }

    Ok(out)
}

fn deserialize_validated_globs<'de, D>(deserializer: D) -> Result<Vec<Pattern>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| {
            Pattern::new(&s)
                .map_err(|e| D::Error::custom(format!("Invalid glob pattern '{s}': {e}")))
        })
        .collect()
}

fn serialize_glob_patterns<S>(patterns: &[Pattern], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let strings: Vec<String> = patterns.iter().map(|p| p.as_str().to_string()).collect();
    strings.serialize(serializer)
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum CommentType {
    /// For thetas:
    /// 1. `<param> (<unit?)`: `TVCL (L/h)`
    /// 2. `<param> cov`: `CRCL cov`
    /// 3. `<type> :<parameterization>`: `RES ERR :stdev`
    ///
    /// For omegas
    /// - `OM(1) <theta name> :<parameterization>`: `OM1 TVCL :EXP`, `OM1 TVKA :OMIT_TBL`
    ///
    /// For sigmas
    /// - `SIG(1) :<parameterization>`: `SIG1 :OMIT_TBL`
    #[serde(rename = "type1")]
    Type1,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct CommentsConfig {
    pub r#type: Option<CommentType>,
    pub error_on_invalid: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ParallelConfig {
    pub mpiexec_path: Option<PathBuf>,
    pub enabled: bool,
    pub num_cpus: u8,
    pub timeout: usize,
    parafile: Option<PathBuf>,
}

impl ParallelConfig {
    pub fn parafile(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.parafile.as_ref(), config_dir)
    }

    pub fn set_parafile(&mut self, parafile: Option<PathBuf>) {
        self.parafile = parafile;
    }

    pub fn generate_parafile(&self) -> String {
        // We will have validated that we have at least 2 nodes and that mpiexec_path is present
        // when this is called
        let total_nodes = self.num_cpus as usize;
        let timeout = self.timeout;
        let mpiexec_path = self.mpiexec_path.as_ref().unwrap();
        let worker_nodes = total_nodes - 1;
        // Parse type 2 refers to evenly load balanced work
        // Transfer Type 1 refers to MPI
        // TIMEOUTI 100 means wait 100 seconds for node to become available
        format!(
            r#"$GENERAL
NODES={total_nodes} PARSE_TYPE=2 TIMEOUTI=100 TIMEOUT={timeout} PARAPRINT=0 TRANSFER_TYPE=1
$COMMANDS
1: {mpiexec_path:?} -wdir "$PWD" -n 1 ./nonmem $*
2:-wdir "$PWD" -n {worker_nodes} ./nonmem -wnf
$DIRECTORIES
1:NONE
2-[nodes]:worker{{#-1}}
"#
        )
    }

    /// Validates that the config is correct and errors otherwise.
    /// This is called before starting a run
    pub fn validate(&self, config_dir: &Path) -> Result<()> {
        if !self.enabled {
            log::debug!("Parallel execution disabled");
            return Ok(());
        }

        // Check that MPI executable exists and is executable
        if let Some(mpiexec_path) = &self.mpiexec_path {
            if !mpiexec_path.exists() {
                bail!("MPI executable not found: {mpiexec_path:?}");
            }
        } else {
            bail!("MPI executable not set in config file");
        }

        // Check that threads is at least 2
        if self.num_cpus < 2 {
            bail!(
                "Parallel execution requires at least 2 threads, got {}",
                self.num_cpus
            );
        }

        if let Some(parafile_path) = self.parafile(config_dir)
            && !parafile_path.exists()
        {
            bail!("Parafile {parafile_path:?} does not exist.",);
        }

        log::debug!("Parallel config is ok!");

        Ok(())
    }
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            mpiexec_path: Default::default(),
            enabled: false,
            num_cpus: 4,
            timeout: 2147483647,
            parafile: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonmemOptions {
    license_file: Option<PathBuf>,
    prsame: bool,
    prcompile: bool,
    prdefault: bool,
    tprdefault: bool,
    background: bool,
    nobuild: bool,
    maxlim: u8,
}

impl NonmemOptions {
    pub fn as_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        if let Some(ref license_file) = self.license_file {
            flags.push(format!("-licfile={}", license_file.display()));
        }
        if self.prsame {
            flags.push("-prsame".to_string());
        }
        if self.prcompile {
            flags.push("-prcompile".to_string());
        }
        if self.prdefault {
            flags.push("-prdefault".to_string());
        }
        if self.tprdefault {
            flags.push("-tprdefault".to_string());
        }
        if self.background {
            flags.push("-background".to_string());
        }
        if self.nobuild {
            flags.push("-nobuild".to_string());
        }

        if self.maxlim != 0 {
            flags.push(format!("-maxlim={}", self.maxlim));
        }

        flags
    }
}

impl Default for NonmemOptions {
    fn default() -> Self {
        Self {
            license_file: None,
            prsame: false,
            prcompile: false,
            prdefault: false,
            tprdefault: false,
            background: false,
            nobuild: false,
            maxlim: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub high_correlation_threshold: f64,
    pub high_condition_threshold: usize,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            high_correlation_threshold: 0.95,
            high_condition_threshold: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Slurm {
    template: Option<PathBuf>,
    pub partition: Option<String>,
    log_folder: Option<PathBuf>,
}

impl Slurm {
    pub fn template(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.template.as_ref(), config_dir)
    }

    pub fn log_folder(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.log_folder.as_ref(), config_dir)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Sge {
    template: Option<PathBuf>,
    log_folder: Option<PathBuf>,
}

impl Sge {
    pub fn template(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.template.as_ref(), config_dir)
    }

    pub fn log_folder(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.log_folder.as_ref(), config_dir)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonmemConfig {
    pub clean_level: u8,
    pub output_dir: Option<String>,
    post_run_script: Option<PathBuf>,
    pub options: NonmemOptions,
    pub versions: HashMap<String, PathBuf>,
    default_version: String,
    #[serde(
        default,
        deserialize_with = "deserialize_validated_globs",
        serialize_with = "serialize_glob_patterns"
    )]
    files_to_copy: Vec<Pattern>,
    #[serde(default)]
    pub parallel: ParallelConfig,
    #[serde(default)]
    pub comments: CommentsConfig,
    #[serde(default)]
    pub summary: Summary,
    #[serde(default)]
    pub slurm: Slurm,
    #[serde(default)]
    pub sge: Sge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCheckResult {
    pub path: PathBuf,
    pub found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultVersionInfo {
    pub name: String,
    pub defined: bool,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonmemInstallation {
    pub name: String,
    pub installation_path: PathBuf,
    pub nmfe: Option<PathBuf>,
    pub nmtran: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpiInfo {
    pub mpi: PathCheckResult,
    pub version_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitrepResult {
    pub default_version: DefaultVersionInfo,
    pub nonmem_installations: Vec<NonmemInstallation>,
    pub mpi_info: Option<MpiInfo>,
    pub slurm_template: Option<PathCheckResult>,
    pub sge_template: Option<PathCheckResult>,
}

impl SitrepResult {
    pub fn has_errors(&self) -> bool {
        if !self.default_version.defined || !self.default_version.valid {
            return true;
        }

        if self
            .nonmem_installations
            .iter()
            .any(|inst| inst.nmfe.is_none() || inst.nmtran.is_none())
        {
            return true;
        }

        if let Some(mpi) = &self.mpi_info
            && (!mpi.mpi.found || mpi.version_output.is_none())
        {
            return true;
        }

        if let Some(slurm_template) = &self.slurm_template
            && !slurm_template.found
        {
            return true;
        }

        if let Some(sge_template) = &self.sge_template
            && !sge_template.found
        {
            return true;
        }

        false
    }
}

impl Default for NonmemConfig {
    fn default() -> Self {
        Self {
            clean_level: 1,
            output_dir: None,
            post_run_script: None,
            options: Default::default(),
            versions: Default::default(),
            default_version: "".to_string(),
            files_to_copy: Vec::new(),
            parallel: Default::default(),
            comments: Default::default(),
            summary: Default::default(),
            slurm: Default::default(),
            sge: Default::default(),
        }
    }
}

impl NonmemConfig {
    pub fn new() -> Result<Self> {
        let mut config = NonmemConfig::default();

        let mpiexec_path = find_mpiexec_path();
        if mpiexec_path.exists() {
            config.parallel.mpiexec_path = Some(mpiexec_path);
        }
        config.versions = find_nonmem_versions()?;
        let mut versions = config.versions.keys().collect::<Vec<_>>();
        versions.sort_by(|a, b| cmp_nonmem_version_names(a, b));
        config.default_version = versions[0].clone();

        Ok(config)
    }

    pub fn post_run_script(&self, config_dir: &Path) -> Option<PathBuf> {
        resolve_path_from_config_dir(self.post_run_script.as_ref(), config_dir)
    }

    pub fn set_post_run_script(&mut self, p: Option<PathBuf>) {
        self.post_run_script = p;
    }

    pub fn files_to_copy(&self) -> &[Pattern] {
        &self.files_to_copy
    }

    pub fn set_default_version(&mut self, version: String) {
        self.default_version = version;
    }

    pub fn get_nonmem_executable_path(&self, version: Option<&str>) -> Result<PathBuf> {
        let version = version.unwrap_or(&self.default_version);
        let path = self
            .versions
            .get(version)
            .ok_or_else(|| anyhow!("version {version} not found"))?;

        let run_dir = path.join("run");
        fs::read_dir(&run_dir)?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("nmfe"))
            })
            .map(|entry| entry.path())
            .ok_or_else(|| anyhow!("nmfe executable not found in {}", run_dir.display()))
    }

    pub fn get_nmtrans_executable_path(&self, version: Option<&str>) -> Result<PathBuf> {
        let version = version.unwrap_or(&self.default_version);
        let path = self
            .versions
            .get(version)
            .ok_or_else(|| anyhow!("version {version} not found"))?;

        let nmtran_exe = path.join("tr").join("NMTRAN.exe");
        if !nmtran_exe.exists() {
            bail!("NMTRAN.exe not found at: {:#?}", nmtran_exe)
        }

        Ok(nmtran_exe)
    }

    pub fn validate(&self) -> SitrepResult {
        let mut default_version = DefaultVersionInfo {
            name: self.default_version.clone(),
            defined: self.versions.contains_key(self.default_version.as_str()),
            valid: false,
        };

        let mut nonmem_installations = Vec::new();
        for (name, path) in &self.versions {
            let nmfe_path = self.get_nonmem_executable_path(Some(name)).ok();
            let nmtran_path = self.get_nmtrans_executable_path(Some(name)).ok();
            if name == default_version.name.as_str() && nmfe_path.is_some() && nmtran_path.is_some()
            {
                default_version.valid = true;
            }
            nonmem_installations.push(NonmemInstallation {
                name: name.clone(),
                installation_path: path.clone(),
                nmfe: nmfe_path,
                nmtran: nmtran_path,
            });
        }

        let slurm_template = self.slurm.template.as_ref().map(|t| PathCheckResult {
            path: t.clone(),
            found: t.exists(),
        });
        let sge_template = self.sge.template.as_ref().map(|t| PathCheckResult {
            path: t.clone(),
            found: t.exists(),
        });

        let mpi_info = if let Some(mpi_path) = &self.parallel.mpiexec_path {
            let mpi = PathCheckResult {
                path: mpi_path.clone(),
                found: mpi_path.exists(),
            };
            let version_output = match Command::new(mpi_path).arg("--version").output() {
                Ok(output) => match std::str::from_utf8(&output.stdout) {
                    Ok(version_str) => Some(version_str.to_string()),
                    Err(e) => {
                        log::debug!("Failed to parse mpiexec version output: {e:?}");
                        None
                    }
                },
                Err(e) => {
                    log::debug!("Failed to run mpiexec --version: {e:?}");
                    None
                }
            };
            Some(MpiInfo {
                mpi,
                version_output,
            })
        } else {
            None
        };

        SitrepResult {
            default_version,
            nonmem_installations,
            slurm_template,
            sge_template,
            mpi_info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_valid_glob_patterns_deserialize() {
        let toml_content = r#"
        [nonmem]
        clean_level = 1
        default_version = "nm760"
        files_to_copy = ["*.mod", "data/**/*.csv", "output.lst"]

        [nonmem.options]
        prsame = false
        prcompile = false
        prdefault = false
        tprdefault = false
        background = false
        nobuild = false
        maxlim = 2

        [nonmem.versions]
        nm760 = "/opt/nonmem/nm760"
        "#;

        let config: Config = toml::from_str(toml_content).expect("Should deserialize valid globs");
        let nonmem = config.nonmem.expect("Should have nonmem config");
        let patterns = nonmem.files_to_copy();

        assert_eq!(patterns.len(), 3);
        assert!(patterns[0].matches("test.mod"));
        assert!(patterns[1].matches("data/subdir/test.csv"));
        assert!(patterns[2].matches("output.lst"));
    }

    #[test]
    fn test_invalid_glob_pattern_fails_deserialize() {
        let toml_content = r#"
        [nonmem]
        clean_level = 1
        default_version = "nm760"
        files_to_copy = ["*.mod", "[invalid"]

        [nonmem.options]
        prsame = false
        prcompile = false
        prdefault = false
        tprdefault = false
        background = false
        nobuild = false
        maxlim = 2

        [nonmem.versions]
        nm760 = "/opt/nonmem/nm760"
        "#;

        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid glob pattern"));
        assert!(error_msg.contains("[invalid"));
    }

    #[test]
    fn test_empty_files_to_copy_works() {
        let toml_content = r#"
        [nonmem]
        clean_level = 1
        default_version = "nm760"

        [nonmem.options]
        prsame = false
        prcompile = false
        prdefault = false
        tprdefault = false
        background = false
        nobuild = false
        maxlim = 2

        [nonmem.versions]
        nm760 = "/opt/nonmem/nm760"
        "#;

        let config: Config =
            toml::from_str(toml_content).expect("Should deserialize without files_to_copy");
        let nonmem = config.nonmem.expect("Should have nonmem config");
        assert_eq!(nonmem.files_to_copy().len(), 0);
    }

    #[test]
    fn test_nonmem_versions_sorted_latest_first_with_lexical_fallback() {
        let mut versions = [
            "nm74gf_nmfe",
            "nm75",
            "nm74",
            "nm73",
            "nm73gf",
            "nm74gf",
            "nm73gf_nmfe",
            "nm76",
        ];
        versions.sort_by(|a, b| cmp_nonmem_version_names(a, b));

        assert_eq!(
            versions,
            [
                "nm76",
                "nm75",
                "nm74",
                "nm74gf",
                "nm74gf_nmfe",
                "nm73",
                "nm73gf",
                "nm73gf_nmfe"
            ]
        );
    }
}
