use anyhow::anyhow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait AllowFilter {
    fn allowed(&self, path: &Path) -> bool;
}

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
        let git_files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect();
        let allow_list = AllowList {
            allowed_files: git_files,
        };
        Ok(allow_list)
    }
}
