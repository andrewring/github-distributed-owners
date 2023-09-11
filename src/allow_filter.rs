use anyhow::anyhow;
use itertools::Itertools;
use log::trace;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait AllowFilter {
    fn allowed(&self, path: &Path) -> bool;
}

#[derive(Debug)]
pub struct FilterGitMetadata {}

impl AllowFilter for FilterGitMetadata {
    fn allowed(&self, path: &Path) -> bool {
        for component in path {
            if component == ".git" {
                return false;
            }
        }
        true
    }
}

pub struct AllowList {
    allowed_files: HashSet<PathBuf>,
    _private: (), // Force use of AllowList::from outside this package
}

impl AllowFilter for AllowList {
    fn allowed(&self, path: &Path) -> bool {
        self.allowed_files.contains(path)
    }
}

impl AllowList {
    pub fn allow_git_files() -> anyhow::Result<AllowList> {
        let output = Command::new("git").arg("ls-files").output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "Error gathering git files:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let git_files: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect();
        trace!(
            "Git files:{}",
            git_files
                .iter()
                .sorted()
                .map(|p| format!("\n - {:?}", &p))
                .join("")
        );
        AllowList::from(git_files, true)
    }

    pub fn from(paths: HashSet<PathBuf>, expand: bool) -> anyhow::Result<AllowList> {
        let mut expanded_paths: HashSet<PathBuf> = HashSet::new();
        for path in paths {
            if path.file_name() != Some(OsStr::new("OWNERS")) {
                trace!("Ignoring allowed file {:?}, not an OWNERS file", path);
                continue;
            }
            // When walking the file tree, paths are absolute.
            // Canonicalize is needed to make these paths to match.
            expanded_paths.insert(if expand {
                path.canonicalize()?
            } else {
                path.to_path_buf()
            });
            let mut parent = path.parent();
            while let Some(dir) = parent {
                if dir.as_os_str().is_empty() {
                    break;
                }
                expanded_paths.insert(if expand {
                    dir.canonicalize()?
                } else {
                    dir.to_path_buf()
                });
                parent = dir.parent();
            }
        }
        Ok(AllowList {
            allowed_files: expanded_paths,
            _private: (),
        })
    }
}

#[cfg(test)]
mod test {
    use crate::allow_filter::{AllowFilter, AllowList, FilterGitMetadata};
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    #[test]
    fn filter_git_metadata() {
        let filter = FilterGitMetadata {};
        assert!(filter.allowed(Path::new("Cargo.lock")));
        assert!(filter.allowed(Path::new("LICENSE")));
        assert!(filter.allowed(Path::new("OWNERS")));
        assert!(filter.allowed(Path::new("src/main.rs")));

        assert!(!filter.allowed(Path::new(".git/hooks/pre-commit")));
    }

    #[test]
    fn allow_list() {
        let allowed_files = ["Cargo.lock", "LICENSE", "OWNERS", "src/OWNERS"]
            .iter()
            .map(PathBuf::from)
            .collect::<HashSet<PathBuf>>();
        let filter = AllowList::from(allowed_files, false).unwrap();
        assert!(filter.allowed(Path::new("OWNERS")));
        assert!(filter.allowed(Path::new("src/OWNERS")));

        // Not OWNERS, so ignored
        assert!(!filter.allowed(Path::new("Cargo.lock")));
        assert!(!filter.allowed(Path::new("LICENSE")));

        // Not included in original allowed_files
        assert!(!filter.allowed(Path::new(".git/hooks/pre-commit")));
        assert!(!filter.allowed(Path::new("abc/OWNERS")));
        assert!(!filter.allowed(Path::new("src/main.rs")));
    }
}
