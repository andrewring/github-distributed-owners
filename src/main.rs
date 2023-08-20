use clap::Parser;
use std::path::PathBuf;

mod codeowners;
mod owners_file;
mod owners_set;
mod owners_tree;
mod pipeline;

#[cfg(test)]
mod test_utils;

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

    pipeline::generate_codeowners_from_files(
        args.repo_root,
        args.output_file,
        args.implicit_inherit,
    )
}
