use std::collections::{HashMap, HashSet};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use fs_err as fs;
use glob::Pattern;
use walkdir::WalkDir;

use crate::TERMINATION_FILENAME;
use crate::run_metadata::{RUN_CONFIG_FILENAME, RUN_END_FILENAME, RUN_START_FILENAME};

const FILES_TO_KEEP: &[&str] = &[
    RUN_START_FILENAME,
    RUN_END_FILENAME,
    RUN_CONFIG_FILENAME,
    TERMINATION_FILENAME,
    ".gitignore",
    // https://github.com/A2-ai/pharos/issues/39
    "PRDERR",
    "OUTPUT",
];
const EXTENSIONS_LEVEL_0: &[&str] = &[".mod", ".sh"];
const EXTENSIONS_LEVEL_1: &[&str] = &[".xml", ".grd", ".shk", ".cor", ".cov", ".ext", ".lst"];
const EXTENSIONS_LEVEL_2: &[&str] = &[".clt", ".coi", ".cpu", ".shm", ".phi"];
const EXTENSIONS_LEVEL_3: &[&str] = &["", ".msf"];

#[inline]
fn get_extensions_for_level(level: u8) -> Vec<&'static str> {
    match level {
        1 => [EXTENSIONS_LEVEL_0, EXTENSIONS_LEVEL_1].concat(),
        2 => [EXTENSIONS_LEVEL_0, EXTENSIONS_LEVEL_1, EXTENSIONS_LEVEL_2].concat(),
        3 => [
            EXTENSIONS_LEVEL_0,
            EXTENSIONS_LEVEL_1,
            EXTENSIONS_LEVEL_2,
            EXTENSIONS_LEVEL_3,
        ]
        .concat(),
        _ => vec![],
    }
}

#[derive(Debug)]
pub struct FileCopier {
    model_name: String,
    last_scan_time: SystemTime,
    pub copied_files: HashSet<String>,
    file_sizes: HashMap<PathBuf, u64>,
    level: u8,
    patterns: Vec<Pattern>,
}

impl FileCopier {
    pub fn new(model_name: String, level: u8, patterns: Vec<Pattern>) -> Self {
        Self {
            model_name,
            level,
            patterns,
            last_scan_time: SystemTime::UNIX_EPOCH,
            copied_files: HashSet::new(),
            file_sizes: HashMap::new(),
        }
    }

    pub fn copy_changed_files(&mut self, source_dir: &Path, dest_dir: &Path) -> Result<()> {
        let scan_start = SystemTime::now();
        let extensions = get_extensions_for_level(self.level);
        log::debug!(
            "Copying changed with clean level {}: {extensions:?}",
            self.level
        );

        let mut copied_files = Vec::new();

        for entry in WalkDir::new(source_dir) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            // Skip the source directory itself
            if path == source_dir {
                continue;
            }

            if path.is_dir() {
                continue;
            }

            let dest_path = match path.strip_prefix(source_dir) {
                Ok(relative_path) => dest_dir.join(relative_path),
                Err(_) => continue,
            };

            if !should_copy_file(path, &extensions, &self.patterns, &self.model_name) {
                continue;
            }

            let modified_time = match entry.metadata() {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => time,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            if modified_time <= self.last_scan_time {
                continue;
            }

            if let Some(parent) = dest_path.parent()
                && fs::create_dir_all(parent).is_err()
            {
                continue;
            }

            let current_size = match entry.metadata() {
                Ok(metadata) => metadata.len(),
                Err(_) => continue,
            };

            let last_known_size = self.file_sizes.get(path).copied().unwrap_or(0);
            let copy_result = if current_size > last_known_size && dest_path.exists() {
                // File grew and destination exists - try incremental copy
                incremental_copy(path, &dest_path, last_known_size)
            } else {
                // New file, shrunk file, or no destination - full copy
                fs::copy(path, &dest_path).map(|_| ()).map_err(Into::into)
            };

            match copy_result {
                Ok(_) => {
                    self.file_sizes.insert(path.to_path_buf(), current_size);
                    copied_files.push(dest_path);
                }
                Err(_) => continue,
            }
        }

        self.last_scan_time = scan_start;
        Ok(())
    }
}

/// We want to copy any files that fit one of the 3 criterias: right extension, matching pattern or
/// an expected output file
pub fn should_copy_file(
    path: impl AsRef<Path>,
    extensions: &[&str],
    patterns: &[Pattern],
    model_name: &str,
) -> bool {
    // If we don't have any extension, assume the user doesn't want to clean anything because
    // they set clean_level to something other than 1, 2 or 3
    if extensions.is_empty() {
        return true;
    }
    let path = path.as_ref();

    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    if FILES_TO_KEEP.contains(&file_name) {
        return true;
    }

    // Check extensions (only if filename matches model_name.extension pattern)
    let matches_extension = extensions.iter().any(|&ext| match ext {
        // Empty string: match files with no extension (no dots in filename)
        "" => !file_name.contains('.') && file_name == model_name,

        // Extension starting with dot: check model_name.extension pattern
        ext if ext.starts_with('.') => {
            // Handle special cases like .gitignore
            if file_name == ext {
                return true;
            }
            // Check if filename matches model_name + extension
            file_name == format!("{model_name}{ext}")
        }

        // Plain suffix: match model_name + suffix
        ext => file_name == format!("{model_name}{ext}"),
    });

    if matches_extension {
        return true;
    }

    // Check glob patterns
    patterns.iter().any(|pattern| pattern.matches(file_name))
}

fn incremental_copy(source: &Path, dest: &Path, start_offset: u64) -> Result<()> {
    // Verify destination size matches expected offset
    let dest_size = fs::metadata(dest)?.len();
    if dest_size != start_offset {
        // Destination was modified - fall back to full copy
        log::debug!("{dest:?} was modified, copying the whole file");
        fs::copy(source, dest)?;
        return Ok(());
    }

    let mut source_file = fs::File::open(source)?;
    source_file.seek(SeekFrom::Start(start_offset))?;

    let mut dest_file = fs::OpenOptions::new().append(true).open(dest)?;
    log::debug!("Appending to {dest:?}.");

    std::io::copy(&mut source_file, &mut dest_file)?;
    Ok(())
}

pub fn cleanup_unwanted_files(
    dir: &Path,
    level: u8,
    patterns: &[Pattern],
    model_name: &str,
) -> Result<()> {
    let extensions = get_extensions_for_level(level);

    // Collect files and directories to remove (process files first, then directories)
    let mut files_to_remove = Vec::new();
    let mut dirs_to_remove = Vec::new();

    for entry in WalkDir::new(dir).contents_first(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip the target directory itself
        if path == dir {
            continue;
        }

        if path.is_file() {
            // If we are here this means the run succeeded.
            // In that case we always want to remove the OUTPUT file as it's not needed anymore
            // even though we wanted it while streaming.
            if path.file_name() == Some("OUTPUT".as_ref()) {
                files_to_remove.push(path.to_path_buf());
            } else if path.file_name() == Some("PRDERR".as_ref()) {
                if fs::metadata(path)?.len() == 0 {
                    files_to_remove.push(path.to_path_buf());
                }
            } else if !should_copy_file(path, &extensions, patterns, model_name) {
                files_to_remove.push(path.to_path_buf());
            }
        } else {
            // Mark directory for removal - will be removed if empty after files are cleaned
            dirs_to_remove.push(path.to_path_buf());
        }
    }

    // Remove unwanted files first
    for file_path in files_to_remove {
        log::debug!("Removing file {file_path:?}.");
        fs::remove_file(&file_path)?;
    }

    // Remove empty directories (in reverse order due to contents_first(true))
    for dir_path in dirs_to_remove {
        log::debug!("Removing directory {dir_path:?}.");
        fs::remove_dir_all(&dir_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_copy_file() {
        let no_patterns = vec![];
        let model_name = "run001";

        // level 1 - only copies files matching model_name.extension
        let extensions = get_extensions_for_level(1);
        assert!(should_copy_file(
            "run001.mod",
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(should_copy_file(
            "run001.sh",
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(should_copy_file(
            ".gitignore",
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(should_copy_file(
            RUN_START_FILENAME,
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(should_copy_file(
            RUN_CONFIG_FILENAME,
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(should_copy_file(
            "run001.grd",
            &extensions,
            &no_patterns,
            model_name
        ));

        // Should NOT copy files that don't match model name
        assert!(!should_copy_file(
            "other.mod",
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(!should_copy_file(
            "data.grd",
            &extensions,
            &no_patterns,
            model_name
        ));

        // level 3
        let extensions = get_extensions_for_level(3);
        assert!(should_copy_file(
            "run001.msf",
            &extensions,
            &no_patterns,
            model_name
        ));
        assert!(!should_copy_file(
            "other.msf",
            &extensions,
            &no_patterns,
            model_name
        ));
    }

    #[test]
    fn test_should_copy_file_with_patterns() {
        let no_extensions = vec![];
        let patterns = vec![
            Pattern::new("*.dat").unwrap(),
            Pattern::new("output_*.txt").unwrap(),
            Pattern::new("config.json").unwrap(),
        ];

        // Should match glob patterns
        assert!(should_copy_file(
            "data.dat",
            &no_extensions,
            &patterns,
            "model"
        ));
        assert!(should_copy_file(
            "output_summary.txt",
            &no_extensions,
            &patterns,
            "model"
        ));
        assert!(should_copy_file(
            "config.json",
            &no_extensions,
            &patterns,
            "model"
        ));

        // No extensions: just copy everything
        assert!(should_copy_file(
            "data.csv",
            &no_extensions,
            &patterns,
            "model"
        ));
    }

    #[test]
    fn test_should_copy_file_extensions_or_patterns() {
        let extensions = vec![".mod"];
        let patterns = vec![Pattern::new("*.dat").unwrap()];

        assert!(should_copy_file("test.mod", &extensions, &patterns, "test"));
        assert!(should_copy_file("test.dat", &extensions, &patterns, "test"));
        assert!(!should_copy_file(
            "test.txt",
            &extensions,
            &patterns,
            "test"
        ));
    }
}
