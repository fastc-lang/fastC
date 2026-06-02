//! FastC transpiler - a C-like language that compiles to C11
//!
//! FastC is designed around NASA/JPL's "Power of 10" rules for
//! safety-critical code, providing strong static guarantees and
//! predictable behavior.

pub mod annotation_check;
pub mod ast;
pub mod bench;
pub mod build;
pub mod build_cache;
pub mod cap_check;
pub mod caps_summary;
pub mod db;
pub mod deps;
pub mod desugar;
pub mod diag;
pub mod discharge;
pub mod emit;
pub mod fmt;
pub mod lexer;
pub mod lower;
pub mod mono;
pub mod noalloc_check;
pub mod p10;
pub mod parser;
pub mod prelude;
pub mod resolve;
pub mod scaffold;
pub mod targets;
pub mod timing;
pub mod typecheck;

mod driver;

pub use ast::Item;
pub use build::{BuildContext, BuildError};
pub use deps::{Cache, Fetcher, Lockfile, Manifest, ModuleLoader, ModuleResolver};
pub use driver::{
    check, check_with_p10, compile, compile_project, compile_with_options, compile_with_p10,
    compile_with_p10_and_discharge, parse,
};
pub use fmt::{check_formatted, format};
pub use p10::{ComplianceReport, P10Checker, P10Config, ProjectReport, SafetyLevel};
pub use scaffold::{BuildTemplate, ProjectType, create_project, init_project};
pub use timing::{CacheStatus, PassTiming, TimingReport};
