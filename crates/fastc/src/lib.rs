//! FastC transpiler - a C-like language that compiles to C11

pub mod ast;
pub mod build;
pub mod deps;
pub mod diag;
pub mod emit;
pub mod fmt;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod resolve;
pub mod scaffold;
pub mod typecheck;

mod driver;

pub use build::{BuildContext, BuildError};
pub use deps::{Cache, Fetcher, Lockfile, Manifest, ModuleLoader, ModuleResolver};
pub use driver::{check, compile, compile_with_options};
pub use fmt::{check_formatted, format};
pub use scaffold::{BuildTemplate, ProjectType, create_project, init_project};
