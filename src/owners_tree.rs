use crate::allow_filter::AllowFilter;
use crate::owners_file::OwnersFileConfig;
use log::{debug, trace};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug, Default)]
pub struct TreeNode {
    pub path: PathBuf,
    pub repo_base: PathBuf,
    pub owners_config: OwnersFileConfig,
    pub children: Vec<TreeNode>,
}

pub type OwnersTree = TreeNode;

impl TreeNode {
    pub fn new<P0: AsRef<Path>, P1: AsRef<Path>>(path: P0, repo_base: P1) -> TreeNode {
        TreeNode {
            path: path
                .as_ref()
                .to_path_buf()
                .canonicalize()
                .expect("Failed to canonicalize path"),
            repo_base: repo_base
                .as_ref()
                .to_path_buf()
                .canonicalize()
                .expect("Failed to canonicalize path"),
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
        let owners_config = OwnersFileConfig::from_file(owners_file, &self.repo_base)?;
        self.owners_config = owners_config;

        Ok(true)
    }

    pub fn load_from_files<P, F>(root: P, allow_filter: &F) -> anyhow::Result<TreeNode>
    where
        P: AsRef<Path>,
        F: AllowFilter,
    {
        let mut root_node = TreeNode::new(&root, &root);
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
        let mut current_loc_node = TreeNode::new(directory, &self.repo_base);
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
    use std::collections::HashMap;
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
        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir_path.to_path_buf(),
            repo_base: temp_dir_path.to_path_buf(),
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
        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir_path.to_path_buf(),
            repo_base: temp_dir_path.to_path_buf(),
            children: vec![TreeNode {
                path: temp_dir_path.join("subdir").to_path_buf(),
                repo_base: temp_dir_path.to_path_buf(),
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
        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir_path.to_path_buf(),
            repo_base: temp_dir_path.to_path_buf(),
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
                path: temp_dir_path.join("subdir/foo").to_path_buf(),
                repo_base: temp_dir_path.to_path_buf(),
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
        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY)?;
        let expected = TreeNode {
            path: temp_dir_path.to_path_buf(),
            repo_base: temp_dir_path.to_path_buf(),
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

    #[test]
    fn included_file() -> anyhow::Result<()> {
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

                include /subdir/bar/OWNERS
                include ../baz/OWNERS
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/bar/OWNERS",
            indoc! {"\
                mary.jackson
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/baz/OWNERS",
            indoc! {"\
                [*.py]
                alan.turing
                "
            },
        )?;
        let temp_dir_path = temp_dir.path().canonicalize()?;
        let mut tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY)?;
        let mut expected = TreeNode {
            path: temp_dir_path.to_path_buf(),
            repo_base: temp_dir_path.to_path_buf(),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    owners: vec!["ada.lovelace".to_string(), "grace.hopper".to_string()]
                        .into_iter()
                        .collect::<HashSet<String>>(),
                    ..OwnersSet::default()
                },
                ..OwnersFileConfig::default()
            },
            children: vec![
                TreeNode {
                    path: temp_dir_path.join("subdir/foo").to_path_buf(),
                    repo_base: temp_dir_path.to_path_buf(),
                    owners_config: OwnersFileConfig {
                        all_files: OwnersSet {
                            owners: vec![
                                "margaret.hamilton".to_string(),
                                "katherine.johnson".to_string(),
                                "mary.jackson".to_string(),
                            ]
                            .into_iter()
                            .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                        pattern_overrides: HashMap::from([(
                            "*.py".to_string(),
                            OwnersSet {
                                owners: vec!["alan.turing".to_string()]
                                    .into_iter()
                                    .collect::<HashSet<String>>(),
                                ..OwnersSet::default()
                            },
                        )]),
                        ..OwnersFileConfig::default()
                    },
                    ..TreeNode::default()
                },
                TreeNode {
                    path: temp_dir_path.join("subdir/bar").to_path_buf(),
                    repo_base: temp_dir_path.to_path_buf(),
                    owners_config: OwnersFileConfig {
                        all_files: OwnersSet {
                            owners: vec!["mary.jackson".to_string()]
                                .into_iter()
                                .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                        ..OwnersFileConfig::default()
                    },
                    ..TreeNode::default()
                },
                TreeNode {
                    path: temp_dir_path.join("subdir/baz").to_path_buf(),
                    repo_base: temp_dir_path.to_path_buf(),
                    owners_config: OwnersFileConfig {
                        all_files: OwnersSet {
                            owners: vec![].into_iter().collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                        pattern_overrides: HashMap::from([(
                            "*.py".to_string(),
                            OwnersSet {
                                owners: vec!["alan.turing".to_string()]
                                    .into_iter()
                                    .collect::<HashSet<String>>(),
                                ..OwnersSet::default()
                            },
                        )]),
                        ..OwnersFileConfig::default()
                    },
                    ..TreeNode::default()
                },
            ],
        };

        tree.children.sort_by_key(|c| c.path.clone());
        expected.children.sort_by_key(|c| c.path.clone());

        assert_eq!(tree, expected);
        Ok(())
    }

    #[test]
    fn included_file_circular_include() -> anyhow::Result<()> {
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
                include /subdir/bar/OWNERS
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/bar/OWNERS",
            indoc! {"\
                include /subdir/bar/OWNERS
                "
            },
        )?;

        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY);

        assert!(tree.is_err());
        Ok(())
    }

    #[test]
    fn included_file_set_statement() -> anyhow::Result<()> {
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
                include /subdir/bar/OWNERS
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/bar/OWNERS",
            indoc! {"\
                set inherit = true
                "
            },
        )?;

        let temp_dir_path = temp_dir.path().canonicalize()?;
        let tree = OwnersTree::load_from_files(&temp_dir_path, &ALLOW_ANY);

        assert!(tree.is_err());
        Ok(())
    }
}
