use crate::allow_filter::{AllowFilter, AllowList, FilterGitMetadata};
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use std::path::PathBuf;

mod codeowners;
mod owners_file;
mod owners_set;
mod owners_tree;
mod pipeline;

mod allow_filter;
#[cfg(test)]
mod test_utils;

const DEFAULT_IMPLICIT_INHERIT: bool = true;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
/// A tool for auto generating GitHub compatible CODEOWNERS files from OWNERS files distributed
/// through the file tree.
struct Args {
    /// Root file in the repository from which to generate a CODEOWNERS file.
    #[clap(short, long)]
    repo_root: Option<PathBuf>,

    /// Output file to write the resulting CODEOWNERS contents into.
    #[clap(short, long)]
    output_file: Option<PathBuf>,

    /// Whether to inherit owners when inheritance is not specified. Default: true.
    #[clap(short, long, parse(try_from_str))]
    // NB: Option<bool> allows for --implicit-inherit [true|false]
    implicit_inherit: Option<bool>,

    /// Don't filter out files which are not managed by git.
    #[clap(long)]
    allow_non_git_files: bool,

    /// Add custom message to the auto-generated header/footer.
    ///
    /// This can be useful if you want to provide context for your specific project,
    /// such as manual steps to regenerate the file.
    #[clap(short, long)]
    message: Option<String>,

    #[clap(flatten)]
    verbose: Verbosity,
}

fn run_pipeline<F: AllowFilter>(args: Args, allow_filter: &F) -> anyhow::Result<()> {
    pipeline::generate_codeowners_from_files(
        args.repo_root,
        args.output_file,
        args.implicit_inherit.unwrap_or(DEFAULT_IMPLICIT_INHERIT),
        allow_filter,
        args.message,
    )
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    if args.allow_non_git_files {
        let allow_filter = FilterGitMetadata {};
        run_pipeline(args, &allow_filter)
    } else {
        let allow_filter = AllowList::allow_git_files()?;
        run_pipeline(args, &allow_filter)
    }
}
