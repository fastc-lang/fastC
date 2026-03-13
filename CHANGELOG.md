# Changelog

All notable changes to FastC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- MkDocs documentation site
- `fastc build --cc` for C compiler integration
- `fastc run` command to build and execute
- Module system with `mod` declarations
- Dependency management with `fastc.toml`
- Language Server Protocol (LSP) support

### Changed
- Improved error messages with source locations
- Better C code formatting in output

### Fixed
- Module functions now included in generated C output

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- Lexer with all FastC tokens
- Recursive-descent parser
- Name resolution
- Type checking with `unsafe` tracking
- C11 code emission
- Pointer types: `ref(T)`, `mref(T)`, `raw(T)`, `rawm(T)`
- Array types: `arr(T, N)`, `slice(T)`
- Optional type: `opt(T)` with `some`, `none`, `if let`
- Result type: `res(T, E)` with `ok`, `err`
- Struct and enum definitions
- `extern "C"` blocks for FFI
- `@repr(C)` attribute for C-compatible layout
- Header generation with `--emit-header`
- Short-circuit `&&` and `||` operators
- Division by zero runtime checks
- Guaranteed evaluation order

### Security
- Safe/unsafe boundary enforcement
- Bounds checking for array access in safe code

[Unreleased]: https://github.com/Skelf-Research/fastc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Skelf-Research/fastc/releases/tag/v0.1.0
