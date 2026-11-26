//! CLI argument definitions.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::changed::ChangedArgs;

/// Static analysis for Groovy, inspired by `CodeNarc` and Checker Framework.
#[derive(Debug, Parser)]
#[command(name = "groovylint", version, about, long_about = None)]
pub struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Analyze Groovy source files with `CodeNarc` rules.
    Check(CheckArgs),
    /// Run annotation-free nullness analysis.
    Nullcheck(NullcheckArgs),
    /// Run project architecture rules.
    Arch(ArchArgs),
}

/// Arguments for `groovylint check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Files or directories to analyze.
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// `CodeNarc` XML ruleset. Defaults to the vendored `CodeNarc` basic ruleset.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Skip every path containing this substring. Repeatable.
    ///
    /// Use it to mirror a main-only `CodeNarc` setup: `--exclude /src/test/`.
    #[arg(long, value_name = "SUBSTRING")]
    pub exclude: Vec<String>,

    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Number of worker threads. Defaults to available parallelism.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,

    /// Restrict the analysis to files the branch changed.
    #[command(flatten)]
    pub changed: ChangedArgs,
}

/// Arguments for `groovylint arch`.
#[derive(Debug, Args)]
pub struct ArchArgs {
    /// Files or directories to analyze.
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Architecture TOML configuration.
    #[arg(short, long, value_name = "FILE")]
    pub config: PathBuf,

    /// Skip every path containing this substring. Repeatable.
    #[arg(long, value_name = "SUBSTRING")]
    pub exclude: Vec<String>,

    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Number of worker threads. Defaults to available parallelism.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,

    /// Restrict the analysis to files the branch changed.
    #[command(flatten)]
    pub changed: ChangedArgs,
}

/// Arguments for `groovylint nullcheck`.
#[derive(Debug, Args)]
pub struct NullcheckArgs {
    /// Files or directories to analyze.
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Nullness reporting mode.
    #[arg(long, value_enum, default_value_t = NullnessModeArg::Conservative)]
    pub mode: NullnessModeArg,

    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Skip every path containing this substring. Repeatable.
    #[arg(long, value_name = "SUBSTRING")]
    pub exclude: Vec<String>,

    /// Optional TOML contract file for project/library models.
    #[arg(long, value_name = "FILE")]
    pub contracts: Option<PathBuf>,

    /// Only analyze classes or packages marked with `@NullMarked`/`@NonNullApi`.
    #[arg(long)]
    pub marked_only: bool,

    /// Report non-null fields that constructors never initialize.
    ///
    /// Turn this off for frameworks that inject fields (`Spring`, `MyBatis`).
    #[arg(
        long,
        value_name = "BOOL",
        default_value_t = true,
        action = clap::ArgAction::Set
    )]
    pub initialization_check: bool,

    /// Number of worker threads. Defaults to available parallelism.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,

    /// Restrict the analysis to files the branch changed.
    #[command(flatten)]
    pub changed: ChangedArgs,
}

/// Nullness analysis modes exposed by the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NullnessModeArg {
    /// Report only definitely-null dereferences.
    Conservative,
    /// Also report possible-null dereferences.
    Inferred,
    /// Inferred plus definite field-initialisation checks.
    Strict,
}

/// Supported diagnostic output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text.
    Text,
    /// Machine-readable JSON.
    Json,
}
