use crate::codeowners::{generate_codeowners, to_codeowners_string};
use crate::owners_tree::OwnersTree;
use indoc::indoc;
use std::fs;
use std::fs::create_dir_all;
use std::path::PathBuf;

const AUTO_GENERATED_NOTICE: &str = indoc! {"\
    ################################################################################
    #                             AUTO GENERATED FILE
    #                            Do Not Manually Update
    #                              For details, see:
    #        https://github.com/andrewring/github-distributed-owners#readme
    ################################################################################"
};

pub fn generate_codeowners_from_files(
    repo_root: Option<PathBuf>,
    output_file: Option<PathBuf>,
    implicit_inherit: bool,
) -> anyhow::Result<()> {
    let root = repo_root.unwrap_or(std::env::current_dir()?);
    let tree = OwnersTree::load_from_files(root)?;

    let codeowners = generate_codeowners(&tree, implicit_inherit)?;
    let mut codeowners_text = to_codeowners_string(codeowners);

    codeowners_text =
        format!("{AUTO_GENERATED_NOTICE}\n\n{codeowners_text}\n\n{AUTO_GENERATED_NOTICE}");

    match output_file {
        None => println!("{}", codeowners_text),
        Some(output_file) => {
            if let Some(parent_dir) = output_file.parent() {
                create_dir_all(parent_dir)?;
            }

            // Files should end with a newline
            codeowners_text.push('\n');

            fs::write(output_file, codeowners_text)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::pipeline::generate_codeowners_from_files;
    use crate::test_utils::create_test_file;
    use indoc::indoc;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_simple() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root_dir = temp_dir.path();
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {
                "ada.lovelace
                grace.hopper
                [*.rs]
                foo.bar
                "
            },
        )?;

        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################

            / ada.lovelace grace.hopper
            /*.rs ada.lovelace foo.bar grace.hopper

            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################
            "
        };

        let output_file = root_dir.join("CODEOWNERS");
        let repo_root = Some(root_dir.to_path_buf());
        let implicit_inherit = true;

        generate_codeowners_from_files(repo_root, Some(output_file.clone()), implicit_inherit)?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_multiple_files() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root_dir = temp_dir.path();
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {
                "ada.lovelace
                grace.hopper
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/foo/OWNERS",
            indoc! {"\
                katherine.johnson
                margaret.hamilton
                "
            },
        )?;

        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################

            / ada.lovelace grace.hopper
            /subdir/foo/ ada.lovelace grace.hopper katherine.johnson margaret.hamilton

            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################
            "
        };

        let output_file = root_dir.join("CODEOWNERS");
        let repo_root = Some(root_dir.to_path_buf());
        let implicit_inherit = true;

        generate_codeowners_from_files(repo_root, Some(output_file.clone()), implicit_inherit)?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_multiple_files_with_overrides() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root_dir = temp_dir.path();
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {
                "ada.lovelace
                grace.hopper
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/foo/OWNERS",
            indoc! {"\
                katherine.johnson
                margaret.hamilton

                [*.rs]
                set inherit = false
                grace.hopper
                "
            },
        )?;

        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################

            / ada.lovelace grace.hopper
            /subdir/foo/ ada.lovelace grace.hopper katherine.johnson margaret.hamilton
            /subdir/foo/*.rs grace.hopper

            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################
            "
        };

        let output_file = root_dir.join("CODEOWNERS");
        let repo_root = Some(root_dir.to_path_buf());
        let implicit_inherit = true;

        generate_codeowners_from_files(repo_root, Some(output_file.clone()), implicit_inherit)?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_empty_root_blanket_owners() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root_dir = temp_dir.path();
        create_test_file(
            &temp_dir,
            "OWNERS",
            indoc! {
                "[*.rs]
                ada.lovelace
                grace.hopper
                "
            },
        )?;
        create_test_file(
            &temp_dir,
            "subdir/foo/OWNERS",
            indoc! {"\
                katherine.johnson
                margaret.hamilton

                [*.rs]
                set inherit = false
                grace.hopper
                "
            },
        )?;

        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################

            /*.rs ada.lovelace grace.hopper
            /subdir/foo/ katherine.johnson margaret.hamilton
            /subdir/foo/*.rs grace.hopper

            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################
            "
        };

        let output_file = root_dir.join("CODEOWNERS");
        let repo_root = Some(root_dir.to_path_buf());
        let implicit_inherit = true;

        generate_codeowners_from_files(repo_root, Some(output_file.clone()), implicit_inherit)?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }
}
