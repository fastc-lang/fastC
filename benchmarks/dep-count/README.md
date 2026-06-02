# Dependency-Count Benchmark

D3 in the v1.x close-out plan. Measures the transitive dependency
graph and number of executable build-script invocations for the
same toy program — an HTTP-fetch-and-parse-JSON server — across
fastC, Rust, Zig, Go, and C.

Each sub-directory contains the equivalent program in the target
language. The `run.sh` script counts:

1. **Transitive dependency count**: number of unique packages
   resolved into the build graph. For Rust this is `cargo tree`
   output; for fastC it's the `fastc.lock` line count; for Go it's
   `go list -m all`; for Zig it's parsing `build.zig.zon`; for C
   it's the explicit list in the makefile.
2. **Executable build-script count**: number of Turing-complete
   build steps in the dep tree. Cargo `build.rs`, gradle plugins,
   npm postinstall, etc. fastC's manifest disallows these by
   design (closed-schema, no build scripts) so the count is
   structurally 0.

Results land in `results.csv` with columns `(language, total_deps,
build_scripts, host, timestamp, git_sha)`.

## Why this matters

The roadmap's claim is that fastC's "no executable build scripts,
ever" + content-hashed vendor-first deps + closed-schema manifest
produces a structurally smaller dep surface than language ecosystems
that have evolved with thousands of cargo crates / npm packages.
Per the roadmap (line 462), the expected numbers for an
HTTP+JSON-parse program are roughly:

| Language | Total deps | Build scripts |
|---|---|---|
| fastC | 4 | 0 |
| Go | 12 | 0 |
| Zig | 8 | 0 |
| Rust | 87+ | varies |
| C | varies | 0 |

This script measures the actual numbers and surfaces them in the
documentation site rubric.
