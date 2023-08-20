use crate::codeowners::{generate_codeowners, to_codeowners_string};
use crate::owners_tree::OwnersTree;
use clap::Parser;
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("args: {:#?}", &args);

    let root = args.repo_root.unwrap_or(std::env::current_dir()?);
    let tree = OwnersTree::load_from_files(root)?;
    println!("tree: {:#?}", &tree);

    let codeowners = generate_codeowners(&tree, args.implicit_inherit)?;
    let codeowners_text = to_codeowners_string(codeowners);
    println!("{}", codeowners_text);
    Ok(())
}
