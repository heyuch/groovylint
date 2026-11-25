//! Command-line interface for `groovylint`.

mod changed;
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
        cli::Command::Nullcheck(args) => run_nullcheck(&args),
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

    let entries = ruleset.entries();
    let engine = Engine::from_codenarc_entries(&registry, &entries);
    if !engine.unsupported().is_empty() {
        eprintln!(
            "note: {} rule(s) enabled from {ruleset_label}; {} CodeNarc rule(s) are not implemented yet and were skipped",
            engine.rule_count(),
            engine.unsupported().len()
        );
    }

    let (files, filter) = select_files(&args.paths, &args.changed)?;
    if files.is_empty() {
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
            eprintln!("{}{}", summary_line(summary), filter_note(filter.as_ref()));
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

fn run_nullcheck(args: &cli::NullcheckArgs) -> Result<ExitCode> {
    let (files, filter) = select_files(&args.paths, &args.changed)?;
    if files.is_empty() {
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
    let mode = match args.mode {
        cli::NullnessModeArg::Conservative => groovylint_analysis::nullability::Mode::Conservative,
        cli::NullnessModeArg::Inferred => groovylint_analysis::nullability::Mode::Inferred,
        cli::NullnessModeArg::Strict => groovylint_analysis::nullability::Mode::Strict,
    };
    let options = groovylint_analysis::nullcheck::Options {
        initialization_check: args.initialization_check,
    };
    let reports = pool.install(|| {
        files
            .par_iter()
            .map(|path| {
                analyze_nullness_file(
                    path,
                    mode,
                    args.contracts.as_deref(),
                    options,
                    args.marked_only,
                )
            })
            .collect::<Result<Vec<_>>>()
    })?;

    let summary = summarize(&reports);
    match args.format {
        cli::OutputFormat::Text => {
            write_text(&reports);
            eprintln!("{}{}", summary_line(summary), filter_note(filter.as_ref()));
        }
        cli::OutputFormat::Json => print!("{}", write_json(&reports, summary)?),
    }
    if summary.total() > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Collects the files to analyze and, when a filter was requested, the summary
/// note that says how many files were checked out of how many.
fn select_files(
    paths: &[std::path::PathBuf],
    changed: &changed::ChangedArgs,
) -> Result<(Vec<std::path::PathBuf>, Option<String>)> {
    let files = discovery::collect_files(paths)?;
    let Some(set) = changed.resolve()? else {
        if files.is_empty() {
            eprintln!("no Groovy source files found");
        }
        return Ok((files, None));
    };

    let total = files.len();
    let source = if set.is_git_range() {
        "changed since"
    } else {
        "listed by"
    };
    let description = set.description().to_string();
    let selected = changed::filter(files, &set);
    let checked = selected.len();
    let note = format!("{checked} of {total} file(s) checked, {source} {description}");
    if checked == 0 {
        eprintln!("no Groovy source files to check ({note}); nothing to do");
    }
    Ok((selected, Some(note)))
}

fn filter_note(filter: Option<&String>) -> String {
    match filter {
        Some(note) => format!(" ({note})"),
        None => String::new(),
    }
}

fn package_defaults(path: &Path) -> groovylint_analysis::nullability::Defaults {
    let Some(directory) = path.parent() else {
        return groovylint_analysis::nullability::Defaults::default();
    };
    let info = directory.join("package-info.java");
    let Ok(text) = fs::read_to_string(info) else {
        return groovylint_analysis::nullability::Defaults::default();
    };
    if text.contains("NullMarked") {
        return groovylint_analysis::nullability::Defaults::marked();
    }
    groovylint_analysis::nullability::Defaults {
        params_nonnull: text.contains("ParametersAreNonnullByDefault")
            || text.contains("NonNullApi"),
        returns_nonnull: text.contains("ReturnValuesAreNonnullByDefault")
            || text.contains("NonNullApi"),
        fields_nonnull: false,
    }
}

fn analyze_nullness_file(
    path: &Path,
    mode: groovylint_analysis::nullability::Mode,
    contracts_path: Option<&Path>,
    options: groovylint_analysis::nullcheck::Options,
    marked_only: bool,
) -> Result<FileReport> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let source = SourceFile::new(path.to_path_buf(), text);
    let diagnostics = match parse(source.text()) {
        Ok(parsed) => {
            let defaults = package_defaults(path);
            if marked_only && !is_marked(&parsed.script, defaults) {
                return Ok(FileReport {
                    source,
                    diagnostics: Vec::new(),
                });
            }
            let contracts = groovylint_analysis::contracts::registry_with_defaults(
                &parsed.script,
                contracts_path,
                defaults,
            )
            .map_err(anyhow::Error::msg)?;
            groovylint_analysis::nullcheck::analyze_with_options(
                &parsed.script,
                mode,
                contracts,
                defaults,
                options,
            )
            .into_iter()
            .map(|finding| {
                let severity = if finding.code.starts_with("Possible") {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                Diagnostic::new(finding.code, severity, finding.message, finding.span)
            })
            .collect()
        }
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

/// Returns whether the script opts into nullness checking.
///
/// A script is marked when a class carries `@NullMarked`/`@NonNullApi`, or when
/// its package does through `package-info.java`.
fn is_marked(
    script: &groovylint_syntax::ast::Script,
    defaults: groovylint_analysis::nullability::Defaults,
) -> bool {
    if defaults.any() {
        return true;
    }
    script.items.iter().any(|item| match item {
        groovylint_syntax::ast::Item::Class(class) => class.annotations.iter().any(|annotation| {
            annotation == "NullMarked"
                || annotation.ends_with(".NullMarked")
                || annotation == "NonNullApi"
                || annotation.ends_with(".NonNullApi")
        }),
        _ => false,
    })
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
