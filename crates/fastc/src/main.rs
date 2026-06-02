use clap::{Parser, Subcommand, ValueEnum};
use miette::{IntoDiagnostic, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fastc")]
#[command(about = "FastC transpiler - compile FastC to C11", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Safety level for Power of 10 rule enforcement
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum CliSafetyLevel {
    /// Standard FastC safety (default)
    #[default]
    Standard,
    /// Full Power of 10 compliance for safety-critical code
    Critical,
    /// Relaxed mode for prototyping
    Relaxed,
}

impl From<CliSafetyLevel> for fastc::SafetyLevel {
    fn from(level: CliSafetyLevel) -> Self {
        match level {
            CliSafetyLevel::Standard => fastc::SafetyLevel::Standard,
            CliSafetyLevel::Critical => fastc::SafetyLevel::SafetyCritical,
            CliSafetyLevel::Relaxed => fastc::SafetyLevel::Relaxed,
        }
    }
}

/// Project type for scaffolding
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliProjectType {
    /// A binary application
    Binary,
    /// A library
    Library,
    /// An FFI wrapper library
    FfiWrapper,
}

impl From<CliProjectType> for fastc::ProjectType {
    fn from(t: CliProjectType) -> Self {
        match t {
            CliProjectType::Binary => fastc::ProjectType::Binary,
            CliProjectType::Library => fastc::ProjectType::Library,
            CliProjectType::FfiWrapper => fastc::ProjectType::FfiWrapper,
        }
    }
}

/// Build system template
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliBuildTemplate {
    /// GNU Make
    Make,
    /// CMake
    Cmake,
    /// Meson
    Meson,
}

impl From<CliBuildTemplate> for fastc::BuildTemplate {
    fn from(t: CliBuildTemplate) -> Self {
        match t {
            CliBuildTemplate::Make => fastc::BuildTemplate::Make,
            CliBuildTemplate::Cmake => fastc::BuildTemplate::CMake,
            CliBuildTemplate::Meson => fastc::BuildTemplate::Meson,
        }
    }
}

/// Output format for `fastc check`. Agents want JSON; humans want
/// the existing "No errors found." text. Default stays text to keep
/// the inner loop unchanged.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum CliCheckFormat {
    #[default]
    Text,
    Json,
}

/// Report output format
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum CliReportFormat {
    /// Pretty-printed JSON (for AI agents)
    #[default]
    Json,
    /// Compact JSON (for CI/CD)
    Compact,
    /// Human-readable text
    Text,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a FastC source file to C
    Compile {
        /// Input FastC source file
        input: PathBuf,

        /// Output C file (use - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,

        /// Also emit a C header file
        #[arg(long)]
        emit_header: bool,

        /// Enable Power of 10 safety-critical rules (enabled by default)
        #[arg(long, hide = true)]
        p10: bool,

        /// Safety level: standard (default), critical (strictest), relaxed (no P10 checks)
        #[arg(long, value_enum, default_value = "standard")]
        safety_level: CliSafetyLevel,

        /// Treat all warnings as errors (strict mode)
        #[arg(long)]
        strict: bool,

        /// Emit per-pass timing JSON when compilation finishes
        #[arg(long)]
        timing: bool,

        /// Path to write the timing JSON (default: stderr)
        #[arg(long, value_name = "PATH", requires = "timing")]
        timing_output: Option<PathBuf>,

        /// Stage 2.1: opt into SMT contract discharge. Tier-1
        /// syntactic discharge always runs; this flag also enables
        /// the tier-2 SMT pipeline (shells out to `z3`). Off by
        /// default to keep `fastc compile` fast; `fastc build` will
        /// flip this on once Z3 is bundled with releases.
        #[arg(long, conflicts_with = "no_prove")]
        prove: bool,

        /// Disable contract discharge entirely (skip both tier-1
        /// and tier-2). Every obligation falls to the runtime trap.
        /// Useful for benchmarking the unproven cost.
        #[arg(long)]
        no_prove: bool,

        /// Per-obligation SMT budget in milliseconds.
        /// Default: 500 ms.
        #[arg(long, value_name = "MS", default_value = "500")]
        prove_budget: u64,

        /// Write the discharge report JSON to this path. Omit to
        /// suppress the report. Use `-` to write to stderr.
        #[arg(long, value_name = "PATH")]
        discharge_output: Option<String>,

        /// Write the per-build `caps.json` capability surface to
        /// this path. Documents every function's declared cap
        /// parameters — the agent-facing answer to "what can this
        /// program structurally do?". `-` writes to stderr.
        #[arg(long, value_name = "PATH")]
        caps_output: Option<String>,

        /// Produce path-independent output. Equivalent to passing
        /// the input as a basename only — `#line` directives embed
        /// `"foo.fc"` instead of the absolute path, so compiling
        /// the same source in different working directories
        /// produces byte-identical C output. Use this for
        /// reproducible-build verification and content-hash-based
        /// caches that key off the C output across machines.
        #[arg(long)]
        reproducible: bool,
    },

    /// Type-check a FastC source file without emitting C
    Check {
        /// Input FastC source file
        input: PathBuf,

        /// Enable Power of 10 safety-critical rules (enabled by default)
        #[arg(long, hide = true)]
        p10: bool,

        /// Safety level: standard (default), critical (strictest), relaxed (no P10 checks)
        #[arg(long, value_enum, default_value = "standard")]
        safety_level: CliSafetyLevel,

        /// Treat all warnings as errors (strict mode)
        #[arg(long)]
        strict: bool,

        /// Emit per-pass timing JSON when compilation finishes
        #[arg(long)]
        timing: bool,

        /// Path to write the timing JSON (default: stderr)
        #[arg(long, value_name = "PATH", requires = "timing")]
        timing_output: Option<PathBuf>,

        /// Emit a machine-readable JSON status object on success
        /// (`{"status": "ok", "file": "...", "safety_level": "..."}`).
        /// On failure, the standard miette diagnostic stream still
        /// goes to stderr — agents read exit code to disambiguate.
        #[arg(long, value_enum, default_value = "text")]
        output_format: CliCheckFormat,
    },

    /// List Power of 10 rules and their status
    P10Rules {
        /// Safety level to show rules for
        #[arg(long, value_enum, default_value = "critical")]
        safety_level: CliSafetyLevel,
    },

    /// Generate compliance certification report (for AI agents and audits)
    CertReport {
        /// Input FastC source file(s)
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Output format: json (default), text, or compact
        #[arg(long, value_enum, default_value = "json")]
        format: CliReportFormat,

        /// Output file (use - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,

        /// Safety level for checking
        #[arg(long, value_enum, default_value = "standard")]
        safety_level: CliSafetyLevel,

        /// Generate project-wide report (aggregates all files)
        #[arg(long)]
        project: bool,

        /// Project name for project report
        #[arg(long)]
        project_name: Option<String>,

        /// Fail with exit code 1 if non-compliant
        #[arg(long)]
        fail_on_violation: bool,
    },

    /// Format a FastC source file
    Fmt {
        /// Input FastC source file
        input: PathBuf,

        /// Output file (use - for stdout, omit to format in place)
        #[arg(short, long)]
        output: Option<String>,

        /// Check if the file is already formatted (exit with error if not)
        #[arg(long)]
        check: bool,
    },

    /// Create a new FastC project
    New {
        /// Project name
        name: String,

        /// Project type
        #[arg(long, short = 't', value_enum, default_value = "binary")]
        r#type: CliProjectType,

        /// Build system template
        #[arg(long, value_enum, default_value = "make")]
        template: CliBuildTemplate,
    },

    /// Initialize a FastC project in the current directory
    Init {
        /// Directory to initialize (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Project type
        #[arg(long, short = 't', value_enum, default_value = "binary")]
        r#type: CliProjectType,

        /// Build system template
        #[arg(long, value_enum, default_value = "make")]
        template: CliBuildTemplate,
    },

    /// Build the project using fastc.toml configuration
    Build {
        /// Build in release mode (optimizations enabled)
        #[arg(long, conflicts_with = "dev")]
        release: bool,

        /// Build in dev mode (tcc backend when available, no optimization)
        #[arg(long)]
        dev: bool,

        /// Output directory for generated C files
        #[arg(short, long, default_value = "build")]
        output: PathBuf,

        /// Compile the generated C code with a C compiler
        #[arg(long)]
        cc: bool,

        /// C compiler to use. Default: tcc when --dev and tcc is on PATH, else cc.
        #[arg(long)]
        compiler: Option<String>,

        /// Additional flags to pass to the C compiler
        #[arg(long)]
        cflags: Option<String>,

        /// Cross-compile target triple (e.g. aarch64-linux-musl, wasm32-wasi).
        /// Run `fastc target list` to see the full set. Backed by `zig cc` by
        /// default; override with --cc-override for a custom toolchain.
        #[arg(long, value_name = "TRIPLE")]
        target: Option<String>,

        /// Override the C compiler with a custom binary (proprietary
        /// cross-toolchain, distro gcc-cross, etc). Bypasses zig cc.
        #[arg(long, value_name = "PATH", conflicts_with = "compiler")]
        cc_override: Option<String>,
    },

    /// Build, compile, and run the project
    Run {
        /// Build in release mode (optimizations enabled)
        #[arg(long, conflicts_with = "dev")]
        release: bool,

        /// Build in dev mode (tcc backend when available, no optimization)
        #[arg(long)]
        dev: bool,

        /// C compiler to use. Default: tcc when --dev and tcc is on PATH, else cc.
        #[arg(long)]
        compiler: Option<String>,

        /// Additional flags to pass to the C compiler
        #[arg(long)]
        cflags: Option<String>,

        /// Cross-compile target triple (e.g. aarch64-linux-musl, wasm32-wasi).
        /// `fastc run` will refuse to execute non-native binaries.
        #[arg(long, value_name = "TRIPLE")]
        target: Option<String>,

        /// Override the C compiler with a custom binary.
        #[arg(long, value_name = "PATH", conflicts_with = "compiler")]
        cc_override: Option<String>,

        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Inspect cross-compile targets and verify backend availability.
    Target {
        #[command(subcommand)]
        action: TargetAction,
    },

    /// Fetch project dependencies without building
    Fetch,

    /// Compute and record the content sha256 of every dependency in
    /// `fastc.lock`. Use this after adding a new dep (or after a
    /// rev/tag bump) to anchor the verifier against the current
    /// fetched tree. Subsequent `fastc build` runs will refuse to
    /// proceed if a dep no longer hashes to the recorded value.
    Lock {
        /// Re-hash dependencies even when their recorded sha256
        /// still matches. Use this when an upstream rev was rewritten
        /// (force-push) and you've decided the new contents are safe.
        #[arg(long)]
        force: bool,
    },

    /// Fetch a candidate dependency, surface what capabilities it
    /// asks for, and (with confirmation) write it into `fastc.toml`.
    /// This is the supply-chain front door: a fastC dep can declare
    /// `ref(CapFsRead)` / `ref(CapNetConnect)` / etc. in its public
    /// surface — `fastc add` extracts that set and shows it to the
    /// user *before* the dep ever runs in a build.
    Add {
        /// Git URL to add (e.g. `https://github.com/Skelf-Research/fastc-http`).
        url: String,
        /// Pin to a specific git commit. If omitted, `fastc add` uses
        /// the resolved HEAD of the default branch and records it.
        #[arg(long)]
        rev: Option<String>,
        /// Name to use for the dep entry in fastc.toml. Defaults to
        /// the dep's own `[package].name` value.
        #[arg(long)]
        name: Option<String>,
        /// Skip the interactive confirmation prompt. Useful for CI
        /// and scripted setups; do not use as a default.
        #[arg(long)]
        yes: bool,
    },

    /// Run the compile-time budget benchmark and report results
    Bench {
        /// Path to the budget TOML (default: auto-discovered)
        #[arg(long)]
        budget: Option<PathBuf>,

        /// Fail with exit code 1 if any benchmark is over budget
        #[arg(long)]
        fail_on_regression: bool,

        /// Only run the named benchmark (matches a key under [budgets.*])
        #[arg(long)]
        only: Option<String>,
    },

    /// Emit a machine-readable JSON summary of every fn in a source
    /// file — name, params, return type, annotations, requires
    /// clauses. The Stage 1.6 agent-facing artifact, designed for
    /// Claude Code / Cursor / Codex consumption without re-parsing
    /// the source.
    Explain {
        /// Input FastC source file
        input: PathBuf,
    },

    /// Dump the project's pub type surface for AI context windows.
    /// Walks every `pub` item across the source tree and emits a
    /// markdown (default) or JSON summary. No bodies — agents get
    /// the signature surface only, optimized for token efficiency.
    Context {
        /// Input FastC source file (or project root)
        input: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value = "markdown")]
        format: ContextFormat,
        /// Restrict to a single module path (e.g., `vec`, `cli`)
        #[arg(long, value_name = "NAME")]
        module: Option<String>,
    },

    /// Semantic diff between two fastC sources (or two project roots).
    /// Reports added / removed pub items, signature changes,
    /// annotation changes, and module-header changes. Bodies are
    /// suppressed unless `--include-bodies` is passed.
    Diff {
        /// Old source
        old: PathBuf,
        /// New source
        new: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value = "markdown")]
        format: DiffFormat,
        /// Include text diffs of function bodies (off by default —
        /// only signature / annotation / header changes are shown)
        #[arg(long)]
        include_bodies: bool,
    },

    /// Run an MCP (Model Context Protocol) stdio server. Reads
    /// newline-delimited JSON-RPC 2.0 requests from stdin, writes
    /// responses to stdout. Implements `initialize`, `tools/list`,
    /// and `tools/call`; one tool is exposed today (`explain`),
    /// which returns the same JSON `fastc explain` prints.
    Mcp {},
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ContextFormat {
    Markdown,
    Json,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum DiffFormat {
    Markdown,
    Json,
}

#[derive(Subcommand)]
enum TargetAction {
    /// List all cross-compile target triples fastC ships presets for.
    List,
    /// Verify that the backend (zig cc by default, or --cc-override) can
    /// produce a binary for `triple`. Exits 0 on success, 1 on failure.
    /// Used by CI matrices to skip targets the runner doesn't support.
    Check {
        /// Target triple (e.g. aarch64-linux-musl)
        triple: String,
        /// Override the C compiler with a custom binary.
        #[arg(long, value_name = "PATH")]
        cc_override: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            emit_header,
            p10: _,
            safety_level,
            strict,
            timing,
            timing_output,
            prove,
            no_prove,
            prove_budget,
            discharge_output,
            caps_output,
            reproducible,
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            // L2: in --reproducible mode, label the source with just
            // its basename so `#line` directives don't bake the
            // absolute path into the C output. The full path is only
            // used for `read_to_string` above (to actually find the
            // file); everything downstream — diagnostics, source
            // maps, build-cache keys — sees the normalized name and
            // therefore produces byte-identical bytes regardless of
            // the user's working directory.
            let filename = if reproducible {
                input
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("source.fc")
                    .to_string()
            } else {
                input.display().to_string()
            };

            // P10 rules are always enabled (use --safety-level=relaxed to disable)
            let mut config = fastc::P10Config::from_level(safety_level.into());
            if strict {
                config.strict_mode = true;
            }

            if timing {
                fastc::timing::install(&filename);
            }

            // --prove enables the SMT tier (tier-1 syntactic discharge
            // always runs unless --no-prove forces every obligation to
            // runtime). --no-prove wins if both somehow set. H2: on-disk
            // discharge cache is rooted at the input file's directory
            // so repeat `fastc compile --prove` runs reuse Z3 verdicts.
            let cache_root = input
                .parent()
                .map(|p| p.to_path_buf())
                .filter(|p| !p.as_os_str().is_empty());
            let discharge_cfg = fastc::discharge::DischargeConfig {
                enable: prove && !no_prove,
                smt_budget_ms: prove_budget,
                cache_root,
            };

            // H4 — global build cache. Skip the lex→…→emit pipeline
            // entirely when this exact (source, version, safety, target,
            // header, strict) tuple has been compiled before. Only
            // active when neither --prove nor --no-prove was passed
            // (so the discharge report — which we'd otherwise lose —
            // isn't silently skipped), and when the header isn't
            // needed (the cache stores the C output only). For
            // common edit-loop usage of `fastc compile foo.fc`, this
            // shaves the per-pass time down to a single read.
            let level_label = match safety_level {
                CliSafetyLevel::Standard => "standard",
                CliSafetyLevel::Critical => "critical",
                CliSafetyLevel::Relaxed => "relaxed",
            };
            let build_key = fastc::build_cache::CacheKey {
                source: &source,
                fastc_version: env!("CARGO_PKG_VERSION"),
                safety_level: level_label,
                target_triple: None,
                emit_header,
                strict,
            };
            let cache_eligible = !prove && !no_prove && !emit_header && !timing;

            let (c_code, header, discharge_report) =
                if cache_eligible && let Some(cached) = fastc::build_cache::lookup(&build_key) {
                    (cached, None, fastc::discharge::DischargeReport::default())
                } else {
                    let result = fastc::compile_with_p10_and_discharge(
                        &source,
                        &filename,
                        emit_header,
                        config,
                        &discharge_cfg,
                    )?;
                    if cache_eligible {
                        fastc::build_cache::store(&build_key, &result.0);
                    }
                    result
                };

            if timing {
                emit_timing(timing_output.as_deref())?;
            }

            if let Some(path) = &discharge_output {
                let json = discharge_report.to_json();
                if path == "-" {
                    eprintln!("{}", json);
                } else {
                    std::fs::write(path, json).into_diagnostic()?;
                    eprintln!("Discharge report: {}", path);
                }
            }

            if let Some(path) = &caps_output {
                let ast = fastc::parse(&source, &filename)?;
                let json = fastc::caps_summary::CapsSummary::from_file(&ast).to_json();
                if path == "-" {
                    eprintln!("{}", json);
                } else {
                    std::fs::write(path, json).into_diagnostic()?;
                    eprintln!("Caps report: {}", path);
                }
            }

            if output == "-" {
                println!("{}", c_code);
                if let Some(h) = header {
                    eprintln!("\n--- Header ---\n{}", h);
                }
            } else {
                std::fs::write(&output, &c_code).into_diagnostic()?;

                if let Some(h) = header {
                    let header_path = output.replace(".c", ".h");
                    std::fs::write(&header_path, &h).into_diagnostic()?;
                }
            }
        }

        Commands::Check {
            input,
            p10: _,
            safety_level,
            strict,
            timing,
            timing_output,
            output_format,
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            // P10 rules are always enabled (use --safety-level=relaxed to disable)
            let mut config = fastc::P10Config::from_level(safety_level.into());
            if strict {
                config.strict_mode = true;
            }

            if timing {
                fastc::timing::install(&filename);
            }

            fastc::check_with_p10(&source, &filename, config)?;

            if timing {
                emit_timing(timing_output.as_deref())?;
            }

            match output_format {
                CliCheckFormat::Text => {
                    eprintln!("No errors found.");
                }
                CliCheckFormat::Json => {
                    // Print a stable JSON status line to stdout so agents
                    // can grep for `"status": "ok"`. Hand-rolled to
                    // avoid pulling serde_json into the hot path.
                    let level_label = match safety_level {
                        CliSafetyLevel::Standard => "standard",
                        CliSafetyLevel::Critical => "critical",
                        CliSafetyLevel::Relaxed => "relaxed",
                    };
                    println!(
                        "{{\"status\": \"ok\", \"file\": \"{}\", \"safety_level\": \"{}\", \"strict\": {}}}",
                        escape_json(&filename),
                        level_label,
                        strict
                    );
                }
            }
        }

        Commands::P10Rules { safety_level } => {
            let config = fastc::P10Config::from_level(safety_level.into());
            let checker = fastc::P10Checker::new(config);
            checker.print_rules_summary();
        }

        Commands::CertReport {
            inputs,
            format,
            output,
            safety_level,
            project,
            project_name,
            fail_on_violation,
        } => {
            let config = fastc::P10Config::from_level(safety_level.into());
            let checker = fastc::P10Checker::new(config.clone());

            let mut file_reports = Vec::new();
            let mut any_non_compliant = false;

            for input in &inputs {
                let source = std::fs::read_to_string(input).into_diagnostic()?;
                let filename = input.display().to_string();

                // Parse the file to count functions and check for violations
                let ast = match fastc::parse(&source, &filename) {
                    Ok(ast) => ast,
                    Err(e) => {
                        eprintln!("Parse error in {}: {:?}", filename, e);
                        continue;
                    }
                };

                let function_count = ast
                    .items
                    .iter()
                    .filter(|item| matches!(item, fastc::Item::Fn(_)))
                    .count();
                let violations = checker.check(&ast, &source);

                let report = fastc::ComplianceReport::new(
                    &filename,
                    &config,
                    &violations,
                    &source,
                    function_count,
                );

                if !report.is_compliant() {
                    any_non_compliant = true;
                }

                file_reports.push(report);
            }

            // Generate output
            let output_text = if project {
                let project_report = fastc::ProjectReport::from_files(
                    project_name,
                    safety_level.into(),
                    file_reports,
                );
                match format {
                    CliReportFormat::Json => project_report.to_json(),
                    CliReportFormat::Compact => {
                        serde_json::to_string(&project_report).unwrap_or_default()
                    }
                    CliReportFormat::Text => {
                        // For project reports in text, concatenate individual reports
                        let mut text = String::new();
                        text.push_str(&format!(
                            "Project: {}\n",
                            project_report.project_name.as_deref().unwrap_or("unnamed")
                        ));
                        text.push_str(&format!("Status: {:?}\n", project_report.status));
                        text.push_str(&format!(
                            "Files: {} analyzed, {} compliant\n\n",
                            project_report.summary.files_analyzed,
                            project_report.summary.files_compliant
                        ));
                        text
                    }
                }
            } else if file_reports.len() == 1 {
                let report = &file_reports[0];
                match format {
                    CliReportFormat::Json => report.to_json(),
                    CliReportFormat::Compact => report.to_json_compact(),
                    CliReportFormat::Text => report.to_text(),
                }
            } else {
                // Multiple files without project flag - output as JSON array
                match format {
                    CliReportFormat::Json => {
                        serde_json::to_string_pretty(&file_reports).unwrap_or_default()
                    }
                    CliReportFormat::Compact => {
                        serde_json::to_string(&file_reports).unwrap_or_default()
                    }
                    CliReportFormat::Text => file_reports
                        .iter()
                        .map(|r| r.to_text())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                }
            };

            if output == "-" {
                println!("{}", output_text);
            } else {
                std::fs::write(&output, &output_text).into_diagnostic()?;
                eprintln!("Report written to {}", output);
            }

            if fail_on_violation && any_non_compliant {
                std::process::exit(1);
            }
        }

        Commands::Fmt {
            input,
            output,
            check,
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            if check {
                // Check mode: verify already formatted
                if fastc::check_formatted(&source, &filename)? {
                    eprintln!("File is already formatted.");
                } else {
                    eprintln!(
                        "File is not formatted. Run `fastc fmt {}` to format.",
                        input.display()
                    );
                    std::process::exit(1);
                }
            } else {
                // Format mode
                let formatted = fastc::format(&source, &filename)?;

                match output.as_deref() {
                    Some("-") => {
                        print!("{}", formatted);
                    }
                    Some(path) => {
                        std::fs::write(path, &formatted).into_diagnostic()?;
                    }
                    None => {
                        // In-place formatting
                        std::fs::write(&input, &formatted).into_diagnostic()?;
                        eprintln!("Formatted {}.", input.display());
                    }
                }
            }
        }

        Commands::New {
            name,
            r#type,
            template,
        } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            fastc::create_project(&name, &current_dir, r#type.into(), template.into())?;
        }

        Commands::Init {
            path,
            r#type,
            template,
        } => {
            let path = if path.is_absolute() {
                path
            } else {
                std::env::current_dir().into_diagnostic()?.join(path)
            };
            fastc::init_project(&path, r#type.into(), template.into())?;
        }

        Commands::Build {
            release,
            dev,
            output,
            cc,
            compiler,
            cflags,
            target,
            cc_override,
        } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            // N3: workspace builds iterate members in declaration
            // order, each member building under its own M1 project
            // cache. cc-linking against a workspace root isn't
            // wired yet (you'd typically link members individually
            // or roll your own driver), so --target / --cc-override
            // are rejected when the root is a workspace.
            if ctx.is_workspace_root() {
                if cc || target.is_some() || cc_override.is_some() {
                    return Err(miette::miette!(
                        "--cc / --target / --cc-override on a `[workspace]` root \
                        isn't supported in v1.0 — build individual members instead, \
                        or open an issue with your use case"
                    ));
                }
                let c_files = ctx
                    .compile_workspace(&output, release)
                    .map_err(|e| miette::miette!("{}", e))?;
                eprintln!("Workspace build complete ({} members).", c_files.len());
                return Ok(());
            }

            let c_file = ctx
                .compile(&output, release)
                .map_err(|e| miette::miette!("{}", e))?;

            // Anything that uses --target / --cc-override implies cc.
            let needs_cc = cc || target.is_some() || cc_override.is_some();
            if needs_cc {
                let target_enum = parse_target_flag(target.as_deref())?;
                let cflags_vec: Vec<&str> = cflags
                    .as_deref()
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                let plan = plan_cc_invocation(
                    compiler.as_deref(),
                    cc_override.as_deref(),
                    target_enum,
                    dev,
                    release,
                )?;
                let prefix_refs: Vec<&str> = plan.prefix_args.iter().map(|s| s.as_str()).collect();
                let output_ext = target_enum.map(|t| t.output_extension()).unwrap_or("");
                ctx.cc_compile(
                    &c_file,
                    &plan.command,
                    &prefix_refs,
                    &cflags_vec,
                    release,
                    output_ext,
                )
                .map_err(|e| miette::miette!("{}", e))?;
            }
        }

        Commands::Run {
            release,
            dev,
            compiler,
            cflags,
            target,
            cc_override,
            args,
        } => {
            let target_enum = parse_target_flag(target.as_deref())?;
            if let Some(t) = target_enum {
                return Err(miette::miette!(
                    "`fastc run --target={}` is not supported: running a cross-compiled \
                    binary requires an emulator (qemu, wasmtime) we don't manage. \
                    Use `fastc build --target={}` to produce the binary, then run it \
                    yourself.",
                    t.triple(),
                    t.triple(),
                ));
            }
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            let output = PathBuf::from("build");
            let c_file = ctx
                .compile(&output, release)
                .map_err(|e| miette::miette!("{}", e))?;

            let cflags_vec: Vec<&str> = cflags
                .as_deref()
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            let plan = plan_cc_invocation(
                compiler.as_deref(),
                cc_override.as_deref(),
                None,
                dev,
                release,
            )?;
            let prefix_refs: Vec<&str> = plan.prefix_args.iter().map(|s| s.as_str()).collect();
            let executable = ctx
                .cc_compile(
                    &c_file,
                    &plan.command,
                    &prefix_refs,
                    &cflags_vec,
                    release,
                    "",
                )
                .map_err(|e| miette::miette!("{}", e))?;

            ctx.run(&executable, &args)
                .map_err(|e| miette::miette!("{}", e))?;
        }

        Commands::Target { action } => match action {
            TargetAction::List => {
                println!("| Triple | Use case |");
                println!("|---|---|");
                for t in fastc::targets::Target::all() {
                    println!("| {} | {} |", t.triple(), t.description());
                }
                println!();
                println!(
                    "Backed by `zig cc` by default. Pass `--cc-override=<path>` to use a \
                    proprietary cross-toolchain instead."
                );
            }
            TargetAction::Check {
                triple,
                cc_override,
            } => {
                let Some(t) = fastc::targets::Target::from_triple(&triple) else {
                    return Err(miette::miette!(
                        "unknown target `{}`. Run `fastc target list` to see supported triples.",
                        triple
                    ));
                };
                match fastc::targets::resolve_target_compiler(Some(t), cc_override.as_deref()) {
                    Ok(c) => {
                        println!(
                            "OK: target `{}` available via `{}{}`.",
                            t.triple(),
                            c.command,
                            if c.extra_args.is_empty() {
                                String::new()
                            } else {
                                format!(" {}", c.extra_args.join(" "))
                            }
                        );
                    }
                    Err(e) => {
                        eprintln!("ERROR: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },

        Commands::Fetch => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            eprintln!("Dependencies fetched successfully.");
        }

        Commands::Lock { force } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;
            ctx.lock_dependencies(force)
                .map_err(|e| miette::miette!("{}", e))?;
        }

        Commands::Add {
            url,
            rev,
            name,
            yes,
        } => {
            run_add(&url, rev.as_deref(), name.as_deref(), yes)?;
        }

        Commands::Bench {
            budget,
            fail_on_regression,
            only,
        } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let budget_path = match budget {
                Some(p) => p,
                None => fastc::bench::find_budget_toml(&current_dir).ok_or_else(|| {
                    miette::miette!(
                        "no compile-time-budget.toml found in {} or any parent",
                        current_dir.display()
                    )
                })?,
            };

            let project_root = budget_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| current_dir.clone());

            let mut config =
                fastc::bench::load_budget(&budget_path).map_err(|e| miette::miette!("{}", e))?;

            if let Some(only_name) = only {
                config.budgets.retain(|k, _| k == &only_name);
                if config.budgets.is_empty() {
                    return Err(miette::miette!("no benchmark matched --only={}", only_name));
                }
            }

            eprintln!(
                "Running {} benchmark(s) from {}",
                config.budgets.len(),
                budget_path.display()
            );

            let report = fastc::bench::run_all(&config, &project_root);

            // Markdown summary always to stderr.
            eprintln!("\n{}", report.to_markdown());

            // Always emit JSON to the configured path.
            if let Some(json_path) = &config.reporting.emit_json {
                let json_path = project_root.join(json_path);
                if let Some(parent) = json_path.parent() {
                    std::fs::create_dir_all(parent).into_diagnostic()?;
                }
                std::fs::write(&json_path, report.to_json()).into_diagnostic()?;
                eprintln!("JSON report: {}", json_path.display());
            }

            if let Some(md_path) = &config.reporting.emit_markdown {
                let md_path = project_root.join(md_path);
                if let Some(parent) = md_path.parent() {
                    std::fs::create_dir_all(parent).into_diagnostic()?;
                }
                std::fs::write(&md_path, report.to_markdown()).into_diagnostic()?;
                eprintln!("Markdown report: {}", md_path.display());
            }

            if fail_on_regression && report.overall_status == fastc::bench::BudgetStatus::Fail {
                std::process::exit(1);
            }
        }

        Commands::Explain { input } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();
            let file = fastc::parse(&source, &filename).map_err(|e| miette::miette!("{:?}", e))?;
            print_explain_json(&file);
        }

        Commands::Context {
            input,
            format,
            module,
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();
            let file = fastc::parse(&source, &filename).map_err(|e| miette::miette!("{:?}", e))?;
            match format {
                ContextFormat::Markdown => print_context_markdown(&file, module.as_deref()),
                ContextFormat::Json => print_context_json(&file, module.as_deref()),
            }
        }

        Commands::Diff {
            old,
            new,
            format,
            include_bodies,
        } => {
            let old_src = std::fs::read_to_string(&old).into_diagnostic()?;
            let new_src = std::fs::read_to_string(&new).into_diagnostic()?;
            let old_name = old.display().to_string();
            let new_name = new.display().to_string();
            let old_file =
                fastc::parse(&old_src, &old_name).map_err(|e| miette::miette!("{:?}", e))?;
            let new_file =
                fastc::parse(&new_src, &new_name).map_err(|e| miette::miette!("{:?}", e))?;
            match format {
                DiffFormat::Markdown => print_diff_markdown(&old_file, &new_file, include_bodies),
                DiffFormat::Json => print_diff_json(&old_file, &new_file, include_bodies),
            }
        }

        Commands::Mcp {} => {
            run_mcp_server();
        }
    }

    Ok(())
}

/// Minimal MCP (Model Context Protocol) stdio server. Reads
/// newline-delimited JSON-RPC 2.0 messages from stdin, writes
/// JSON-RPC responses to stdout. Implements three methods:
///
/// - `initialize`: returns server capabilities.
/// - `tools/list`: returns the available tools (one in v1: `explain`).
/// - `tools/call`: dispatches to the named tool.
///
/// Today the only tool is `explain(path: string)` which returns the
/// same JSON `fastc explain <path>` would print, wrapped in an
/// MCP `content` array. Future tools — `check`, `compile`,
/// `caps_summary`, `discharge_report` — bolt on by adding match
/// arms to `handle_tools_call`.
///
/// The framing is line-delimited JSON (one message per line)
/// rather than the LSP-style Content-Length headers that some MCP
/// clients use. Modern clients (Claude Code, Cursor) accept both;
/// line-delimited is simpler to implement without a third-party
/// MCP SDK dependency, and keeps the binary small.
fn run_mcp_server() {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let reader = stdin.lock();
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(req) => handle_mcp_request(req),
            Err(e) => mcp_error_response(
                serde_json::Value::Null,
                -32700,
                &format!("Parse error: {}", e),
            ),
        };
        let serialized = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        if writeln!(out, "{}", serialized).is_err() {
            break;
        }
        let _ = out.flush();
    }
}

fn handle_mcp_request(req: serde_json::Value) -> serde_json::Value {
    let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = req
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    match method.as_str() {
        "initialize" => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "fastc-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }),
        "tools/list" => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "explain",
                        "description": "Return a JSON summary of every fn in a fastC source file — name, params, return type, annotations, requires clauses, doc comments. The stable agent-facing artifact.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Filesystem path to a .fc source file."
                                }
                            },
                            "required": ["path"]
                        }
                    }
                ]
            }
        }),
        "tools/call" => handle_tools_call(req, id),
        _ => mcp_error_response(id, -32601, &format!("Method not found: {}", method)),
    }
}

fn handle_tools_call(req: serde_json::Value, id: serde_json::Value) -> serde_json::Value {
    let params = req
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match name.as_str() {
        "explain" => {
            let path = match args.get("path").and_then(|p| p.as_str()) {
                Some(p) => p.to_string(),
                None => {
                    return mcp_error_response(id, -32602, "Missing required 'path' argument");
                }
            };
            match std::fs::read_to_string(&path) {
                Ok(source) => match fastc::parse(&source, &path) {
                    Ok(file) => {
                        let json = explain_to_string(&file);
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [
                                    { "type": "text", "text": json }
                                ]
                            }
                        })
                    }
                    Err(e) => mcp_error_response(id, -32000, &format!("Parse failed: {:?}", e)),
                },
                Err(e) => {
                    mcp_error_response(id, -32000, &format!("Failed to read {}: {}", path, e))
                }
            }
        }
        _ => mcp_error_response(id, -32601, &format!("Unknown tool: {}", name)),
    }
}

fn mcp_error_response(id: serde_json::Value, code: i64, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Render the same JSON `print_explain_json` outputs, but return it
/// as a String instead of writing to stdout. Used by the MCP server
/// to embed the explain output as a tool result.
fn explain_to_string(file: &fastc::ast::File) -> String {
    let mut entries: Vec<String> = Vec::new();
    walk_for_explain(&file.items, None, &mut entries);
    let body = entries.join(",\n");
    format!(
        "{{\n  \"functions\": [\n{}\n  ]\n}}",
        if body.is_empty() { String::new() } else { body }
    )
}

/// Walk the parsed file and emit a JSON document describing every
/// top-level (and mod-nested) `fn` declaration. Format mirrors what
/// the future `fastc-mcp` server will surface as an MCP resource:
///
/// ```json
/// {
///   "functions": [
///     {
///       "name": "safe_div",
///       "module": null,
///       "params": [
///         { "name": "value",   "type": "i32" },
///         { "name": "divisor", "type": "i32" }
///       ],
///       "return": "i32",
///       "annotations": [],
///       "requires": ["(divisor != 0)", "(divisor > 0)"],
///       "is_unsafe": false,
///       "doc_comments": []
///     }
///   ]
/// }
/// ```
///
/// Hand-rolled because pulling in `serde_json` for one subcommand
/// would inflate compile time more than the savings buy us.
fn print_explain_json(file: &fastc::ast::File) {
    let mut entries: Vec<String> = Vec::new();
    walk_for_explain(&file.items, None, &mut entries);
    let mut module_entries: Vec<String> = Vec::new();
    walk_for_module_explain(&file.items, None, &mut module_entries);
    println!("{{");
    println!("  \"functions\": [");
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        println!("{}{}", e, comma);
    }
    println!("  ],");
    println!("  \"modules\": [");
    for (i, e) in module_entries.iter().enumerate() {
        let comma = if i + 1 < module_entries.len() {
            ","
        } else {
            ""
        };
        println!("{}{}", e, comma);
    }
    println!("  ]");
    println!("}}");
}

/// Walk every inline `mod` declaration and emit its header (when set)
/// as a JSON entry. Empty when no modules carry headers — agents
/// can use the presence of header info to detect v1.3-style sources.
fn walk_for_module_explain(
    items: &[fastc::ast::Item],
    parent: Option<&str>,
    out: &mut Vec<String>,
) {
    for item in items {
        if let fastc::ast::Item::Mod(m) = item {
            let path = match parent {
                Some(p) => format!("{}::{}", p, m.name),
                None => m.name.clone(),
            };
            if let Some(h) = &m.header {
                out.push(render_module_header_json(&path, h));
            }
            if let Some(body) = &m.body {
                walk_for_module_explain(body, Some(&path), out);
            }
        }
    }
}

fn render_module_header_json(path: &str, h: &fastc::ast::ModuleHeader) -> String {
    let owns = h
        .owns
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect::<Vec<_>>()
        .join(", ");
    let depends = h
        .depends
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect::<Vec<_>>()
        .join(", ");
    let invariants = h
        .invariants
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect::<Vec<_>>()
        .join(", ");
    let module_name = match &h.module_name {
        Some(n) => format!("\"{}\"", escape_json(n)),
        None => "null".to_string(),
    };
    let arch = match &h.arch {
        Some(a) => format!("\"{}\"", escape_json(a)),
        None => "null".to_string(),
    };
    let threading = match &h.threading {
        Some(t) => format!("\"{}\"", escape_json(t)),
        None => "null".to_string(),
    };
    format!(
        "    {{\n      \"path\": \"{}\",\n      \"module\": {},\n      \"owns\": [{}],\n      \"arch\": {},\n      \"depends\": [{}],\n      \"threading\": {},\n      \"invariants\": [{}]\n    }}",
        escape_json(path),
        module_name,
        owns,
        arch,
        depends,
        threading,
        invariants
    )
}

fn walk_for_explain(items: &[fastc::ast::Item], module: Option<&str>, out: &mut Vec<String>) {
    for item in items {
        match item {
            fastc::ast::Item::Fn(f) => {
                out.push(render_fn_explain(f, module));
            }
            fastc::ast::Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let nested = match module {
                        Some(parent) => format!("{}::{}", parent, m.name),
                        None => m.name.clone(),
                    };
                    walk_for_explain(body, Some(&nested), out);
                }
            }
            _ => {}
        }
    }
}

fn render_fn_explain(f: &fastc::ast::FnDecl, module: Option<&str>) -> String {
    let params = f
        .params
        .iter()
        .map(|p| {
            format!(
                "      {{ \"name\": \"{}\", \"type\": \"{}\" }}",
                escape_json(&p.name),
                escape_json(&type_to_string(&p.ty))
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let annotations = f
        .annotations
        .iter()
        .map(|a| format!("\"{}\"", escape_json(a)))
        .collect::<Vec<_>>()
        .join(", ");
    let requires = f
        .requires
        .iter()
        .map(|e| format!("\"{}\"", escape_json(&expr_to_string(e))))
        .collect::<Vec<_>>()
        .join(", ");
    let ensures = f
        .ensures
        .iter()
        .map(|e| format!("\"{}\"", escape_json(&expr_to_string(e))))
        .collect::<Vec<_>>()
        .join(", ");
    // Stage 1.4 / agent-facing surface: which capability tokens does
    // this fn accept? Scans `ref(Cap*)` / `mref(Cap*)` params in
    // declaration order with duplicates collapsed.
    let caps = collect_fn_caps(f)
        .iter()
        .map(|c| format!("\"{}\"", escape_json(c)))
        .collect::<Vec<_>>()
        .join(", ");
    let doc_comments = f
        .doc_comments
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect::<Vec<_>>()
        .join(", ");
    let module_field = match module {
        Some(m) => format!("\"{}\"", escape_json(m)),
        None => "null".to_string(),
    };
    // v1.3 structured annotations. Emitted as JSON-null when unset
    // so consumers can detect "not annotated" from "annotated false".
    let mem_field = match &f.mem {
        Some(m) => format!("\"arena={}\"", escape_json(&m.arena)),
        None => "null".to_string(),
    };
    let panics_field = match &f.panics {
        Some(fastc::ast::PanicsAnnot::Never) => "\"never\"".to_string(),
        Some(fastc::ast::PanicsAnnot::Always) => "\"always\"".to_string(),
        Some(fastc::ast::PanicsAnnot::On(e)) => {
            format!("\"on({})\"", escape_json(&expr_to_string(e)))
        }
        None => "null".to_string(),
    };
    let purity_field = match &f.purity {
        Some(fastc::ast::PurityLevel::Pure) => "\"pure\"".to_string(),
        Some(fastc::ast::PurityLevel::Effect) => "\"effect\"".to_string(),
        Some(fastc::ast::PurityLevel::Io) => "\"io\"".to_string(),
        None => "null".to_string(),
    };
    let complexity_field = match &f.complexity {
        Some(c) => format!("\"{}\"", bigo_to_string(c)),
        None => "null".to_string(),
    };
    format!(
        "    {{\n      \"name\": \"{}\",\n      \"module\": {},\n      \"params\": [{}\n      ],\n      \"return\": \"{}\",\n      \"annotations\": [{}],\n      \"caps\": [{}],\n      \"requires\": [{}],\n      \"ensures\": [{}],\n      \"mem\": {},\n      \"panics\": {},\n      \"purity\": {},\n      \"complexity\": {},\n      \"is_test\": {},\n      \"is_unsafe\": {},\n      \"doc_comments\": [{}]\n    }}",
        escape_json(&f.name),
        module_field,
        if f.params.is_empty() {
            "".to_string()
        } else {
            format!("\n{}", params)
        },
        escape_json(&type_to_string(&f.return_type)),
        annotations,
        caps,
        requires,
        ensures,
        mem_field,
        panics_field,
        purity_field,
        complexity_field,
        f.is_test,
        f.is_unsafe,
        doc_comments
    )
}

fn bigo_to_string(b: &fastc::ast::BigO) -> String {
    use fastc::ast::BigO;
    match b {
        BigO::Const => "O(1)".to_string(),
        BigO::N => "O(n)".to_string(),
        BigO::Log => "O(log n)".to_string(),
        BigO::NLogN => "O(n log n)".to_string(),
        BigO::NPow(k) => format!("O(n^{})", k),
        BigO::Exp => "O(2^n)".to_string(),
        BigO::Other(s) => format!("O({})", s),
    }
}

/// Walk a function's params and return the set of `Cap*` struct
/// names it accepts via `ref(...)` / `mref(...)` — the agent-facing
/// answer to "what permissions does this fn demand to call?".
fn collect_fn_caps(f: &fastc::ast::FnDecl) -> Vec<String> {
    use fastc::ast::TypeExpr;
    const KNOWN: &[&str] = &[
        "CapFsRead",
        "CapFsWrite",
        "CapNetConnect",
        "CapNetListen",
        "CapProcSpawn",
        "CapTimeRead",
        "CapRand",
        "CapEnvRead",
    ];
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for p in &f.params {
        let inner = match &p.ty {
            TypeExpr::Ref(t) | TypeExpr::Mref(t) => t.as_ref(),
            _ => continue,
        };
        let name = match inner {
            TypeExpr::Named(n) | TypeExpr::NamedGeneric(n, _) => n.clone(),
            _ => continue,
        };
        if KNOWN.iter().any(|k| *k == name.as_str()) && seen.insert(name.clone()) {
            out.push(name);
        }
    }
    out
}

fn type_to_string(ty: &fastc::ast::TypeExpr) -> String {
    use fastc::ast::TypeExpr;
    match ty {
        TypeExpr::Void => "void".to_string(),
        TypeExpr::Primitive(p) => format!("{:?}", p).to_lowercase(),
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::NamedGeneric(n, args) => format!(
            "{}[{}]",
            n,
            args.iter()
                .map(type_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeExpr::Ref(t) => format!("ref({})", type_to_string(t)),
        TypeExpr::Mref(t) => format!("mref({})", type_to_string(t)),
        TypeExpr::Raw(t) => format!("raw({})", type_to_string(t)),
        TypeExpr::Rawm(t) => format!("rawm({})", type_to_string(t)),
        TypeExpr::Own(t) => format!("own({})", type_to_string(t)),
        TypeExpr::Slice(t) => format!("slice({})", type_to_string(t)),
        TypeExpr::Arr(t, _) => format!("arr({}, ..)", type_to_string(t)),
        TypeExpr::Opt(t) => format!("opt({})", type_to_string(t)),
        TypeExpr::Res(a, b) => format!("res({}, {})", type_to_string(a), type_to_string(b)),
        TypeExpr::Fn { params, ret, .. } => format!(
            "fn({}) -> {}",
            params
                .iter()
                .map(type_to_string)
                .collect::<Vec<_>>()
                .join(", "),
            type_to_string(ret)
        ),
    }
}

fn expr_to_string(e: &fastc::ast::Expr) -> String {
    // Best-effort textual rendering for the requires JSON. Round-
    // tripping through the formatter would give nicer output but
    // pulls in a lot of bytes; the explain JSON is for AI agents,
    // not human review, so a structural dump is fine.
    use fastc::ast::Expr;
    match e {
        Expr::IntLit { value, .. } => value.to_string(),
        Expr::FloatLit { raw, .. } => raw.clone(),
        Expr::BoolLit { value, .. } => value.to_string(),
        Expr::Ident { name, .. } => name.clone(),
        Expr::Binary { op, lhs, rhs, .. } => format!(
            "({} {} {})",
            expr_to_string(lhs),
            binop_to_string(*op),
            expr_to_string(rhs)
        ),
        Expr::Unary { op, operand, .. } => {
            let s = match op {
                fastc::ast::UnaryOp::Neg => "-",
                fastc::ast::UnaryOp::Not => "!",
                fastc::ast::UnaryOp::BitNot => "~",
            };
            format!("({}{})", s, expr_to_string(operand))
        }
        Expr::Paren { inner, .. } => format!("({})", expr_to_string(inner)),
        Expr::Call { callee, args, .. } => format!(
            "{}({})",
            expr_to_string(callee),
            args.iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Field { base, field, .. } => format!("{}.{}", expr_to_string(base), field),
        _ => "<expr>".to_string(),
    }
}

fn binop_to_string(op: fastc::ast::BinOp) -> &'static str {
    use fastc::ast::BinOp;
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Parse a `--target=<triple>` value into the enum, returning a helpful
/// error pointing the user at `fastc target list` for typos.
fn parse_target_flag(triple: Option<&str>) -> Result<Option<fastc::targets::Target>> {
    match triple {
        None => Ok(None),
        Some(t) => match fastc::targets::Target::from_triple(t) {
            Some(target) => Ok(Some(target)),
            None => Err(miette::miette!(
                "unknown target `{}`. Run `fastc target list` to see supported triples.",
                t
            )),
        },
    }
}

/// The resolved C compiler invocation plan: which binary to call and any
/// args that must appear before the source file (e.g. zig's `cc`
/// subcommand + `--target=...`).
struct CcPlan {
    command: String,
    prefix_args: Vec<String>,
}

/// Resolve `(--compiler, --cc-override, --target, --dev, --release)` into
/// a concrete compiler invocation plan.
///
/// Priority (highest first):
/// 1. `--cc-override=<path>` — explicit user-supplied binary. Wins over
///    everything, including --target (we trust the user knows their toolchain
///    targets the right triple).
/// 2. `--target=<triple>` — route through `zig cc --target=...`. Errors if
///    zig isn't on PATH.
/// 3. `--compiler=<path>` — legacy explicit compiler flag, no target.
/// 4. `--dev` (and not `--release`) — auto-detect tcc on PATH.
/// 5. Default — `cc`.
fn plan_cc_invocation(
    explicit_compiler: Option<&str>,
    cc_override: Option<&str>,
    target: Option<fastc::targets::Target>,
    dev: bool,
    release: bool,
) -> Result<CcPlan> {
    if cc_override.is_some() || target.is_some() {
        let resolved = fastc::targets::resolve_target_compiler(target, cc_override)
            .map_err(|e| miette::miette!("{}", e))?;
        return Ok(CcPlan {
            command: resolved.command,
            prefix_args: resolved.extra_args,
        });
    }
    if let Some(c) = explicit_compiler {
        return Ok(CcPlan {
            command: c.to_string(),
            prefix_args: vec![],
        });
    }
    let command = if dev && !release {
        fastc::build::detect_dev_compiler("cc")
    } else {
        "cc".to_string()
    };
    Ok(CcPlan {
        command,
        prefix_args: vec![],
    })
}

/// Implements `fastc add <url>` — fetches a candidate dependency,
/// surfaces its capability surface, prompts the user, and appends
/// the entry to `fastc.toml` + records the content hash in
/// `fastc.lock`.
///
/// This is intentionally a "transparent prompt" flow, not a silent
/// installer. The point of fastC's supply-chain story is that the
/// user sees what they're authorizing before any of the dep's code
/// runs (and remember: fastC dep code cannot run at install time
/// because there is no build.rs equivalent — the worst a malicious
/// dep can do is be incorrect after you import it).
fn run_add(url: &str, rev: Option<&str>, name_override: Option<&str>, yes: bool) -> Result<()> {
    use fastc::deps::{Cache, Dependency, Fetcher, GitVersion, Manifest};
    use std::io::Write;

    let cwd = std::env::current_dir().into_diagnostic()?;
    let manifest_path = Manifest::find(&cwd)
        .ok_or_else(|| miette::miette!("no fastc.toml found in current directory or parents"))?;
    let project_root = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| cwd.clone());

    eprintln!("Adding dependency from {}", url);

    // 1. Fetch into the shared cache.
    let cache =
        Cache::new().ok_or_else(|| miette::miette!("failed to locate fastc cache directory"))?;
    let fetcher = Fetcher::with_cache(cache);
    let probe_name = name_override.unwrap_or("__probe");
    let probe_version = GitVersion {
        rev: rev.map(|r| r.to_string()),
        ..Default::default()
    };
    let probe_dep = Dependency::Git {
        git: url.to_string(),
        version: probe_version,
        sha256: None,
        sigstore: None,
    };
    let path = fetcher
        .fetch(probe_name, &probe_dep)
        .map_err(|e| miette::miette!("fetch failed: {}", e))?;
    eprintln!("  fetched to {}", path.display());

    let resolved_rev = match rev {
        Some(r) => r.to_string(),
        None => Fetcher::head_commit(&path)
            .map_err(|e| miette::miette!("couldn't resolve HEAD commit: {}", e))?,
    };

    // 2. Compute content hash.
    let sha = fastc::deps::hash_tree(&path)
        .map_err(|e| miette::miette!("hashing dep tree failed: {}", e))?;

    // 3. Probe the dep's own manifest for its name / version.
    let dep_manifest = path.join("fastc.toml");
    if !dep_manifest.exists() {
        return Err(miette::miette!(
            "the fetched dependency at {} has no fastc.toml — refusing to add",
            path.display()
        ));
    }
    let dep_mf = Manifest::load(&dep_manifest)
        .map_err(|e| miette::miette!("invalid dep manifest: {}", e))?;
    let dep_name = name_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| dep_mf.package.name.clone());

    // 4. Scan capability surface.
    let caps = scan_capabilities(&path);

    // 5. Print summary.
    eprintln!();
    eprintln!(
        "  package: {} {}",
        dep_mf.package.name, dep_mf.package.version
    );
    eprintln!("  git:     {}", url);
    eprintln!("  rev:     {}", resolved_rev);
    eprintln!("  sha256:  {}", sha);
    eprintln!("  caps:    {}", format_capabilities(&caps));
    eprintln!();

    if caps
        .iter()
        .any(|c| c == "CapNetConnect" || c == "CapProcSpawn" || c == "CapFsWrite")
    {
        eprintln!(
            "  ⚠ this dependency declares high-impact capabilities. \
            Review its source before approving."
        );
    }

    // 6. Prompt.
    if !yes {
        eprint!("Add `{}` to fastc.toml? [y/N] ", dep_name);
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).into_diagnostic()?;
        let trimmed = answer.trim().to_ascii_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            eprintln!("Aborted. Nothing changed.");
            return Ok(());
        }
    }

    // 7. Append to fastc.toml.
    append_dep_to_manifest(&manifest_path, &dep_name, url, &resolved_rev, &sha)
        .map_err(|e| miette::miette!("failed to update fastc.toml: {}", e))?;
    eprintln!("Updated {}", manifest_path.display());

    // 8. Update the lockfile via BuildContext::lock_dependencies so
    //    every dep (including the new one) ends up anchored.
    let mut ctx = fastc::BuildContext::new(&project_root).map_err(|e| miette::miette!("{}", e))?;
    ctx.lock_dependencies(false)
        .map_err(|e| miette::miette!("{}", e))?;

    Ok(())
}

/// Walk every `.fc` file under `root` and collect the set of `Cap*`
/// types that appear in `ref(Cap...)` / `mref(Cap...)` positions.
/// This is the capability surface — what the dep can do once you've
/// passed it the right tokens.
///
/// Intentionally a string-level scan rather than full parsing: it's
/// fast, works on any dep regardless of compile errors, and the
/// false-positive shape (a comment that happens to mention `CapX`)
/// errs toward over-warning, which is the safer default for a
/// supply-chain prompt.
fn scan_capabilities(root: &std::path::Path) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if name == ".git" || name == "target" || name == "build" {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !name.ends_with(".fc") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for cap in extract_caps_from_text(&text) {
                found.insert(cap);
            }
        }
    }
    found.into_iter().collect()
}

fn extract_caps_from_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if &bytes[i..i + 3] == b"Cap" {
            // Must be a token start: preceded by non-identifier byte.
            let prev_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            if !prev_ok {
                i += 1;
                continue;
            }
            let mut j = i + 3;
            while j < bytes.len() && is_ident_byte(bytes[j]) {
                j += 1;
            }
            if j > i + 3 {
                if let Ok(s) = std::str::from_utf8(&bytes[i..j]) {
                    out.push(s.to_string());
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn format_capabilities(caps: &[String]) -> String {
    if caps.is_empty() {
        return "(none declared)".to_string();
    }
    caps.join(", ")
}

/// Append a new `[dependencies]` entry to fastc.toml. Preserves the
/// file's existing content exactly; just adds the new line under
/// the `[dependencies]` table (creating the table if absent).
///
/// We don't use a TOML serializer here on purpose — `toml`'s
/// serializer rewrites the whole file and would clobber comments,
/// formatting, and field ordering. A line-level append is enough
/// for what `fastc add` needs to do; richer edits live in a future
/// `fastc edit` command if it's ever needed.
fn append_dep_to_manifest(
    path: &std::path::Path,
    name: &str,
    url: &str,
    rev: &str,
    sha256: &str,
) -> std::io::Result<()> {
    let existing = std::fs::read_to_string(path)?;
    let entry = format!(
        "{} = {{ git = \"{}\", rev = \"{}\", sha256 = \"{}\" }}\n",
        name, url, rev, sha256
    );

    let mut out = String::new();
    let mut inserted = false;
    let mut in_deps_section = false;
    let mut deps_section_end: Option<usize> = None;
    for (idx, line) in existing.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_deps_section && deps_section_end.is_none() {
                deps_section_end = Some(idx);
            }
            in_deps_section = trimmed == "[dependencies]";
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_deps_section && deps_section_end.is_none() {
        // [dependencies] is the last section; append at the end.
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&entry);
        inserted = true;
    } else if let Some(_end_idx) = deps_section_end {
        // [dependencies] was followed by another section. Reconstruct
        // with the new entry inserted at the section's end.
        let mut rebuilt = String::new();
        let mut in_deps = false;
        for line in existing.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                if in_deps {
                    if !rebuilt.ends_with("\n\n") {
                        rebuilt.push('\n');
                    }
                    rebuilt.push_str(&entry);
                    in_deps = false;
                }
                if trimmed == "[dependencies]" {
                    in_deps = true;
                }
            }
            rebuilt.push_str(line);
            rebuilt.push('\n');
        }
        out = rebuilt;
        inserted = true;
    }
    if !inserted {
        // No [dependencies] table yet — append one.
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n[dependencies]\n");
        out.push_str(&entry);
    }

    std::fs::write(path, out)
}

/// Write the active `TimingReport` to `dest`. `None` writes JSON to stderr;
/// `Some(path)` writes to that file, creating parent directories as needed.
fn emit_timing(dest: Option<&std::path::Path>) -> Result<()> {
    let Some(report) = fastc::timing::take() else {
        return Ok(());
    };
    let json = report.to_json();
    match dest {
        None => eprintln!("{}", json),
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).into_diagnostic()?;
            }
            std::fs::write(path, &json).into_diagnostic()?;
            eprintln!("Timing report written to {}", path.display());
        }
    }
    Ok(())
}

/// `fastc context` — markdown summary of every `pub` item across the
/// project. Optimized for AI context windows: signatures only, no
/// bodies. Each module section lists its functions, structs, traits,
/// and constants in source order with their full signature and
/// annotation surface.
fn print_context_markdown(file: &fastc::ast::File, module_filter: Option<&str>) {
    println!("# Project Surface\n");
    walk_for_context_md(&file.items, None, module_filter);
}

fn walk_for_context_md(items: &[fastc::ast::Item], path: Option<&str>, filter: Option<&str>) {
    let mut emitted_header = false;
    for item in items {
        match item {
            fastc::ast::Item::Mod(m) => {
                let nested = match path {
                    Some(p) => format!("{}::{}", p, m.name),
                    None => m.name.clone(),
                };
                if let Some(body) = &m.body {
                    walk_for_context_md(body, Some(&nested), filter);
                }
            }
            fastc::ast::Item::Fn(f) => {
                if !filter_match(path, filter) {
                    continue;
                }
                if !emitted_header {
                    println!("## Module `{}`\n", path.unwrap_or("(root)"));
                    emitted_header = true;
                }
                let params = f
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, type_to_string(&p.ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "- `fn {}({}) -> {}`",
                    f.name,
                    params,
                    type_to_string(&f.return_type)
                );
                for a in &f.annotations {
                    println!("  - @{}", a);
                }
                if let Some(p) = &f.purity {
                    println!("  - @purity({:?})", p);
                }
                if let Some(p) = &f.panics {
                    println!("  - @panics({:?})", p);
                }
                if let Some(c) = &f.complexity {
                    println!("  - @complexity({})", bigo_to_string(c));
                }
                for r in &f.requires {
                    println!("  - @requires({})", expr_to_string(r));
                }
                for e in &f.ensures {
                    println!("  - @ensures({})", expr_to_string(e));
                }
            }
            fastc::ast::Item::Struct(s) => {
                if !filter_match(path, filter) {
                    continue;
                }
                if !emitted_header {
                    println!("## Module `{}`\n", path.unwrap_or("(root)"));
                    emitted_header = true;
                }
                println!("- `struct {}`", s.name);
                for fld in &s.fields {
                    println!("  - `{}: {}`", fld.name, type_to_string(&fld.ty));
                }
            }
            fastc::ast::Item::Trait(t) => {
                if !filter_match(path, filter) {
                    continue;
                }
                if !emitted_header {
                    println!("## Module `{}`\n", path.unwrap_or("(root)"));
                    emitted_header = true;
                }
                println!("- `trait {}` ({} methods)", t.name, t.methods.len());
            }
            _ => {}
        }
    }
}

fn filter_match(path: Option<&str>, filter: Option<&str>) -> bool {
    match (path, filter) {
        (_, None) => true,
        (Some(p), Some(f)) => p == f || p.starts_with(&format!("{}::", f)),
        (None, Some(_)) => false,
    }
}

/// `fastc context --format=json` — same surface, JSON shape that
/// mirrors `fastc explain` so MCP / agent tooling can consume it
/// uniformly.
fn print_context_json(file: &fastc::ast::File, _filter: Option<&str>) {
    // For now, JSON form is just the same as explain. The schema is
    // additive — future revisions may split functions / structs /
    // traits / constants into distinct arrays.
    print_explain_json(file);
}

/// `fastc diff` — semantic diff at the AST level. Reports added /
/// removed pub items, signature changes, annotation changes, module
/// header changes.
fn print_diff_markdown(old: &fastc::ast::File, new: &fastc::ast::File, _include_bodies: bool) {
    use std::collections::BTreeMap;

    let old_fns = collect_pub_fns(&old.items);
    let new_fns = collect_pub_fns(&new.items);
    let mut old_map: BTreeMap<String, &fastc::ast::FnDecl> = BTreeMap::new();
    let mut new_map: BTreeMap<String, &fastc::ast::FnDecl> = BTreeMap::new();
    for (k, v) in &old_fns {
        old_map.insert(k.clone(), *v);
    }
    for (k, v) in &new_fns {
        new_map.insert(k.clone(), *v);
    }

    let mut added: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let mut changed: Vec<(String, String, String)> = Vec::new();

    for (k, f_new) in &new_map {
        match old_map.get(k) {
            None => added.push(k.clone()),
            Some(f_old) => {
                let old_sig = fn_signature_summary(f_old);
                let new_sig = fn_signature_summary(f_new);
                if old_sig != new_sig {
                    changed.push((k.clone(), old_sig, new_sig));
                }
            }
        }
    }
    for k in old_map.keys() {
        if !new_map.contains_key(k) {
            removed.push(k.clone());
        }
    }

    println!("# Semantic diff\n");
    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        println!("_No semantic changes._");
        return;
    }
    if !added.is_empty() {
        println!("## Added (+{})\n", added.len());
        for k in &added {
            println!("- {}", k);
        }
        println!();
    }
    if !removed.is_empty() {
        println!("## Removed (-{})\n", removed.len());
        for k in &removed {
            println!("- {}", k);
        }
        println!();
    }
    if !changed.is_empty() {
        println!("## Changed ({})\n", changed.len());
        for (k, old_sig, new_sig) in &changed {
            println!("- **{}**", k);
            println!("  - was: `{}`", old_sig);
            println!("  - now: `{}`", new_sig);
        }
    }
}

fn print_diff_json(old: &fastc::ast::File, new: &fastc::ast::File, _include_bodies: bool) {
    use std::collections::BTreeMap;

    let old_fns = collect_pub_fns(&old.items);
    let new_fns = collect_pub_fns(&new.items);
    let mut old_map: BTreeMap<String, &fastc::ast::FnDecl> = BTreeMap::new();
    let mut new_map: BTreeMap<String, &fastc::ast::FnDecl> = BTreeMap::new();
    for (k, v) in &old_fns {
        old_map.insert(k.clone(), *v);
    }
    for (k, v) in &new_fns {
        new_map.insert(k.clone(), *v);
    }
    let mut added: Vec<&String> = Vec::new();
    let mut removed: Vec<&String> = Vec::new();
    let mut changed: Vec<(String, String, String)> = Vec::new();
    for (k, f_new) in &new_map {
        match old_map.get(k) {
            None => added.push(k),
            Some(f_old) => {
                let old_sig = fn_signature_summary(f_old);
                let new_sig = fn_signature_summary(f_new);
                if old_sig != new_sig {
                    changed.push((k.clone(), old_sig, new_sig));
                }
            }
        }
    }
    for k in old_map.keys() {
        if !new_map.contains_key(k) {
            removed.push(k);
        }
    }
    println!("{{");
    println!(
        "  \"added\": [{}],",
        added
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  \"removed\": [{}],",
        removed
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  \"changed\": [");
    for (i, (k, old_sig, new_sig)) in changed.iter().enumerate() {
        let comma = if i + 1 < changed.len() { "," } else { "" };
        println!(
            "    {{ \"name\": \"{}\", \"old\": \"{}\", \"new\": \"{}\" }}{}",
            escape_json(k),
            escape_json(old_sig),
            escape_json(new_sig),
            comma
        );
    }
    println!("  ]");
    println!("}}");
}

fn collect_pub_fns(items: &[fastc::ast::Item]) -> Vec<(String, &fastc::ast::FnDecl)> {
    let mut out = Vec::new();
    collect_pub_fns_inner(items, None, &mut out);
    out
}

fn collect_pub_fns_inner<'a>(
    items: &'a [fastc::ast::Item],
    path: Option<&str>,
    out: &mut Vec<(String, &'a fastc::ast::FnDecl)>,
) {
    for item in items {
        match item {
            fastc::ast::Item::Fn(f) => {
                let key = match path {
                    Some(p) => format!("{}::{}", p, f.name),
                    None => f.name.clone(),
                };
                out.push((key, f));
            }
            fastc::ast::Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let nested = match path {
                        Some(p) => format!("{}::{}", p, m.name),
                        None => m.name.clone(),
                    };
                    collect_pub_fns_inner(body, Some(&nested), out);
                }
            }
            _ => {}
        }
    }
}

fn fn_signature_summary(f: &fastc::ast::FnDecl) -> String {
    let params = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, type_to_string(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let purity = match &f.purity {
        Some(p) => format!(" @purity({:?})", p),
        None => String::new(),
    };
    let panics = match &f.panics {
        Some(p) => format!(" @panics({:?})", p),
        None => String::new(),
    };
    format!(
        "fn {}({}) -> {}{}{}",
        f.name,
        params,
        type_to_string(&f.return_type),
        purity,
        panics
    )
}
