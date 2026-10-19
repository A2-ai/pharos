use std::path::{Component, Path, PathBuf};

/// Lexically resolve `.` and `..` components without consulting the
/// filesystem (so symlinks are never followed and nothing has to exist).
/// `a/b/../c` becomes `a/c`; a leading `..` with nothing to pop is kept, and
/// at an absolute root `..` has nowhere to go and is dropped, matching how
/// the OS resolves it.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_dot_and_dotdot() {
        assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize_path(Path::new("./a/./b")), PathBuf::from("a/b"));
        assert_eq!(
            normalize_path(Path::new("../../x")),
            PathBuf::from("../../x")
        );
        assert_eq!(normalize_path(Path::new("/a/../..")), PathBuf::from("/"));
        assert_eq!(normalize_path(Path::new("a/..")), PathBuf::from("."));
        assert_eq!(
            normalize_path(Path::new("scm/1001/../../1001.mod")),
            PathBuf::from("1001.mod")
        );
    }
}
