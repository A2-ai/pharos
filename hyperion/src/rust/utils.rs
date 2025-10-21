use extendr_api::prelude::*;
use std::path::{Path, PathBuf};
use nonmem::Model;
use fs_err as fs;

/// Finds the correct output file path with the specified extension
///
/// This function handles various input formats and locates the expected
/// NONMEM output file location following the standard directory structure.
///
/// # Examples:
/// ```
/// // Directory input
/// find_output_file("models/run001", "ext") → "models/run001/run001.ext"
///
/// // .mod file input
/// find_output_file("models/run001.mod", "ext") → "models/run001/run001.ext"
///
/// // Already correct path
/// find_output_file("models/run001/run001.ext", "ext") → "models/run001/run001.ext"
///
/// // Different extension
/// find_output_file("models/run001", "grd") → "models/run001/run001.grd"
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
    let model_path = find_output_file(&path, "mod").ok()?;
    let content = fs::read_to_string(model_path).ok()?;
    Model::parse(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use insta::glob;
    
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
            assert!(result.is_some(), "Expected Some(Model) when valid mod file exists in test data");
        })
    }

    #[test]
    fn test_try_parse_model_no_mod_file() {
        let temp_dir = TempDir::new().unwrap();
        let run_dir = temp_dir.path().join("run001");
        fs::create_dir(&run_dir).unwrap();

        // Don't create a mod file - should return None
        let result = try_parse_model(run_dir.to_str().unwrap());
        assert!(result.is_none(), "Expected None when mod file doesn't exist");
    }
}
