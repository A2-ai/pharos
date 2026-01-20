use extendr_api::Result;
use extendr_api::prelude::*;
use extendr_api::serializer::to_robj;

use fs_err as fs;
use std::path::Component;
use std::path::{Path, PathBuf};

// pharos config and nonmem crate
use config::{CONFIG_FILENAME, CommentType, Config, NonmemConfig, find_config_dir};
use nonmem::Model;

// hyperion core
use hyperion_core::{OptionExt, ResultExt, extendr_err};

/// Finds the correct output file path with the specified extension
///
/// This function handles various input formats and locates the expected
/// NONMEM output file location following the standard directory structure.
///
/// # Examples:
/// ```
/// # use hyperion_nonmem::utils::find_output_file;
/// // Directory input returns expected path
/// let result = find_output_file("models/run001", "ext");
/// // Should return "models/run001/run001.ext"
///
/// // .mod file input returns expected path
/// let result = find_output_file("models/run001.mod", "ext");
/// // Should return "models/run001/run001.ext"
/// ```
///
/// # Arguments:
/// * `input_path` - The input path (directory, .mod file, or output file)
/// * `extension` - The desired file extension (without dot, e.g. "ext", "grd")
///
/// # Returns:
/// * `Ok(PathBuf)` - The path to the output file
/// * `Err(Error)` - If the output file doesn't exist
pub fn find_output_file(input_path: impl AsRef<Path>, extension: &str) -> Result<PathBuf> {
    let path = input_path.as_ref();

    // If the input already has the target extension, check if it exists
    if let Some(current_ext) = path.extension()
        && current_ext == extension
    {
        if path.exists() {
            return Ok(path.to_path_buf());
        } else {
            return Err(extendr_err!("File not found: {}", path.display()));
        }
    }
    // Determine the base name for the output file
    let basename = if path.is_dir() {
        // Directory input: use directory name as basename
        path.file_name()
            .ok_or_extendr_err("Cannot determine directory name")?
            .to_string_lossy()
            .to_string()
    } else {
        // File input: use file stem as basename, handling special cases
        let stem = path
            .file_stem()
            .ok_or_extendr_err("Cannot determine file stem")?
            .to_string_lossy();

        // Handle metadata files: run001_metadata.json -> run001
        if stem.ends_with("_metadata") {
            stem.strip_suffix("_metadata").unwrap().to_string()
        } else {
            stem.to_string()
        }
    };

    // Construct the expected output file path
    let output_path = if path.is_dir() {
        // Directory/basename/basename.ext
        path.join(format!("{}.{}", basename, extension))
    } else {
        // parent/basename/basename.ext
        let parent = path
            .parent()
            .ok_or_extendr_err("Cannot determine parent directory")?;
        parent
            .join(&basename)
            .join(format!("{}.{}", basename, extension))
    };

    // Verify the output file exists
    if output_path.exists() {
        Ok(output_path)
    } else {
        Err(extendr_err!(
            "Output file not found: {}\nExpected location based on input: {}",
            output_path.display(),
            path.display()
        ))
    }
}

/// Resolve a model input path (.mod or .ctl), with a fallback for output-model paths.
pub fn resolve_input_model_path(input_path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = input_path.as_ref();

    if path.is_dir() {
        return Err(extendr_err!(
            "Expected .mod or .ctl file path, got directory: {}",
            path.display()
        ));
    }

    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some("mod") => "mod",
        Some("ctl") => "ctl",
        _ => {
            return Err(extendr_err!(
                "Expected .mod or .ctl file path: {}",
                path.display()
            ));
        }
    };

    if path.exists() {
        let stem = path
            .file_stem()
            .ok_or_extendr_err("Could not determine model file stem")?
            .to_string_lossy()
            .to_string();
        if let Some(parent) = path.parent()
            && parent
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == stem.as_str())
        {
            let candidate = parent.with_extension(ext);
            return Err(extendr_err!(
                "Expected input model file, got output model file: {}\n\
                 Try: {}",
                path.display(),
                candidate.display()
            ));
        }

        return Ok(path.to_path_buf());
    }

    Err(extendr_err!("File not found: {}", path.display()))
}

/// Builds a model source string relative to the pharos config directory when available.
pub fn get_model_source_path(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let config_dir = find_config_dir().map_to_extendr_err("Failed to find config dir")?;

    if let Some(dir) = config_dir {
        let rel = make_relative_path(&dir, path);
        return Ok(rel.to_string_lossy().to_string());
    }

    Ok(path.to_string_lossy().to_string())
}

/// Resolve a model source string into an absolute or config-relative path.
pub fn resolve_model_source_path(source: &str) -> Result<PathBuf> {
    let source_path = Path::new(source);
    if source_path.is_absolute() {
        return Ok(source_path.to_path_buf());
    }

    if let Some(dir) = find_config_dir().map_to_extendr_err("Failed to find config dir")? {
        return Ok(dir.join(source_path));
    }

    Ok(source_path.to_path_buf())
}

/// Resolve a model object's model_source attribute to an input model path.
pub fn resolve_model_input_path_from_robj(model: &Robj) -> Result<PathBuf> {
    let source = model
        .get_attrib("model_source")
        .ok_or_extendr_err("Model object is missing model_source attribute")?;
    let source_str = source
        .as_str()
        .ok_or_extendr_err("model_source attribute must be a string")?;
    let source_path = resolve_model_source_path(source_str)?;
    resolve_input_model_path(source_path)
}

fn make_relative_path(base: &Path, target: &Path) -> PathBuf {
    let base_components: Vec<Component<'_>> = base.components().collect();
    let target_components: Vec<Component<'_>> = target.components().collect();

    if base_components.first() != target_components.first() {
        return target.to_path_buf();
    }

    let mut idx = 0;
    let max = base_components.len().min(target_components.len());
    while idx < max && base_components[idx] == target_components[idx] {
        idx += 1;
    }

    let mut rel = PathBuf::new();
    for _ in idx..base_components.len() {
        rel.push("..");
    }
    for comp in target_components.iter().skip(idx) {
        rel.push(comp.as_os_str());
    }

    rel
}

/// Gives Some(Model) if model path is found
pub fn try_parse_model(path: &str) -> Option<Model> {
    let path_buf = std::path::Path::new(path);

    // If input is a file, use its parent directory for finding mod file
    let search_path = if path_buf.is_file() {
        path_buf.parent()?.to_str()?
    } else {
        path
    };

    let model_path = find_output_file(search_path, "mod")
        .or_else(|_| find_output_file(path, "ctl"))
        .ok()?;
    let content = fs::read_to_string(model_path).ok()?;
    Model::parse(&content).ok()
}

/// Gets the comment type from pharos.toml configuration
///
/// @return Option<CommentType> from pharos config, None if not found or config doesn't exist
pub fn get_comment_type() -> Option<CommentType> {
    find_config_dir()
        .ok()
        .flatten()
        .map(|dir| dir.join(CONFIG_FILENAME))
        .and_then(|path| Config::load(path).ok())
        .and_then(|config| config.nonmem.as_ref().and_then(|n| n.comments.r#type))
}

pub fn load_nonmem_config(run_nonmem_version: Option<&str>) -> Result<(PathBuf, NonmemConfig)> {
    let p = if let Some(root_dir) =
        find_config_dir().map_to_extendr_err("Failed to find config dir")?
    {
        root_dir.join(CONFIG_FILENAME)
    } else {
        std::env::current_dir()
            .map_to_extendr_err("Failed to get current directory")?
            .join(CONFIG_FILENAME)
    };

    if !p.exists() {
        return Err(extendr_err!(
            "pharos config file not found in current of parent directories",
        ));
    }

    let config = Config::load(&p).map_to_extendr_err("Failed to load config")?;

    let nonmem_config = config
        .nonmem
        .ok_or_extendr_err("pharos config file does not contain nonmem configuration")?;

    if let Some(version) = run_nonmem_version
        && !nonmem_config.versions.contains_key(version)
    {
        return Err(extendr_err!(
            "nonmem version {version} not found in config file"
        ));
    }

    Ok((p, nonmem_config))
}

/// Gets the pharos.toml configuration as an R object
///
/// @return pharos config as nested list structure
/// @export
///
/// @examples \dontrun{
/// config <- get_pharos_config()
/// config$nonmem$summary$high_correlation_threshold
/// config$nonmem$summary$high_condition_threshold
/// }
#[extendr]
pub fn get_pharos_config() -> Result<Robj> {
    let config_path = find_config_dir()
        .map_to_extendr_err("Failed to find config dir")?
        .ok_or_extendr_err("Could not find pharos config directory")?
        .join(CONFIG_FILENAME);

    let config = Config::load(config_path).map_to_extendr_err("Failed to load config")?;

    // Extract the values we need and build R-compatible structure manually
    let correlation_threshold = config
        .nonmem
        .as_ref()
        .map(|n| n.summary.high_correlation_threshold)
        .unwrap_or(0.95);

    let condition_threshold = config
        .nonmem
        .as_ref()
        .map(|n| n.summary.high_condition_threshold as f64)
        .unwrap_or(1000.0);

    // Build nested list structure: config$nonmem$summary$...
    let summary_list = list!(
        high_correlation_threshold = correlation_threshold,
        high_condition_threshold = condition_threshold
    );

    let nonmem_list = list!(summary = summary_list);

    let result = list!(nonmem = nonmem_list);

    Ok(result.into_robj())
}

/// Get the comment type from pharos.toml config file
///
///
/// @return CommentType R object
/// @export
///
/// @examples \dontrun{
/// get_comment_type()
/// }
#[extendr(r_name = "get_comment_type")]
pub fn get_comment_type_wrap() -> Result<Robj> {
    let comment_type = get_comment_type();
    let robj = to_robj(&comment_type).map_to_extendr_err("Failed to serialize to Robj")?;

    Ok(robj)
}
/// @keywords internal
/// @noRd
#[extendr(r_name = "resolve_input_model_path")]
pub fn resolve_input_model_path_wrap(path: &str) -> Result<Robj> {
    let path = resolve_input_model_path(path)?;
    Ok(path.to_string_lossy().into_robj())
}

extendr_module! {
    mod utils;
    fn get_pharos_config;
    fn get_comment_type_wrap;
    fn resolve_input_model_path_wrap;
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::glob;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_output_file_directory_input() {
        let temp_dir = TempDir::new().unwrap();
        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let ext_file = run_dir.join("run001.ext");
        fs::write(&ext_file, "test content").unwrap();

        let result = find_output_file(&run_dir, "ext").unwrap();
        assert_eq!(result, ext_file);
    }

    #[test]
    fn test_find_output_file_mod_input() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let ext_file = run_dir.join("run001.ext");
        fs::write(&ext_file, "test content").unwrap();

        let result = find_output_file(&mod_file, "ext").unwrap();
        assert_eq!(result, ext_file);
    }

    #[test]
    fn test_find_output_file_already_correct() {
        let temp_dir = TempDir::new().unwrap();
        let ext_file = temp_dir.path().join("run001.ext");
        fs::write(&ext_file, "test content").unwrap();

        let result = find_output_file(&ext_file, "ext").unwrap();
        assert_eq!(result, ext_file);
    }

    #[test]
    fn test_find_output_file_metadata_input() {
        let temp_dir = TempDir::new().unwrap();
        let metadata_file = temp_dir.path().join("run001_metadata.json");
        fs::write(&metadata_file, "{}").unwrap();

        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        let ext_file = run_dir.join("run001.ext");
        fs::write(&ext_file, "test content").unwrap();

        let result = find_output_file(&metadata_file, "ext").unwrap();
        assert_eq!(result, ext_file);
    }

    #[test]
    fn test_find_output_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let run_dir = temp_dir.path().join("run001");

        let result = find_output_file(&run_dir, "ext");
        assert!(result.is_err());
    }

    #[test]
    fn test_try_parse_model_success() {
        // Use real test data instead of creating temporary files
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data");
        glob!(test_dir, "**/*.mod", |path| {
            let result = try_parse_model(path.to_str().unwrap());
            assert!(
                result.is_some(),
                "Expected Some(Model) when valid mod file exists in test data"
            );
        })
    }

    #[test]
    fn test_try_parse_model_success_for_output_file() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data");
        glob!(test_dir, "**/*.grd", |path| {
            let result = try_parse_model(path.to_str().unwrap());
            assert!(
                result.is_some(),
                "Expected Some(Model) when valid mod file exists in test data"
            );
        })
    }

    #[test]
    fn test_try_parse_model_no_mod_file() {
        let temp_dir = TempDir::new().unwrap();
        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        // Don't create a mod file - should return None
        let result = try_parse_model(run_dir.to_str().unwrap());
        assert!(
            result.is_none(),
            "Expected None when mod file doesn't exist"
        );
    }

    #[test]
    fn test_resolve_input_model_path_ok() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let result = resolve_input_model_path(&mod_file).unwrap();
        assert_eq!(result, mod_file);
    }

    #[test]
    fn test_resolve_input_model_path_rejects_output_model() {
        let temp_dir = TempDir::new().unwrap();
        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();
        let output_mod = run_dir.join("run001.mod");
        fs::write(&output_mod, "test content").unwrap();

        let err = resolve_input_model_path(&output_mod).unwrap_err();
        let message = format!("{err}");
        assert!(message.contains("Expected input model file"));
        assert!(message.contains("Try:"));
    }

    #[test]
    fn test_resolve_input_model_path_rejects_wrong_extension() {
        let temp_dir = TempDir::new().unwrap();
        let txt_file = temp_dir.path().join("run001.txt");
        fs::write(&txt_file, "test content").unwrap();

        let err = resolve_input_model_path(&txt_file).unwrap_err();
        let message = format!("{err}");
        assert!(message.contains("Expected .mod or .ctl"));
    }

    #[test]
    fn test_resolve_model_source_path_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let mod_file = temp_dir.path().join("run001.mod");
        fs::write(&mod_file, "test content").unwrap();

        let result = resolve_model_source_path(mod_file.to_string_lossy().as_ref()).unwrap();
        assert_eq!(result, mod_file);
    }
}
