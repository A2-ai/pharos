use config::{CommentType, Config, find_config_dir};
use extendr_api::prelude::*;
use fs_err as fs;
use nonmem::Model;
use std::path::{Path, PathBuf};

/// Finds the correct output file path with the specified extension
///
/// This function handles various input formats and locates the expected
/// NONMEM output file location following the standard directory structure.
///
/// # Examples:
/// ```
/// // Directory input
/// find_output_file("models/run001", "ext") -> "models/run001/run001.ext"
///
/// // .mod file input
/// find_output_file("models/run001.mod", "ext") -> "models/run001/run001.ext"
///
/// // Already correct path
/// find_output_file("models/run001/run001.ext", "ext") -> "models/run001/run001.ext"
///
/// // Different extension
/// find_output_file("models/run001", "grd") -> "models/run001/run001.grd"
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
            return Err(Error::Other(format!("File not found: {}", path.display())));
        }
    }
    // Determine the base name for the output file
    let basename = if path.is_dir() {
        // Directory input: use directory name as basename
        path.file_name()
            .ok_or_else(|| Error::Other("Cannot determine directory name".to_string()))?
            .to_string_lossy()
            .to_string()
    } else {
        // File input: use file stem as basename, handling special cases
        let stem = path
            .file_stem()
            .ok_or_else(|| Error::Other("Cannot determine file stem".to_string()))?
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
            .ok_or_else(|| Error::Other("Cannot determine parent directory".to_string()))?;
        parent
            .join(&basename)
            .join(format!("{}.{}", basename, extension))
    };

    // Verify the output file exists
    if output_path.exists() {
        Ok(output_path)
    } else {
        Err(Error::Other(format!(
            "Output file not found: {}\nExpected location based on input: {}",
            output_path.display(),
            path.display()
        )))
    }
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

    let model_path = find_output_file(search_path, "mod").ok()?;
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
        .map(|dir| dir.join("pharos.toml"))
        .and_then(|path| Config::load(path).ok())
        .and_then(|config| config.nonmem.as_ref().and_then(|n| n.comments.r#type))
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
        .map_err(|e| Error::Other(format!("Failed to find config dir: {e}")))?
        .ok_or_else(|| Error::Other("Could not find pharos config directory".to_string()))?
        .join("pharos.toml");

    let config = Config::load(config_path)
        .map_err(|e| Error::Other(format!("Failed to load config: {e}")))?;

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

extendr_module! {
    mod utils;
    fn get_pharos_config;
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
}
