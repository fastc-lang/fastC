//! FastC transpiler - a C-like language that compiles to C11
//!
//! FastC is designed around NASA/JPL's "Power of 10" rules for
//! safety-critical code, providing strong static guarantees and
//! predictable behavior.

pub mod ast;
pub mod build;
pub mod deps;
pub mod diag;
pub mod emit;
pub mod fmt;
pub mod lexer;
pub mod lower;
pub mod p10;
pub mod parser;
pub mod resolve;
pub mod scaffold;
pub mod typecheck;

mod driver;

pub use ast::Item;
pub use build::{BuildContext, BuildError};
pub use deps::{Cache, Fetcher, Lockfile, Manifest, ModuleLoader, ModuleResolver};
pub use driver::{check, check_with_p10, compile, compile_with_options, compile_with_p10, parse};
pub use fmt::{check_formatted, format};
pub use p10::{ComplianceReport, P10Checker, P10Config, ProjectReport, SafetyLevel};
pub use scaffold::{BuildTemplate, ProjectType, create_project, init_project};
