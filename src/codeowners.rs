use crate::owners_tree::{OwnersTree, TreeNode};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn to_codeowners_string(codeowners: HashMap<String, HashSet<String>>) -> String {
    codeowners
        .keys()
        .sorted()
        .map(|pattern| {
            let mut line = pattern.to_string();
            if line == "/" {
                // Unlike non-root directories, the repo root directory cannot be used as a catch all path.
                // Instead, you have to use `*` at the root directory to achieve the same results.
                // https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners
                line = "*".to_string();
            }
            let owners = codeowners
                .get(pattern)
                .unwrap()
                .iter()
                .sorted()
                .map(|owner| {
                    // CODEOWNERS syntax can take any of the following formats:
                    // - @<username>
                    // - user@email.tld
                    // - @org/group
                    // For non-email versions, we can safely protect against errors
                    // by prepending an @
                    if owner.contains('@') {
                        owner.to_string()
                    } else {
                        format!("@{}", owner)
                    }
                })
                .join(" ");
            if !owners.is_empty() {
                line = format!("{} {}", line, owners);
            }
            line
        })
        // Don't include a root level owner line if no owners are specified
        .filter(|line| line != "*")
        .join("\n")
}

pub fn generate_codeowners(
    owners_tree: &OwnersTree,
    implicit_inherit: bool,
) -> anyhow::Result<HashMap<String, HashSet<String>>> {
    let mut codeowners = HashMap::new();
    add_codeowners(
        owners_tree,
        &owners_tree.path,
        &HashSet::default(),
        implicit_inherit,
        &mut codeowners,
    )?;
    Ok(codeowners)
}

fn add_codeowners(
    tree_node: &TreeNode,
    root_path: &Path,
    parent_owners: &HashSet<String>,
    implicit_inherit: bool,
    codeowners: &mut HashMap<String, HashSet<String>>,
) -> anyhow::Result<()> {
    let owners_config = &tree_node.owners_config;
    let owners_set = &owners_config.all_files;
    let mut relative_path = tree_node
        .path
        .strip_prefix(root_path)?
        .to_string_lossy()
        .to_string()
        + "/";
    // Always use explicit paths from root
    if !relative_path.starts_with('/') {
        relative_path = format!("/{}", relative_path);
    }

    // Gather directory level owners
    let mut owners = HashSet::default();
    if owners_set.inherit == Some(true) || (implicit_inherit && owners_set.inherit.is_none()) {
        owners.extend(parent_owners.clone());
    }
    owners.extend(owners_set.owners.clone());

    // Add directory level ownership
    codeowners.insert(relative_path.clone(), owners.clone());

    // Add overrides
    for (override_pattern, override_owners_set) in &owners_config.pattern_overrides {
        let mut override_owners = override_owners_set.owners.clone();
        if override_owners_set.inherit == Some(true)
            || implicit_inherit && override_owners_set.inherit.is_none()
        {
            override_owners.extend(owners.clone());
        }
        let mut pattern = relative_path.to_owned();
        pattern.push_str(override_pattern.as_str());
        codeowners.insert(pattern, override_owners);
    }

    for child in &tree_node.children {
        add_codeowners(child, root_path, &owners, implicit_inherit, codeowners)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::codeowners::{generate_codeowners, to_codeowners_string};
    use crate::owners_file::OwnersFileConfig;
    use crate::owners_set::OwnersSet;
    use crate::owners_tree::TreeNode;
    use indoc::indoc;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    #[test]
    fn generate_codeowners_single_simple() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace", "grace.hopper", "margaret.hamilton"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::default(),
            },
            children: Vec::default(),
        };
        let implicit_inherit = true;

        let expected = HashMap::from([(
            "/".to_string(),
            vec!["ada.lovelace", "grace.hopper", "margaret.hamilton"]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
        )]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_multiple_simple() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace", "grace.hopper"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::default(),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: None,
                        owners: vec!["margaret.hamilton", "katherine.johnson"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                    },
                    pattern_overrides: HashMap::default(),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = true;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec![
                    "ada.lovelace",
                    "grace.hopper",
                    "margaret.hamilton",
                    "katherine.johnson",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_single_with_overrides() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace", "grace.hopper"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::from([(
                    "*.rs".to_string(),
                    OwnersSet {
                        owners: vec!["margaret.hamilton", "katherine.johnson"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                        ..OwnersSet::default()
                    },
                )]),
            },
            children: Vec::default(),
        };
        let implicit_inherit = true;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec![
                    "ada.lovelace",
                    "grace.hopper",
                    "margaret.hamilton",
                    "katherine.johnson",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_multiple_with_overrides() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::from([(
                    "*.rs".to_string(),
                    OwnersSet {
                        owners: vec!["margaret.hamilton"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                        ..OwnersSet::default()
                    },
                )]),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: None,
                        owners: vec!["grace.hopper"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                    },
                    pattern_overrides: HashMap::from([(
                        "*.rs".to_string(),
                        OwnersSet {
                            owners: vec!["katherine.johnson"]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                    )]),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = true;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["ada.lovelace", "margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["ada.lovelace", "grace.hopper", "katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_no_implicit_inherit() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::from([(
                    "*.rs".to_string(),
                    OwnersSet {
                        owners: vec!["margaret.hamilton"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                        ..OwnersSet::default()
                    },
                )]),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: None,
                        owners: vec!["grace.hopper"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                    },
                    pattern_overrides: HashMap::from([(
                        "*.rs".to_string(),
                        OwnersSet {
                            owners: vec!["katherine.johnson"]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                    )]),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = false;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_selective_inherit() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::from([(
                    "*.rs".to_string(),
                    OwnersSet {
                        owners: vec!["margaret.hamilton"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                        inherit: Some(false),
                    },
                )]),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: Some(false),
                        owners: vec!["grace.hopper"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                    },
                    pattern_overrides: HashMap::from([(
                        "*.rs".to_string(),
                        OwnersSet {
                            owners: vec!["katherine.johnson"]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                    )]),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = true;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["grace.hopper", "katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_selective_inherit_with_no_implicit() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::from([(
                    "*.rs".to_string(),
                    OwnersSet {
                        owners: vec!["margaret.hamilton"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                        inherit: Some(true),
                    },
                )]),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: Some(true),
                        owners: vec!["grace.hopper"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<HashSet<String>>(),
                    },
                    pattern_overrides: HashMap::from([(
                        "*.rs".to_string(),
                        OwnersSet {
                            owners: vec!["katherine.johnson"]
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<HashSet<String>>(),
                            ..OwnersSet::default()
                        },
                    )]),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = false;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["ada.lovelace", "margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn generate_codeowners_subdir_without_owners() -> anyhow::Result<()> {
        let tree_node = TreeNode {
            path: PathBuf::from("/tree/root"),
            repo_base: PathBuf::from("/tree/root"),
            owners_config: OwnersFileConfig {
                all_files: OwnersSet {
                    inherit: None,
                    owners: vec!["ada.lovelace", "grace.hopper"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<String>>(),
                },
                pattern_overrides: HashMap::default(),
            },
            children: vec![TreeNode {
                path: PathBuf::from("/tree/root/foo/bar"),
                repo_base: PathBuf::from("/tree/root"),
                owners_config: OwnersFileConfig {
                    all_files: OwnersSet {
                        inherit: Some(false),
                        owners: HashSet::default(),
                    },
                    pattern_overrides: HashMap::default(),
                },
                children: vec![],
            }],
        };
        let implicit_inherit = true;

        let expected = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            ("/foo/bar/".to_string(), HashSet::default()),
        ]);

        let codeowners = generate_codeowners(&tree_node, implicit_inherit)?;

        assert_eq!(codeowners, expected);

        Ok(())
    }

    #[test]
    fn to_codeowners_string_multilevel() -> anyhow::Result<()> {
        let codeowners = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["ada.lovelace", "margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let expected = indoc!(
            "* @ada.lovelace
            /*.rs @ada.lovelace @margaret.hamilton
            /foo/bar/ @ada.lovelace @grace.hopper
            /foo/bar/*.rs @katherine.johnson"
        )
        .to_string();

        let codeowners_text = to_codeowners_string(codeowners);

        assert_eq!(codeowners_text, expected);

        Ok(())
    }

    #[test]
    fn to_codeowners_string_multilevel_sorting() -> anyhow::Result<()> {
        let codeowners = HashMap::from([
            (
                "/foo/bar/*.rs".to_string(),
                vec!["katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/foo/bar/".to_string(),
                vec!["ada.lovelace", "grace.hopper"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["ada.lovelace", "margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let expected = indoc!(
            "* @ada.lovelace
            /*.rs @ada.lovelace @margaret.hamilton
            /foo/bar/ @ada.lovelace @grace.hopper
            /foo/bar/*.rs @katherine.johnson"
        )
        .to_string();

        let codeowners_text = to_codeowners_string(codeowners);

        assert_eq!(codeowners_text, expected);

        Ok(())
    }

    #[test]
    fn to_codeowners_string_subdir_without_owners() -> anyhow::Result<()> {
        let codeowners = HashMap::from([
            (
                "/".to_string(),
                vec!["ada.lovelace"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            (
                "/*.rs".to_string(),
                vec!["ada.lovelace", "margaret.hamilton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
            ("/foo/bar/".to_string(), HashSet::default()),
            (
                "/foo/bar/*.rs".to_string(),
                vec!["katherine.johnson"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<HashSet<String>>(),
            ),
        ]);

        let expected = indoc!(
            "* @ada.lovelace
            /*.rs @ada.lovelace @margaret.hamilton
            /foo/bar/
            /foo/bar/*.rs @katherine.johnson"
        )
        .to_string();

        let codeowners_text = to_codeowners_string(codeowners);

        assert_eq!(codeowners_text, expected);

        Ok(())
    }
}
