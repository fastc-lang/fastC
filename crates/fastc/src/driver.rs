//! Compilation driver - orchestrates the compilation phases

use std::path::Path;

use crate::deps::{Manifest, ModuleLoader};
use crate::diag::CompileError;
use crate::emit::Emitter;
use crate::lexer::{strip_comments, Lexer};
use crate::lower::Lower;
use crate::parser::Parser;
use crate::resolve::Resolver;
use crate::typecheck::TypeChecker;

/// Type-check FastC source without emitting C
///
/// Runs phases 1-4 only: Lex, Parse, Resolve, TypeCheck.
/// Returns `Ok(())` if the source is valid, or an error otherwise.
pub fn check(source: &str, filename: &str) -> Result<(), CompileError> {
    // Phase 1: Lex (strip comments for parser)
    let lexer = Lexer::new(source);
    let tokens = strip_comments(lexer.collect());

    // Phase 2: Parse
    let mut parser = Parser::new(&tokens, source, filename);
    let mut ast = parser.parse_file()?;

    // Phase 2.5: Module expansion (if in a project)
    let source_path = Path::new(filename);
    if let Some(project_root) = find_project_root(source_path) {
        let source_dir = source_path.parent().unwrap_or(Path::new("."));
        let mut loader = ModuleLoader::new(&project_root);
        loader.expand_modules(&mut ast, source_dir)?;
    }

    // Phase 3: Resolve names
    let mut resolver = Resolver::new(source);
    let symbols = {
        resolver.resolve(&ast)?;
        resolver.into_symbols()
    };

    // Phase 4: Type check
    let mut typechecker = TypeChecker::new(source, symbols);
    typechecker.check(&ast)?;

    Ok(())
}

/// Compile FastC source code to C11
pub fn compile(source: &str, filename: &str) -> Result<String, CompileError> {
    let (c_code, _) = compile_with_options(source, filename, false)?;
    Ok(c_code)
}

/// Compile FastC source code to C11 with optional header generation
pub fn compile_with_options(
    source: &str,
    filename: &str,
    emit_header: bool,
) -> Result<(String, Option<String>), CompileError> {
    // Phase 1: Lex (strip comments for parser)
    let lexer = Lexer::new(source);
    let tokens = strip_comments(lexer.collect());

    // Phase 2: Parse
    let mut parser = Parser::new(&tokens, source, filename);
    let mut ast = parser.parse_file()?;

    // Phase 2.5: Module expansion (if in a project)
    let source_path = Path::new(filename);
    if let Some(project_root) = find_project_root(source_path) {
        let source_dir = source_path.parent().unwrap_or(Path::new("."));
        let mut loader = ModuleLoader::new(&project_root);
        loader.expand_modules(&mut ast, source_dir)?;
    }

    // Phase 3: Resolve names
    let mut resolver = Resolver::new(source);
    let symbols = {
        resolver.resolve(&ast)?;
        resolver.into_symbols()
    };

    // Phase 4: Type check
    let mut typechecker = TypeChecker::new(source, symbols);
    typechecker.check(&ast)?;

    // Phase 5: Lower to C AST
    let mut lowerer = Lower::new();
    let c_ast = lowerer.lower(&ast);

    // Phase 6: Emit C code
    let mut emitter = Emitter::new();
    let c_code = emitter.emit(&c_ast);

    // Phase 7 (optional): Emit header
    let header = if emit_header {
        // Extract module name from filename (without extension)
        let module_name = std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        Some(emitter.emit_header(&c_ast, module_name))
    } else {
        None
    };

    Ok((c_code, header))
}

/// Find the project root by looking for fastc.toml
fn find_project_root(source_path: &Path) -> Option<std::path::PathBuf> {
    Manifest::find(source_path).map(|manifest_path| {
        manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    })
}
