use crate::allow_filter::AllowFilter;
use crate::owners_file::OwnersFileConfig;
use log::{debug, trace};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug, Default)]
pub struct TreeNode {
    pub path: PathBuf,
    pub owners_config: OwnersFileConfig,
    pub children: Vec<TreeNode>,
}

pub type OwnersTree = TreeNode;

impl TreeNode {
    pub fn new<P: AsRef<Path>>(path: P) -> TreeNode {
        TreeNode {
            path: path.as_ref().to_path_buf(),
            ..TreeNode::default()
        }
    }

    pub fn maybe_load_owners_file<F>(&mut self, allow_filter: &F) -> anyhow::Result<bool>
    where
        F: AllowFilter,
    {
        let owners_file = self.path.join("OWNERS");
        if !owners_file.exists() || !owners_file.is_file() {
            return Ok(false);
        }
        if !allow_filter.allowed(&owners_file) {
            trace!(
                "Skipping {:?} in {:?} due to filter",
                owners_file,
                self.path
            );
            return Ok(false);
        }

        debug!("Parsing {:?}", &owners_file);
        let owners_config = OwnersFileConfig::from_file(owners_file)?;
        self.owners_config = owners_config;

        Ok(true)
    }

    pub fn load_from_files<P, F>(root: P, allow_filter: &F) -> anyhow::Result<TreeNode>
    where
        P: AsRef<Path>,
        F: AllowFilter,
    {
        let mut root_node = TreeNode::new(&root);
        root_node.maybe_load_owners_file(allow_filter)?;
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() &&
                // Don't process file tree branches with no allowed files
                allow_filter.allowed(&path)
            {
                root_node.load_children_from_files(&path, allow_filter)?;
            }
        }
        Ok(root_node)
    }

    fn load_children_from_files<F>(
        &mut self,
        directory: &Path,
        allow_filter: &F,
    ) -> anyhow::Result<()>
    where
        F: AllowFilter,
    {
        if directory.file_name().unwrap() == ".git" {
            // Don't process git metadata
            return Ok(());
        }
        let mut current_loc_node = TreeNode::new(directory);
        let has_current_owners_file = current_loc_node.maybe_load_owners_file(allow_filter)?;
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if has_current_owners_file {
                    current_loc_node.load_children_from_files(&path, allow_filter)?;
                } else {
                    self.load_children_from_files(&path, allow_filter)?;
                }
            }
        }
        if has_current_owners_file {
            self.children.push(current_loc_node);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::allow_filter::FilterGitMetadata;
    use crate::owners_file::OwnersFileConfig;
    use crate::owners_set::OwnersSet;
    use crate::owners_tree::{OwnersTree, TreeNode};
    use crate::test_utils::create_test_file;
    use indoc::indoc;
    use std::collections::HashSet;
    use tempfile::tempdir;

    const ALLOW_ANY: FilterGitMetadata = FilterGitMetadata {};

    #[test]
    fn single_file_at_root() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {"\
                ada.lovelace
                grace.hopper
                margaret.hamilton
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path(), &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir.path().to_path_buf(),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    owners: vec![
                        "ada.lovelace".to_string(),
                        "grace.hopper".to_string(),
                        "margaret.hamilton".to_string(),
                    ]
                    .into_iter()
                    .collect::<HashSet<String>>(),
                    ..OwnersSet::default()
                },
                ..OwnersFileConfig::default()
            },
            ..TreeNode::default()
        };

        assert_eq!(tree, expected);
        Ok(())
    }

    #[test]
    fn single_file_not_at_root() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        create_test_file(
            &temp_dir,
            "subdir/OWNERS",
            indoc! {"\
                ada.lovelace
                grace.hopper
                margaret.hamilton
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path(), &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir.path().to_path_buf(),
            children: vec![TreeNode {
                path: temp_dir.path().join("subdir").to_path_buf(),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        owners: vec![
                            "ada.lovelace".to_string(),
                            "grace.hopper".to_string(),
                            "margaret.hamilton".to_string(),
                        ]
                        .into_iter()
                        .collect::<HashSet<String>>(),
                        ..OwnersSet::default()
                    },
                    ..OwnersFileConfig::default()
                },
                ..TreeNode::default()
            }],
            ..TreeNode::default()
        };

        assert_eq!(tree, expected);
        Ok(())
    }

    #[test]
    fn multiple_files() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {"\
                ada.lovelace
                grace.hopper
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/foo/OWNERS",
            indoc! {"\
                margaret.hamilton
                katherine.johnson
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path(), &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir.path().to_path_buf(),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    owners: vec!["ada.lovelace".to_string(), "grace.hopper".to_string()]
                        .into_iter()
                        .collect::<HashSet<String>>(),
                    ..OwnersSet::default()
                },
                ..OwnersFileConfig::default()
            },
            children: vec![TreeNode {
                path: temp_dir.path().join("subdir/foo").to_path_buf(),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        owners: vec![
                            "margaret.hamilton".to_string(),
                            "katherine.johnson".to_string(),
                        ]
                        .into_iter()
                        .collect::<HashSet<String>>(),
                        ..OwnersSet::default()
                    },
                    ..OwnersFileConfig::default()
                },
                ..TreeNode::default()
            }],
        };

        assert_eq!(tree, expected);
        Ok(())
    }

    #[test]
    fn ignore_hidden_files() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {"\
                ada.lovelace
                grace.hopper
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/.git/OWNERS",
            indoc! {"\
                margaret.hamilton
                katherine.johnson
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path(), &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir.path().to_path_buf(),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    owners: vec!["ada.lovelace".to_string(), "grace.hopper".to_string()]
                        .into_iter()
                        .collect::<HashSet<String>>(),
                    ..OwnersSet::default()
                },
                ..OwnersFileConfig::default()
            },
            ..TreeNode::default()
        };

        assert_eq!(tree, expected);
        Ok(())
    }
}
