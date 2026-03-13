# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in FastC, please report it responsibly.

### How to Report

**Do not open a public issue for security vulnerabilities.**

Instead, please email us at: **security@skelfresearch.com**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested fixes (optional)

### What to Expect

1. **Acknowledgment**: We will acknowledge receipt within 48 hours
2. **Assessment**: We will assess the vulnerability and its impact
3. **Updates**: We will keep you informed of our progress
4. **Resolution**: We aim to resolve critical issues within 90 days
5. **Credit**: We will credit reporters in our release notes (unless you prefer anonymity)

### Scope

This policy applies to:
- The FastC compiler (`fastc`)
- The FastC language server (`fastc-lsp`)
- The FastC runtime (`runtime/`)
- Generated C code security implications

### Out of Scope

- Vulnerabilities in third-party dependencies (report to their maintainers)
- Issues in user-written FastC code
- Theoretical attacks without proof of concept

## Security Considerations

### Generated C Code

FastC generates C11 code. While FastC provides safety guarantees in safe code:

- `unsafe` blocks can introduce vulnerabilities
- The generated C inherits the security properties of your C compiler
- Always use compiler hardening flags in production (`-fstack-protector`, etc.)

### Compiler Security

The FastC compiler:
- Does not execute user code during compilation
- Does not make network requests
- Processes only local files specified by the user

## Best Practices

When using FastC in security-sensitive applications:

1. Minimize `unsafe` code
2. Review generated C code for critical paths
3. Use memory sanitizers during development
4. Keep FastC updated to the latest version
5. Enable compiler security features when building generated C
