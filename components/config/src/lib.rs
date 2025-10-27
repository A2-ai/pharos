mod templating;

pub use templating::render_output_template;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use fs_err as fs;
use glob::Pattern;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use which::which;

pub const CONFIG_FILENAME: &str = "pharos.toml";

fn find_mpiexec_path() -> PathBuf {
    which("mpiexec").unwrap_or_else(|_| PathBuf::from("/opt/bin/mpich/bin/mpiexec"))
}

/// Find where the root dir is (eg where the config file).
/// If we can't find it and we reached a .git folder/no more parent folder, this returns None.
pub fn find_config_dir() -> Result<Option<PathBuf>> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join(CONFIG_FILENAME).exists() {
            return Ok(Some(current));
        }

        if current.join(".git").is_dir() {
            break;
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    Ok(None)
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
    /// For omegas
    /// `OM(1) <theta name> :<parameterization>`: `OM1 TVCL :EXP`, `OM1 TVKA :OMIT_TBL`
    /// For sigmas
    /// `SIG(1) :<parameterization>`: `SIG1 :OMIT_TBL`
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
    pub mpiexec_path: PathBuf,
    pub enabled: bool,
    pub num_cpus: u8,
    pub timeout: usize,
    pub parafile: Option<PathBuf>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonmemConfig {
    pub clean_level: u8,
    pub output_dir: Option<String>,
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
}

impl Default for NonmemConfig {
    fn default() -> Self {
        Self {
            clean_level: 1,
            output_dir: None,
            options: Default::default(),
            versions: Default::default(),
            default_version: "".to_string(),
            files_to_copy: Vec::new(),
            parallel: Default::default(),
            comments: Default::default(),
            summary: Default::default(),
        }
    }
}

impl NonmemConfig {
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

        Ok(path.join("tr").join("NMTRAN.exe"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub nonmem: Option<NonmemConfig>,
}

impl Config {
    pub fn new_nonmem() -> Self {
        let mut config = NonmemConfig {
            default_version: "nm760".to_string(),
            ..Default::default()
        };

        config.parallel.mpiexec_path = find_mpiexec_path();
        config
            .versions
            .insert("nm760".to_string(), PathBuf::from("/opt/nonmem/nm760"));
        Self {
            nonmem: Some(config),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_serialize_deserialize_roundtrip() {
        let mut config = Config::new_nonmem();
        if let Some(ref mut nonmem) = config.nonmem {
            nonmem.files_to_copy = vec![
                Pattern::new("*.mod").unwrap(),
                Pattern::new("data/**/*.csv").unwrap(),
            ];
        }

        let serialized = toml::to_string(&config).expect("Should serialize config");
        let deserialized: Config = toml::from_str(&serialized).expect("Should deserialize config");

        let nonmem = deserialized.nonmem.expect("Should have nonmem config");
        let patterns = nonmem.files_to_copy();

        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].matches("test.mod"));
        assert!(patterns[1].matches("data/subdir/test.csv"));
    }
}
