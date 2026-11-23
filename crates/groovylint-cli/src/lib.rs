//! Command-line interface for `groovylint`.

mod cli;
mod discovery;
mod report;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::thread::available_parallelism;

use anyhow::{Context, Result, bail};
use clap::Parser;
use groovylint_config as config;
use groovylint_core::{Diagnostic, Severity, SourceFile};
use groovylint_rules::{Engine, RuleRegistry};
use groovylint_syntax::parser::parse;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use crate::report::{FileReport, summarize, summary_line, write_json, write_text};

/// Parses command-line arguments and runs the requested command.
pub fn run() -> ExitCode {
    let cli = cli::Cli::parse();
    match try_run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn try_run(cli: cli::Cli) -> Result<ExitCode> {
    match cli.command {
        cli::Command::Check(args) => run_check(&args),
    }
}

fn run_check(args: &cli::CheckArgs) -> Result<ExitCode> {
    let registry = RuleRegistry::builtin();
    let (ruleset, ruleset_label) = if let Some(path) = &args.config {
        (config::load(path)?, path.display().to_string())
    } else {
        (
            config::default_basic(),
            "rulesets/basic.xml (CodeNarc 4.0.0)".to_string(),
        )
    };

    for warning in &ruleset.warnings {
        eprintln!("warning: {warning}");
    }

    let class_names = ruleset.class_names();
    let engine = Engine::from_codenarc_classes(&registry, &class_names);
    if !engine.unsupported().is_empty() {
        eprintln!(
            "note: {} rule(s) enabled from {ruleset_label}; {} CodeNarc rule(s) are not implemented yet and were skipped",
            engine.rule_count(),
            engine.unsupported().len()
        );
    }

    let files = discovery::collect_files(&args.paths)?;
    if files.is_empty() {
        eprintln!("no Groovy source files found");
        return Ok(ExitCode::SUCCESS);
    }

    let threads = args
        .threads
        .unwrap_or_else(|| available_parallelism().map_or(1, std::num::NonZeroUsize::get));
    if threads == 0 {
        bail!("--threads must be greater than zero");
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .context("failed to create worker thread pool")?;

    let reports = pool.install(|| {
        files
            .par_iter()
            .map(|path| analyze_file(&engine, path))
            .collect::<Result<Vec<_>>>()
    })?;

    let summary = summarize(&reports);
    match args.format {
        cli::OutputFormat::Text => {
            write_text(&reports);
            eprintln!("{}", summary_line(summary));
        }
        cli::OutputFormat::Json => {
            print!("{}", write_json(&reports, summary)?);
        }
    }

    if summary.total() > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn analyze_file(engine: &Engine, path: &Path) -> Result<FileReport> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let source = SourceFile::new(path.to_path_buf(), text);

    let diagnostics = match parse(source.text()) {
        Ok(parsed) => engine.lint(&source, &parsed),
        Err(error) => vec![
            Diagnostic::new(
                "ParseError",
                Severity::Error,
                format!("failed to parse Groovy source: {}", error.message),
                error.span,
            )
            .with_help(
                "The parser currently supports a subset of Groovy; report this file if it is valid Groovy.",
            ),
        ],
    };

    Ok(FileReport {
        source,
        diagnostics,
    })
}
