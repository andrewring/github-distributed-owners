use crate::allow_filter::AllowFilter;
use crate::codeowners::{generate_codeowners, to_codeowners_string};
use crate::owners_tree::OwnersTree;
use indoc::indoc;
use std::fs;
use std::fs::create_dir_all;
use std::path::PathBuf;
use textwrap::wrap;

fn get_auto_generated_notice<S: AsRef<str>>(message: Option<S>) -> String {
    let mut out = indoc! {"\
        ################################################################################
        #                             AUTO GENERATED FILE
        #                            Do Not Manually Update
        "
    }
    .to_string();

    if let Some(message) = message {
        wrap(message.as_ref(), 78).iter().for_each(|line| {
            out.push_str(format!("# {: ^78}", line).trim());
            out.push('\n');
        });
    }

    out.push_str(indoc! {"\
        #                              For details, see:
        #        https://github.com/andrewring/github-distributed-owners#readme
        ################################################################################"
    });

    out
}

pub fn generate_codeowners_from_files<F, S>(
    repo_root: Option<PathBuf>,
    output_file: Option<PathBuf>,
    implicit_inherit: bool,
    allow_filter: &F,
    message: Option<S>,
) -> anyhow::Result<()>
where
    F: AllowFilter,
    S: AsRef<str>,
{
    let root = repo_root.unwrap_or(std::env::current_dir()?);
    let tree = OwnersTree::load_from_files(root, allow_filter)?;

    let codeowners = generate_codeowners(&tree, implicit_inherit)?;
    let mut codeowners_text = to_codeowners_string(codeowners);
    let auto_generated_notice = get_auto_generated_notice(message);

    codeowners_text =
        format!("{auto_generated_notice}\n\n{codeowners_text}\n\n{auto_generated_notice}");

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
    use crate::allow_filter::FilterGitMetadata;
    use crate::pipeline::{generate_codeowners_from_files, get_auto_generated_notice};
    use crate::test_utils::create_test_file;
    use indoc::indoc;
    use std::fs;
    use tempfile::tempdir;

    const ALLOW_ANY: FilterGitMetadata = FilterGitMetadata {};

    #[test]
    fn test_generate_codeowners_from_files_simple() -> anyhow::Result<()> {
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

            * @ada.lovelace @grace.hopper
            /*.rs @ada.lovelace @foo.bar @grace.hopper

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
        let message = Option::<String>::None;

        generate_codeowners_from_files(
            repo_root,
            Some(output_file.clone()),
            implicit_inherit,
            &ALLOW_ANY,
            message,
        )?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_generate_codeowners_from_files_multiple_files() -> anyhow::Result<()> {
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

            * @ada.lovelace @grace.hopper
            /subdir/foo/ @ada.lovelace @grace.hopper @katherine.johnson @margaret.hamilton

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
        let message = Option::<String>::None;

        generate_codeowners_from_files(
            repo_root,
            Some(output_file.clone()),
            implicit_inherit,
            &ALLOW_ANY,
            message,
        )?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_generate_codeowners_from_files_multiple_files_with_overrides() -> anyhow::Result<()> {
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

            * @ada.lovelace @grace.hopper
            /subdir/foo/ @ada.lovelace @grace.hopper @katherine.johnson @margaret.hamilton
            /subdir/foo/*.rs @grace.hopper

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
        let message = Option::<String>::None;

        generate_codeowners_from_files(
            repo_root,
            Some(output_file.clone()),
            implicit_inherit,
            &ALLOW_ANY,
            message,
        )?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_generate_codeowners_from_files_empty_root_blanket_owners() -> anyhow::Result<()> {
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

            /*.rs @ada.lovelace @grace.hopper
            /subdir/foo/ @katherine.johnson @margaret.hamilton
            /subdir/foo/*.rs @grace.hopper

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
        let message = Option::<String>::None;

        generate_codeowners_from_files(
            repo_root,
            Some(output_file.clone()),
            implicit_inherit,
            &ALLOW_ANY,
            message,
        )?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }

    #[test]
    fn test_get_auto_generated_notice_default() {
        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################"
        };
        assert_eq!(get_auto_generated_notice::<String>(None), expected);
    }

    #[test]
    fn test_get_auto_generated_notice_short() {
        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            #                          Some short text on one line
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################"
        };
        let message = "Some short text on one line";
        assert_eq!(get_auto_generated_notice(Some(message)), expected);
    }

    #[test]
    fn test_get_auto_generated_notice_multiline() {
        let expected = indoc! {"\
            ################################################################################
            #                             AUTO GENERATED FILE
            #                            Do Not Manually Update
            # A much longer custom message which doesn't fit on a single line. It will need
            #                   to be wrapped into multiple lines, neatly.
            #                              For details, see:
            #        https://github.com/andrewring/github-distributed-owners#readme
            ################################################################################"
        };
        let message =
            "A much longer custom message which doesn't fit on a single line. It will need to be wrapped into multiple \
            lines, neatly.";
        assert_eq!(get_auto_generated_notice(Some(message)), expected);
    }
}
