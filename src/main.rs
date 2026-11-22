use cargo_c4doc::{
    Error, Result, check, drift, io_error,
    model::{self, Model},
    output, render, rust_analysis, source,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    version,
    about = "Evidence-backed C4 documentation for coding agents",
    long_about = "Collect source evidence, compare observations, validate an agent-authored C4 model, and generate Markdown + Mermaid. No LLM calls or repository code execution.\n\nExit codes: 0 success, 1 invalid input/incomplete analysis/stale output/review gate/I/O failure, 2 invalid command usage."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze Cargo targets and reachable Rust modules without building the project.
    Analyze {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Select one workspace package by name.
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// List source files with line counts and SHA-256; never emit file contents.
    Scan {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "5000", value_parser = clap::value_parser!(u32).range(1..))]
        max_files: u32,
        #[arg(long, default_value = "2097152", value_parser = clap::value_parser!(u64).range(1..=2097152))]
        max_bytes: u64,
        /// Create a JSON file without overwriting; otherwise print to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Compare saved analyze results for changes requiring architecture review.
    Drift {
        /// Previous analyze JSON, collected from the same root and package selection.
        before: PathBuf,
        /// Current analyze JSON; this command does not run analysis or change sources.
        after: PathBuf,
        /// Annotate changed paths with cited claims and mapped components from this model.
        #[arg(long)]
        model: Option<PathBuf>,
        /// Previous scan JSON, collected from analyze's source_root.
        #[arg(long, requires = "after_scan")]
        before_scan: Option<PathBuf>,
        /// Current scan JSON, with the same root and limits as the previous scan.
        #[arg(long, requires = "before_scan")]
        after_scan: Option<PathBuf>,
        /// Exit 1 when changes require review; incomplete comparisons always exit 1.
        #[arg(long)]
        check: bool,
        /// Create a JSON report without overwriting; otherwise print to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Create a review-required draft; never infer architecture from directory names.
    Init {
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "c4.json")]
        output: PathBuf,
    },
    /// Produce a source citation to attach to an observed claim.
    Cite {
        path: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u64).range(1..))]
        start: u64,
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        end: Option<u64>,
    },
    /// Print the JSON Schema generated from the model's Rust types.
    Schema,
    /// Check C4 scopes, references, source ranges, and source digests.
    Check {
        model: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Treat inference, open questions, and other warnings as failures.
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        json: bool,
    },
    /// Generate all views after validation; protect existing hand-edited files.
    Render {
        model: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "docs/c4")]
        output: PathBuf,
        /// Check for stale documents without writing anything (for CI).
        #[arg(long)]
        check: bool,
        #[arg(long)]
        strict: bool,
    },
}

fn json<T: Serialize>(value: &T, path: Option<&Path>) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    match path {
        Some(path) => output::write_new(path, &bytes),
        None => io::stdout()
            .lock()
            .write_all(&bytes)
            .map_err(|e| io_error("stdout", e)),
    }
}

fn load(path: &Path) -> Result<Model> {
    Ok(serde_json::from_slice(&source::read_bounded(
        path,
        8 * 1024 * 1024,
    )?)?)
}

fn run(command: Command) -> Result<bool> {
    match command {
        Command::Analyze {
            path,
            package,
            output,
        } => {
            let analysis = rust_analysis::analyze(&path, package.as_deref())?;
            json(&analysis, output.as_deref())?;
            return Ok(analysis.complete);
        }
        Command::Scan {
            root,
            max_files,
            max_bytes,
            output,
        } => {
            json(
                &source::scan(&root, max_files as usize, max_bytes)?,
                output.as_deref(),
            )?;
        }
        Command::Init { title, output } => {
            if title.trim().is_empty() {
                return Err(Error::Invalid("title must not be blank".into()));
            }
            json(&model::starter(title), Some(&output))?;
            eprintln!(
                "Created {}. Research the repository and replace the draft before publishing.",
                output.display()
            );
        }
        Command::Drift {
            before,
            after,
            model,
            before_scan,
            after_scan,
            check,
            output,
        } => {
            let report = drift::compare_files(
                &before,
                &after,
                model.as_deref(),
                before_scan.as_deref().zip(after_scan.as_deref()),
            )?;
            json(&report, output.as_deref())?;
            return Ok(report.complete && (!check || !report.review_required));
        }
        Command::Cite {
            path,
            root,
            start,
            end,
        } => json(&source::cite(&root, path, start, end)?, None)?,
        Command::Schema => json(&schemars::schema_for!(Model), None)?,
        Command::Check {
            model,
            root,
            strict,
            json: machine,
        } => {
            let report = match load(&model) {
                Ok(model) => check::validate(&model, &root),
                Err(error) => check::Report {
                    errors: vec![check::Diagnostic {
                        location: model.display().to_string(),
                        message: error.to_string(),
                    }],
                    warnings: vec![],
                },
            };
            if machine {
                json(&report, None)?;
            } else {
                for (severity, diagnostics) in
                    [("error", &report.errors), ("warning", &report.warnings)]
                {
                    for diagnostic in diagnostics {
                        eprintln!(
                            "{severity}: {}: {}",
                            diagnostic.location, diagnostic.message
                        );
                    }
                }
                eprintln!(
                    "check: {} ({} errors, {} warnings)",
                    if report.passes(strict) {
                        "PASS"
                    } else {
                        "FAIL"
                    },
                    report.errors.len(),
                    report.warnings.len()
                );
            }
            return Ok(report.passes(strict));
        }
        Command::Render {
            model,
            root,
            output,
            check,
            strict,
        } => {
            let root = dunce::canonicalize(&root).map_err(|e| io_error(&root, e))?;
            let output = output::destination(&output)?;
            let model_path = dunce::canonicalize(&model).map_err(|e| io_error(&model, e))?;
            if model_path.starts_with(&output) {
                return Err(Error::Invalid(
                    "input model must be outside the generated document directory".into(),
                ));
            }
            let model = load(&model_path)?;
            // Regeneration must never overwrite a file cited by its own input model.
            let claim_sources = model
                .elements
                .iter()
                .map(|e| &e.evidence)
                .chain(model.relationships.iter().map(|r| &r.evidence))
                .flat_map(model::Evidence::sources);
            let mapped_sources = model
                .elements
                .iter()
                .flat_map(|e| e.mapped_entries())
                .map(|entry| &entry.source);
            for citation in claim_sources.chain(mapped_sources) {
                if root.join(&citation.path).starts_with(&output) {
                    return Err(Error::Invalid(
                        "source citations must be outside the generated document directory".into(),
                    ));
                }
            }
            let documents = render::documents(&model, &root, &output, strict)?;
            output::publish(&output, &documents, check)?;
            eprintln!(
                "{}: {} views, {} files in {}",
                if check { "up to date" } else { "rendered" },
                model.views.len(),
                documents.len(),
                output.display()
            );
        }
    }
    Ok(true)
}

fn main() -> ExitCode {
    let mut arguments: Vec<_> = std::env::args_os().collect();
    if arguments.get(1).is_some_and(|argument| argument == "c4doc") {
        arguments.remove(1);
    }
    match run(Cli::parse_from(arguments).command) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::BrokenPipe => {
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cargo-c4doc: {error}");
            ExitCode::FAILURE
        }
    }
}
