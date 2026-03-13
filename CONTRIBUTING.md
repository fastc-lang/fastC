# Contributing to FastC

Thank you for your interest in contributing to FastC! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### Reporting Bugs

Before submitting a bug report:

1. Check the [existing issues](https://github.com/Skelf-Research/fastc/issues) to avoid duplicates
2. Ensure you're using the latest version
3. Collect relevant information (OS, Rust version, FastC version)

When submitting a bug report, include:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Minimal code example that demonstrates the issue
- Error messages and stack traces

### Suggesting Features

Feature requests are welcome! Please:

1. Check existing issues and discussions first
2. Describe the problem your feature would solve
3. Propose a concrete solution
4. Consider how it affects existing code

### Pull Requests

1. **Fork and clone** the repository
2. **Create a branch** for your changes: `git checkout -b feature/my-feature`
3. **Make your changes** following our coding standards
4. **Add tests** for new functionality
5. **Run the test suite**: `cargo test`
6. **Commit** with clear messages
7. **Push** and create a Pull Request

#### PR Guidelines

- Keep PRs focused on a single change
- Update documentation as needed
- Add tests for new features
- Ensure CI passes
- Respond to review feedback promptly

## Development Setup

### Prerequisites

- Rust 1.85 or later
- Git

### Building

```bash
git clone https://github.com/Skelf-Research/fastc.git
cd fastc
cargo build
```

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Project Structure

```
crates/
├── fastc/           # Main compiler
│   ├── src/
│   │   ├── lexer/   # Tokenization
│   │   ├── parser/  # Parsing
│   │   ├── ast/     # AST definitions
│   │   ├── resolve/ # Name resolution
│   │   ├── typecheck/ # Type checking
│   │   ├── lower/   # FastC → C lowering
│   │   └── emit/    # C code emission
│   └── tests/       # Integration tests
└── fastc-lsp/       # Language server
```

## Coding Standards

### Rust Style

- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` and address warnings
- Write documentation comments for public APIs
- Keep functions focused and reasonably sized

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add support for generic functions

- Implement generic type parameters
- Add type inference for generics
- Update parser for new syntax
```

Prefixes:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `test:` - Tests
- `refactor:` - Code refactoring
- `chore:` - Maintenance

### Testing

- Write tests for new features
- Include both positive and negative test cases
- Use snapshot tests for compiler output
- Test edge cases and error conditions

## Review Process

1. All PRs require at least one review
2. CI must pass before merging
3. Maintain a clean commit history
4. Squash commits if requested

## Getting Help

- Open an issue for questions
- Check the [documentation](https://docs.skelfresearch.com/fastc)
- Review existing issues and PRs for context

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
