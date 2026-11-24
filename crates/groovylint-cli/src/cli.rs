//! CLI argument definitions.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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

    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Number of worker threads. Defaults to available parallelism.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,
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

    /// Optional TOML contract file for project/library models.
    #[arg(long, value_name = "FILE")]
    pub contracts: Option<PathBuf>,

    /// Number of worker threads. Defaults to available parallelism.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,
}

/// Nullness analysis modes exposed by the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NullnessModeArg {
    /// Report only definitely-null dereferences.
    Conservative,
    /// Also report possible-null dereferences.
    Inferred,
}

/// Supported diagnostic output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text.
    Text,
    /// Machine-readable JSON.
    Json,
}
