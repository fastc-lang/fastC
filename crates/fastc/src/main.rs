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
    },

    /// Type-check a FastC source file without emitting C
    Check {
        /// Input FastC source file
        input: PathBuf,
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
        } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            let (c_code, header) = fastc::compile_with_options(&source, &filename, emit_header)?;

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

        Commands::Check { input } => {
            let source = std::fs::read_to_string(&input).into_diagnostic()?;
            let filename = input.display().to_string();

            fastc::check(&source, &filename)?;
            eprintln!("No errors found.");
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
