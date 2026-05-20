# Ecosystem Strategy

This document specifies fastC's package and ecosystem plan: how a new language with one fork gets to a usable library ecosystem without paying npm's or Cargo's tax in malware exposure, dependency bloat, and agent confusion.

The plan lands across **stages 1.7 (Vendor-First Package System)** and **1.8 (fastc-core Curated Stdlib Extensions)** of the [roadmap](roadmap.md). The package infrastructure is documented in [docs/supply-chain.md](supply-chain.md); this document covers what we curate, why, and how the ecosystem grows from one fork to a million-line codebase without losing the supply-chain story.

## The thesis

For the first two years, the answer to "is there a fastC library for X" should be:

- **"Yes, in fastc-core."** A canonical, audited, capability-typed, contract-annotated package under the `Skelf-Research/fastc-core` organization.
- **"No, write it locally."** Vendor it into your project; you have the source, the language is small, the surface is documented.

We deliberately do not say "yes, here is a community package." Not because the community will not produce good packages — they will — but because the *trust* a fastC user gets from `fastc-core` is exactly what the supply-chain story is built on. Blessing community packages too early dilutes that trust. We can revisit after the first audit cycle (~12 months) when there is real signal on which community packages have held up.

This is the Go approach, intentionally. Go's success despite a smaller community than Rust comes in large part from `golang.org/x/*` and the standard library being good enough that most projects do not need anything else for years.

## The chicken-and-egg cycle

Every new package ecosystem hits the same wall, articulated clearly in McKayla Washburn's January 2026 Zig package talk: "You cannot get adoption without tooling, and you cannot get tooling without adoption." Every new language pays the M×N integration tax — Snyk, Socket, Dependabot, Renovate, JFrog, SPDX, CycloneDX, PURL, Sigstore, GitHub dependency graph — all of them need to add support, and most will not until you have users, and you do not get users until they have support.

fastC breaks the cycle by **not needing most of that tooling**. Specifically:

- **No central registry → no Snyk-equivalent needed.** Capability-aware `fastc add` does dependency review at install time, surfacing the very information Snyk and Socket extract.
- **Content-hashed deps → no Dependabot needed for version pinning.** Versions are commit SHAs; there is no semver drift to track.
- **Sigstore signing → no JFrog-equivalent needed.** The signature is verifiable from any client without a paid platform.
- **No build scripts → no proc-macro audit tooling needed.** There is no proc-macro to audit.

We do still want the tooling that exists (GitHub dependency graph integration, SPDX SBOM generation), and we will add it as adoption grows. But the *baseline* is usable without any of them.

## The fastc-core curation plan

The target is **30–50 curated packages under `Skelf-Research/fastc-core` within 12 months.** Curation criteria for every package:

1. **Full annotation coverage.** Every `pub` function has the complete annotation set (`@mem`, `@caps`, `@requires`, `@ensures`, `@panics`, `@purity`, `@complexity`).
2. **Capability-typed I/O.** I/O signatures take capability tokens. No ambient access.
3. **Contract-annotated public API.** Pre/postconditions on every public function (lowered to runtime asserts in stage 1.5; SMT-discharged in 2.1).
4. **Sigstore-signed releases.** Every tag is signed via Skelf's keyless flow.
5. **`AGENTS.md` in every repo.** Documents the canonical idiom for the package, with examples, capability requirements, and contract summaries. This is what coding agents read.
6. **Test coverage minimum 80%.** Tests are in-tree, run on every PR.
7. **One reviewer + one author per change.** No solo merges to `main`.
8. **Audit cycle.** Every package re-audited every 6 months. Audit record in `AUDIT.md` in the repo, with reviewer initials, date, and a one-line "no concerns" or a list of concerns + remediation.

A package that does not meet all eight properties does not ship as `fastc-core`.

## Launch set (weeks 3–4 of the 8-week plan)

Five packages, calibrated to the minimum surface a real fastC project needs:

| Package | Purpose | Caps | Notes |
|---------|---------|------|-------|
| `fastc-http` | HTTP/1.1 client and server | `net.connect`, `net.listen`, `time.read` | Sync only in launch; async waits for stage 2.3 |
| `fastc-json` | JSON parser + emitter | `()` (pure) | Streaming + DOM-style APIs |
| `fastc-toml` | TOML parser | `()` (pure) | Used for `fastc.toml` itself |
| `fastc-log` | Structured logging | `fs.write("./logs/")` configurable | JSON output by default |
| `fastc-cli` | Argument parsing | `()` (pure) for parsing, `env.read` for env-fallback | Derive-style API |

These five let a user build a working "HTTP service that logs to disk and reads config from TOML." The `MANIFESTO.md` launch demo (week 7–8) is exactly that program, compiled in fastC, Rust, Zig, and Go, with token counts and compile times published.

## Six-month set

Six more packages, calibrated to round out the typical service backend:

| Package | Purpose | Caps |
|---------|---------|------|
| `fastc-sqlite` | SQLite binding (via system library) | `fs.read`, `fs.write` (db path), `proc.spawn` (none — pure FFI) |
| `fastc-crypto-primitives` | Constant-time SHA-256, AES, Ed25519 verify | `()` (pure) |
| `fastc-regex` | RE2-style regex (no backtracking) | `()` (pure) |
| `fastc-uuid` | UUID v4 / v7 | `rand`, `time.read` |
| `fastc-time` | Timezone-aware dates beyond `time.read` | `time.read`, `fs.read("/usr/share/zoneinfo/")` |
| `fastc-base64` | Encoding/decoding | `()` (pure) |

## One-year set

Approximately 19–34 additional packages covering the long tail: async runtime, TLS (rustls-equivalent in fastC), websocket, CSV, gzip/deflate, x509 parser, postgres client, redis client, prometheus metrics, OpenTelemetry tracing, kafka client, etc.

This is where curation discipline matters most. Many of these will be requested before they are stable. The bar:

- If a candidate package has fewer than three production users, it does not ship as `fastc-core`. It can live under `Skelf-Research/fastc-incubator` (different org, weaker promises) until it is ready.
- If a candidate has more than 5 transitive deps beyond `fastc-core`, it is rejected — either the design is wrong or the package is too ambitious for one fastc-core unit.

## `fastc.dev` — search-over-GitHub

There is no fastC registry. There is a search frontend at `fastc.dev` indexing public GitHub repos matching the `fastc-<name>` naming convention. The display, for each result:

- Package name + GitHub link.
- Capability requirements (read from the dep's `caps.json` at the latest tagged release).
- Contract discharge stats (`proven N, runtime-checked M`).
- Last audit date (read from `AUDIT.md`).
- Star count, last commit date, dependency count.
- Sigstore signature status.

The point is to surface the trust signals *at search time*, so the first "is this safe?" decision happens before `fastc add`. This is the pkg.go.dev model, adapted for capability-aware packages.

Infrastructure footprint: a static-site generator running on GitHub Actions, hitting GitHub's search API hourly. No backend, no database (other than the generated JSON indexes), no account system. Outage of `fastc.dev` does not break `fastc add` — users can still add deps by direct GitHub URL.

## Bootstrapping discipline

Every `fastc-core` package must be implemented in fastC. No falling back to "well, the HTTP client wraps a C library" except where it is genuinely the right call (sqlite, regex via RE2). When we wrap a C library, the wrapper:

- Declares the `extern "C"` interface in an `unsafe` block.
- Provides a safe fastC API above it with full annotations.
- The safe API's capability set reflects the C library's actual I/O behaviour (audited).
- The wrapper's `AGENTS.md` clearly documents "this package wraps libsqlite3; the safe layer's caps reflect what sqlite3 does to the filesystem."

This discipline ensures that fastC's safety guarantees are not undermined by sneaking C libraries in through stdlib extensions.

## Community packages — the eventual path

After 12 months and the first audit cycle, we will introduce a "community" tier:

- Packages under any GitHub org, indexed by `fastc.dev`.
- Display flagged as "community" with weaker trust signals (no Sigstore mandate, no audit record).
- Capability and contract metadata still required — these are language-level, not curation-level, and there is no way to skip them.
- `fastc add` shows the same capability prompt for community packages as for `fastc-core` packages.

The point is that the *language-level* defenses (capability typing, content hashing, no build scripts) work regardless of who curates the package. Curation adds an additional trust signal; it does not gate access to the language.

This is when fastC's package ecosystem transitions from "Go-style curated" to "the whole world can publish." The trust gradient is explicit and visible. Until then, we say no.

## Comparison to existing ecosystems

| Property | crates.io | npm | pkg.go.dev | Zon | fastC |
|----------|:---------:|:---:|:----------:|:---:|:-----:|
| Central registry | yes | yes | no | yes | **no** |
| Curation level | none (open) | none | none | none | **fastc-core curated, community tier opens at month 12** |
| Capability metadata | none | none | none | none | **mandatory** |
| Contract metadata | none | none | none | none | **mandatory** |
| Provenance mandate | opt-in | opt-in | no | no | **mandatory on fastc-core** |
| Audit records | none | none | none | none | **mandatory on fastc-core** |
| Single canonical answer per domain | no | no | partial | no | **yes (fastc-core)** |
| Number of HTTP packages | many | many | one (`net/http`) | few | **one (`fastc-http`)** |

The "single canonical answer per domain" row is what makes fastC the easiest target for coding agents. With 50,000+ crates on crates.io, an agent has to research whether to use Axum vs. Actix vs. Rocket vs. warp vs. ten others — and the "current best" answer changes every few months. With one curated `fastc-http`, the agent reads `AGENTS.md`, uses the documented idiom, and is done.

## Distribution risk

The honest gap, called out in the [roadmap](roadmap.md) front matter: fastC has zero stars at the time of this writing, and the whole curated-ecosystem strategy depends on developers wanting to use a curated ecosystem. The launch sequence in weeks 1–8 of the roadmap is the answer: ship the benchmark + manifesto + 5 core packages + coordinated launch, then measure adoption.

If `fastc-core` packages reach 1000+ stars cumulatively in the first 90 days post-launch, the strategy is working. If they do not, we reassess — possibly by opening up curation criteria sooner, possibly by changing the launch narrative.

## Open questions

- **Donation pipeline.** If a third party writes a great fastC package and offers it to `fastc-core`, what is the process? Current lean: we audit it against the eight criteria, request changes if needed, then accept under `Skelf-Research/fastc-core` with the original author as co-maintainer. Need a written contribution policy before this scales.
- **Versioning.** `fastc-core` packages will have versions. Should the version namespace be per-package (`fastc-http v1.2.0`) or unified (`fastc-core 2026.05`)? Lean: per-package, semver, but `fastc-core 2026.05` published as a "compatible set" tag for users who want to pin against a vetted combination.
- **EOL policy.** When a `fastc-core` package is superseded (e.g., the launch `fastc-http` is replaced by an async-capable version after stage 2.3), what happens to the old one? Current lean: keep the old version available, mark deprecated, set a 12-month removal horizon. Document in `AUDIT.md`.
- **`AGENTS.md` format.** This file is what coding agents read for canonical idiom. The format needs to be stable across packages so agents can rely on its structure. Need a spec — should `AGENTS.md` itself become a checked annotation surface? Lean: yes, eventually, via a separate spec; ship informal markdown for v1.
- **Funding.** Curating 30–50 packages is real work. Sustained over years, this needs paid maintainers. Strategy is out of scope for this document, but flag it as an existential question.
