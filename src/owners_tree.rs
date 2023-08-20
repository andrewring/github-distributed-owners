use crate::owners_file::OwnersFileConfig;
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

    pub fn maybe_load_owners_file(&mut self) -> anyhow::Result<bool> {
        let owners_file = self.path.join("OWNERS");
        if !owners_file.exists() || !owners_file.is_file() {
            return Ok(false);
        }

        let owners_config = OwnersFileConfig::from_file(owners_file)?;
        self.owners_config = owners_config;

        Ok(true)
    }

    pub fn load_from_files<P: AsRef<Path>>(root: P) -> anyhow::Result<TreeNode> {
        let mut root_node = TreeNode::new(&root);
        root_node.maybe_load_owners_file()?;
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                root_node.load_children_from_files(&path)?;
            }
        }
        Ok(root_node)
    }

    fn load_children_from_files(&mut self, directory: &Path) -> anyhow::Result<()> {
        let mut current_loc_node = TreeNode::new(directory);
        let has_current_owners_file = current_loc_node.maybe_load_owners_file()?;
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if has_current_owners_file {
                    current_loc_node.load_children_from_files(&path)?;
                } else {
                    self.load_children_from_files(&path)?;
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
    use crate::owners_file::OwnersFileConfig;
    use crate::owners_set::OwnersSet;
    use crate::owners_tree::{OwnersTree, TreeNode};
    use indoc::indoc;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn single_file_at_root() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        fs::write(
            temp_dir.path().join("OWNERS"),
            indoc! {"\
                ada.lovelace
                grace.hopper
                margaret.hamilton
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path())?;
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
        let owners_dir = temp_dir.path().join("subdir");
        fs::create_dir_all(&owners_dir)?;
        fs::write(
            owners_dir.join("OWNERS"),
            indoc! {"\
                ada.lovelace
                grace.hopper
                margaret.hamilton
                "
            },
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path())?;
        let expected = TreeNode {
            path: temp_dir.path().to_path_buf(),
            children: vec![TreeNode {
                path: owners_dir.to_path_buf(),
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
        fs::write(
            temp_dir.path().join("OWNERS"),
            indoc! {"\
                ada.lovelace
                grace.hopper
                "
            },
        )?;
        let subdir = temp_dir.path().join("subdir").join("foo");
        fs::create_dir_all(&subdir)?;
        fs::write(
            subdir.join("OWNERS"),
            r#"
            margaret.hamilton
            katherine.johnson
        "#,
        )?;
        let tree = OwnersTree::load_from_files(temp_dir.path())?;
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
                path: subdir.to_path_buf(),
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
}
