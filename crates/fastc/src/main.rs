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
        #[arg(long)]
        release: bool,

        /// Output directory for generated C files
        #[arg(short, long, default_value = "build")]
        output: PathBuf,

        /// Compile the generated C code with a C compiler
        #[arg(long)]
        cc: bool,

        /// C compiler to use (default: cc)
        #[arg(long, default_value = "cc")]
        compiler: String,

        /// Additional flags to pass to the C compiler
        #[arg(long)]
        cflags: Option<String>,
    },

    /// Build, compile, and run the project
    Run {
        /// Build in release mode (optimizations enabled)
        #[arg(long)]
        release: bool,

        /// C compiler to use (default: cc)
        #[arg(long, default_value = "cc")]
        compiler: String,

        /// Additional flags to pass to the C compiler
        #[arg(long)]
        cflags: Option<String>,

        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Fetch project dependencies without building
    Fetch,
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
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            // P10 rules are always enabled (use --safety-level=relaxed to disable)
            let mut config = fastc::P10Config::from_level(safety_level.into());
            if strict {
                config.strict_mode = true;
            }
            let (c_code, header) = fastc::compile_with_p10(&source, &filename, emit_header, config)?;

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

        Commands::Check { input, p10: _, safety_level, strict } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            // P10 rules are always enabled (use --safety-level=relaxed to disable)
            let mut config = fastc::P10Config::from_level(safety_level.into());
            if strict {
                config.strict_mode = true;
            }
            fastc::check_with_p10(&source, &filename, config)?;
            eprintln!("No errors found.");
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

                let function_count = ast.items.iter().filter(|item| matches!(item, fastc::Item::Fn(_))).count();
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
                    CliReportFormat::Compact => serde_json::to_string(&project_report).unwrap_or_default(),
                    CliReportFormat::Text => {
                        // For project reports in text, concatenate individual reports
                        let mut text = String::new();
                        text.push_str(&format!("Project: {}\n", project_report.project_name.as_deref().unwrap_or("unnamed")));
                        text.push_str(&format!("Status: {:?}\n", project_report.status));
                        text.push_str(&format!("Files: {} analyzed, {} compliant\n\n",
                            project_report.summary.files_analyzed,
                            project_report.summary.files_compliant));
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
                    CliReportFormat::Json => serde_json::to_string_pretty(&file_reports).unwrap_or_default(),
                    CliReportFormat::Compact => serde_json::to_string(&file_reports).unwrap_or_default(),
                    CliReportFormat::Text => {
                        file_reports.iter().map(|r| r.to_text()).collect::<Vec<_>>().join("\n\n")
                    }
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
            output,
            cc,
            compiler,
            cflags,
        } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            // Fetch dependencies first
            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            // Compile the project to C
            let c_file = ctx
                .compile(&output, release)
                .map_err(|e| miette::miette!("{}", e))?;

            // Optionally compile with C compiler
            if cc {
                let cflags_vec: Vec<&str> = cflags
                    .as_deref()
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                ctx.cc_compile(&c_file, &compiler, &cflags_vec, release)
                    .map_err(|e| miette::miette!("{}", e))?;
            }
        }

        Commands::Run {
            release,
            compiler,
            cflags,
            args,
        } => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            // Fetch dependencies first
            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            // Compile the project to C
            let output = PathBuf::from("build");
            let c_file = ctx
                .compile(&output, release)
                .map_err(|e| miette::miette!("{}", e))?;

            // Compile with C compiler
            let cflags_vec: Vec<&str> = cflags
                .as_deref()
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            let executable = ctx
                .cc_compile(&c_file, &compiler, &cflags_vec, release)
                .map_err(|e| miette::miette!("{}", e))?;

            // Run the program
            ctx.run(&executable, &args)
                .map_err(|e| miette::miette!("{}", e))?;
        }

        Commands::Fetch => {
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let mut ctx =
                fastc::BuildContext::new(&current_dir).map_err(|e| miette::miette!("{}", e))?;

            ctx.fetch_dependencies()
                .map_err(|e| miette::miette!("{}", e))?;

            eprintln!("Dependencies fetched successfully.");
        }
    }

    Ok(())
}
