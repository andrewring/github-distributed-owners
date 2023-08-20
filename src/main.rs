use crate::codeowners::{generate_codeowners, to_codeowners_string};
use crate::owners_tree::OwnersTree;
use clap::Parser;
use std::fs;
use std::fs::create_dir_all;
use std::path::PathBuf;

mod codeowners;
mod owners_file;
mod owners_set;
mod owners_tree;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
/// A tool for auto generating GitHub compatible CODEOWNERS files from OWNERS files distributed
/// through the file tree.
struct Args {
    /// Root file in the repository from which to generate a CODEOWNERS file.
    #[arg(short, long)]
    repo_root: Option<PathBuf>,

    /// Output file to write the resulting CODEOWNERS contents into.
    #[arg(short, long)]
    output_file: Option<PathBuf>,

    /// Whether to inherit owners when inheritance is not specified. Default: true.
    #[arg(short, long, default_value = "true")]
    implicit_inherit: bool,
}

fn generate_codeowners_from_files(args: Args) -> anyhow::Result<()> {
    let root = args.repo_root.unwrap_or(std::env::current_dir()?);
    let tree = OwnersTree::load_from_files(root)?;

    let codeowners = generate_codeowners(&tree, args.implicit_inherit)?;
    let mut codeowners_text = to_codeowners_string(codeowners);

    match args.output_file {
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    generate_codeowners_from_files(args)
}

#[cfg(test)]
mod test {
    use crate::{generate_codeowners_from_files, Args};
    use indoc::indoc;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_simple() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;
        let root_dir = temp_dir.path();
        fs::write(
            root_dir.join("OWNERS"),
            indoc! {
                "ada.lovelace
                grace.hopper
                "
            },
        )?;

        let expected = indoc! {"\
            / ada.lovelace grace.hopper
            "
        };

        let output_file = root_dir.join("CODEOWNERS");
        let args = Args {
            repo_root: Some(root_dir.to_path_buf()),
            output_file: Some(output_file.clone()),
            implicit_inherit: true,
        };

        generate_codeowners_from_files(args)?;

        let generated_codeowners = fs::read_to_string(output_file)?;

        assert_eq!(generated_codeowners, expected);

        Ok(())
    }
}
