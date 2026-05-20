//! Compilation driver - orchestrates the compilation phases

use std::path::Path;

use crate::ast::File;
use crate::deps::{Manifest, ModuleLoader};
use crate::desugar::desugar;
use crate::diag::CompileError;
use crate::emit::Emitter;
use crate::lexer::{Lexer, strip_comments};
use crate::lower::Lower;
use crate::mono::monomorphize;
use crate::p10::{P10Checker, P10Config};
use crate::parser::Parser;
use crate::prelude::prelude_items;
use crate::resolve::Resolver;
use crate::timing::time_pass;
use crate::typecheck::TypeChecker;

/// Parse FastC source code into an AST (phases 1-2 only)
///
/// This is useful for analysis tools that need the AST without full compilation.
/// Does not perform name resolution or type checking.
pub fn parse(source: &str, filename: &str) -> Result<File, CompileError> {
    // Phase 1: Lex (strip comments for parser)
    let tokens = time_pass("lex", || {
        let lexer = Lexer::new(source);
        strip_comments(lexer.collect())
    });

    // Phase 2: Parse
    let ast = time_pass("parse", || {
        let mut parser = Parser::new(&tokens, source, filename);
        parser.parse_file()
    })?;

    Ok(ast)
}

/// Type-check FastC source without emitting C
///
/// Runs phases 1-4 plus Power of 10 checking with standard config.
/// Returns `Ok(())` if the source is valid, or an error otherwise.
pub fn check(source: &str, filename: &str) -> Result<(), CompileError> {
    check_with_p10(source, filename, P10Config::standard())
}

/// Type-check FastC source with Power of 10 rule enforcement
///
/// Runs phases 1-4 plus Power of 10 checking.
/// Returns `Ok(())` if the source is valid, or an error otherwise.
pub fn check_with_p10(
    source: &str,
    filename: &str,
    p10_config: P10Config,
) -> Result<(), CompileError> {
    let tokens = time_pass("lex", || {
        let lexer = Lexer::new(source);
        strip_comments(lexer.collect())
    });

    let mut ast = time_pass("parse", || {
        let mut parser = Parser::new(&tokens, source, filename);
        parser.parse_file()
    })?;

    let source_path = Path::new(filename);
    if let Some(project_root) = find_project_root(source_path) {
        let source_dir = source_path.parent().unwrap_or(Path::new("."));
        time_pass("module_load", || {
            let mut loader = ModuleLoader::new(&project_root);
            loader.expand_modules(&mut ast, source_dir)
        })?;
    }

    // Inject the built-in prelude (Eq/Ord/Copy traits + primitive impls)
    // before any further pass — these declarations become part of the
    // user's compilation unit. See `crate::prelude`.
    let ast = time_pass("prelude", || prepend_prelude(ast));

    // Lift impl blocks' methods to free `Type_method` functions. The
    // original `Item::Impl` and `Item::Trait` items remain so resolve,
    // typecheck, and mono can read the trait-impl table.
    let ast = time_pass("desugar", || desugar(&ast));

    let symbols = time_pass("resolve", || {
        let mut resolver = Resolver::new(source);
        resolver.resolve(&ast)?;
        Ok::<_, CompileError>(resolver.into_symbols())
    })?;

    time_pass("typecheck", || {
        let mut typechecker = TypeChecker::new(source, symbols);
        typechecker.check(&ast)
    })?;

    time_pass("p10", || {
        let p10_checker = P10Checker::new(p10_config);
        p10_checker.check_and_report(&ast, source)
    })?;

    Ok(())
}

/// Compile FastC source code to C11
pub fn compile(source: &str, filename: &str) -> Result<String, CompileError> {
    let (c_code, _) = compile_with_options(source, filename, false)?;
    Ok(c_code)
}

/// Compile FastC source code to C11 with Power of 10 rule enforcement
pub fn compile_with_p10(
    source: &str,
    filename: &str,
    emit_header: bool,
    p10_config: P10Config,
) -> Result<(String, Option<String>), CompileError> {
    let tokens = time_pass("lex", || {
        let lexer = Lexer::new(source);
        strip_comments(lexer.collect())
    });

    let mut ast = time_pass("parse", || {
        let mut parser = Parser::new(&tokens, source, filename);
        parser.parse_file()
    })?;

    let source_path = Path::new(filename);
    if let Some(project_root) = find_project_root(source_path) {
        let source_dir = source_path.parent().unwrap_or(Path::new("."));
        time_pass("module_load", || {
            let mut loader = ModuleLoader::new(&project_root);
            loader.expand_modules(&mut ast, source_dir)
        })?;
    }

    // Inject the built-in prelude (Eq/Ord/Copy traits + primitive impls)
    // before any further pass — these declarations become part of the
    // user's compilation unit. See `crate::prelude`.
    let ast = time_pass("prelude", || prepend_prelude(ast));

    // Lift impl blocks' methods to free `Type_method` functions. The
    // original `Item::Impl` and `Item::Trait` items remain so resolve,
    // typecheck, and mono can read the trait-impl table.
    let ast = time_pass("desugar", || desugar(&ast));

    let symbols = time_pass("resolve", || {
        let mut resolver = Resolver::new(source);
        resolver.resolve(&ast)?;
        Ok::<_, CompileError>(resolver.into_symbols())
    })?;

    time_pass("typecheck", || {
        let mut typechecker = TypeChecker::new(source, symbols.clone());
        typechecker.check(&ast)
    })?;

    time_pass("p10", || {
        let p10_checker = P10Checker::new(p10_config);
        p10_checker.check_and_report(&ast, source)
    })?;

    // Generic functions are erased here; every call site is rewritten to the
    // mangled specialized name. Non-generic programs see no change.
    let mono_ast = time_pass("mono", || monomorphize(&ast, &symbols, source))?;

    let c_ast = time_pass("lower", || {
        let mut lowerer = Lower::new();
        lowerer.lower(&mono_ast)
    });

    let (c_code, header) = time_pass("emit", || {
        let mut emitter = Emitter::new();
        let c_code = emitter.emit(&c_ast);
        let header = if emit_header {
            let module_name = std::path::Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            Some(emitter.emit_header(&c_ast, module_name))
        } else {
            None
        };
        (c_code, header)
    });

    Ok((c_code, header))
}

/// Compile FastC source code to C11 with optional header generation
///
/// Uses standard Power of 10 config by default.
pub fn compile_with_options(
    source: &str,
    filename: &str,
    emit_header: bool,
) -> Result<(String, Option<String>), CompileError> {
    compile_with_p10(source, filename, emit_header, P10Config::standard())
}

/// Compile FastC source code to C11 with dependency support
///
/// This entry point is used by `BuildContext::compile` to pass dependency paths
/// through the module loader.
pub fn compile_project(
    source: &str,
    filename: &str,
    emit_header: bool,
    dep_dirs: Vec<(String, std::path::PathBuf)>,
) -> Result<(String, Option<String>), CompileError> {
    let tokens = time_pass("lex", || {
        let lexer = Lexer::new(source);
        strip_comments(lexer.collect())
    });

    let mut ast = time_pass("parse", || {
        let mut parser = Parser::new(&tokens, source, filename);
        parser.parse_file()
    })?;

    let source_path = Path::new(filename);
    if let Some(project_root) = find_project_root(source_path) {
        let source_dir = source_path.parent().unwrap_or(Path::new("."));
        time_pass("module_load", || {
            let mut loader = ModuleLoader::with_dep_dirs(&project_root, dep_dirs);
            loader.expand_modules(&mut ast, source_dir)
        })?;
    }

    // Inject the built-in prelude (Eq/Ord/Copy traits + primitive impls)
    // before any further pass — these declarations become part of the
    // user's compilation unit. See `crate::prelude`.
    let ast = time_pass("prelude", || prepend_prelude(ast));

    // Lift impl blocks' methods to free `Type_method` functions. The
    // original `Item::Impl` and `Item::Trait` items remain so resolve,
    // typecheck, and mono can read the trait-impl table.
    let ast = time_pass("desugar", || desugar(&ast));

    let symbols = time_pass("resolve", || {
        let mut resolver = Resolver::new(source);
        resolver.resolve(&ast)?;
        Ok::<_, CompileError>(resolver.into_symbols())
    })?;

    time_pass("typecheck", || {
        let mut typechecker = TypeChecker::new(source, symbols.clone());
        typechecker.check(&ast)
    })?;

    time_pass("p10", || {
        let p10_checker = P10Checker::new(P10Config::standard());
        p10_checker.check_and_report(&ast, source)
    })?;

    let mono_ast = time_pass("mono", || monomorphize(&ast, &symbols, source))?;

    let c_ast = time_pass("lower", || {
        let mut lowerer = Lower::new();
        lowerer.lower(&mono_ast)
    });

    let (c_code, header) = time_pass("emit", || {
        let mut emitter = Emitter::new();
        let c_code = emitter.emit(&c_ast);
        let header = if emit_header {
            let module_name = std::path::Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            Some(emitter.emit_header(&c_ast, module_name))
        } else {
            None
        };
        (c_code, header)
    });

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

/// Prepend the built-in prelude items (traits + primitive impls) to a
/// user file. The prelude provides `Eq`, `Ord`, `Copy` and the
/// corresponding primitive impls so bounded generics like
/// `fn max[T: Ord](...)` can be instantiated with primitive types.
fn prepend_prelude(mut user: File) -> File {
    let mut items = prelude_items();
    items.append(&mut user.items);
    File { items }
}
