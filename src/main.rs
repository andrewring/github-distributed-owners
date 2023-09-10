use crate::allow_filter::{AllowFilter, AllowList, FilterGitMetadata};
use clap::Parser;
use std::path::PathBuf;

mod codeowners;
mod owners_file;
mod owners_set;
mod owners_tree;
mod pipeline;

mod allow_filter;
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
    // NB: Option<bool> allows for --implicit-inherit [true|false], default means it's always Some
    implicit_inherit: Option<bool>,

    /// Don't filter out files which are not managed by git.
    #[arg(long)]
    allow_non_git_files: bool,
}

fn run_pipeline<F: AllowFilter>(args: Args, allow_filter: &F) -> anyhow::Result<()> {
    pipeline::generate_codeowners_from_files(
        args.repo_root,
        args.output_file,
        args.implicit_inherit
            .expect("--implicit-inherit must be set to either true or false"),
        allow_filter,
    )
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.allow_non_git_files {
        let allow_filter = FilterGitMetadata {};
        run_pipeline(args, &allow_filter)
    } else {
        let allow_filter = AllowList::allow_git_files()?;
        run_pipeline(args, &allow_filter)
    }
}
