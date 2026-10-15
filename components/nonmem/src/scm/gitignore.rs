//! The out_dir's `.gitignore`: one pharos-owned file at the root of the SCM
//! process's directory, chosen by `[nonmem.scm] track_in_git`.
//!
//! The file names directories, not state, so it is right for the whole
//! process from the moment it is written: git patterns may name directories
//! that do not exist yet. It is written by `scm init`, and rewritten by every
//! submit and by the driver, so a changed `pharos.toml` takes effect at the
//! next command. The fits' own run directories keep the `.gitignore` every
//! pharos run writes (NONMEM scratch files, `.msf`), whatever the level.

use std::path::Path;

use ::config::GitTracking;
use anyhow::{Context, Result};
use fs_err as fs;

use super::config::CONFIG_SUFFIX;
use super::shown;
use super::{FORWARD_FINAL_DIR, SCM_SUMMARY_FILENAME, SCM_SUMMARY_MD};

/// The login-node driver's record (`scm submit`), in the out_dir
pub const DRIVER_LOG_FILENAME: &str = "scm_driver.log";

/// The `.gitignore` for an SCM process on a model with `stem`, at `level`.
pub fn render_gitignore(stem: &str, level: GitTracking) -> String {
    let mut out = format!(
        "# Written by pharos (nonmem.scm track_in_git = \"{}\"). Do not edit; change pharos.toml instead.\n",
        level.as_str()
    );
    if level == GitTracking::All {
        return out;
    }

    // `/*` ignores only the out_dir's own entries, so an un-ignored directory
    // is tracked whole; a bare `*` would match at every depth and keep its
    // contents ignored.
    out.push_str("/*\n!/.gitignore\n");
    out.push_str(&format!("!/{stem}{CONFIG_SUFFIX}\n"));
    out.push_str(&format!("!/{SCM_SUMMARY_FILENAME}\n"));
    out.push_str(&format!("!/{SCM_SUMMARY_MD}\n"));
    out.push_str(&format!("!/{DRIVER_LOG_FILENAME}\n"));
    if level == GitTracking::Milestones {
        out.push_str("!/base/\n!/full/\n");
        out.push_str(&format!("!/{FORWARD_FINAL_DIR}/\n"));
    }
    out.push_str("!/final/\n");
    out
}

/// Write (or rewrite) the out_dir's `.gitignore` for `level`.
pub fn write_gitignore(out_dir: &Path, stem: &str, level: GitTracking) -> Result<()> {
    let path = out_dir.join(".gitignore");
    fs::write(&path, render_gitignore(stem, level))
        .with_context(|| format!("failed to write {}", shown(&path)))
}

/// The text at each level is pinned by the `gitignore_for_each_tracking_level`
/// snapshots.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_replaces_whatever_was_there() {
        let dir = tempfile::tempdir().unwrap();
        write_gitignore(dir.path(), "1001", GitTracking::Final).unwrap();
        write_gitignore(dir.path(), "1001", GitTracking::All).unwrap();
        let text = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(text, render_gitignore("1001", GitTracking::All));
    }
}
