# Supply Chain

This document specifies fastC's supply-chain story end to end. It is the structural rebuttal to the dominant 2025/2026 attack surface in the systems-language space.

## The threat model

We assume an attacker can:

- Publish a malicious package under a typosquatted name to any public hosting (GitHub, GitLab, registry).
- Compromise a legitimate maintainer's account on a registry (the 2025 rustfoundation.dev phishing campaign is the canonical example).
- Submit a malicious dependency update to a legitimate package whose maintainer is asleep.
- Run a watering-hole attack against developers who copy install commands from blog posts.

We do **not** assume the attacker can:

- Compromise GitHub's underlying infrastructure (an assumption we share with everyone).
- Compromise the Sigstore root key (an assumption shared with the OpenSSF ecosystem).
- Modify a content-hashed binary without changing its hash.

Against this model, the design goal is: **no arbitrary code from any third party executes during install or build of a fastC project.** Period.

## The 2025/2026 incident landscape

Every property in this design is calibrated against a real incident from the last twelve months:

- **`faster_log`, `async_println`** (mid-2025, crates.io). ~8,424 downloads. Build-time payload that read Solana/Ethereum private keys from common wallet paths and exfiltrated them. Defeats: code execution at build time. **fastC defense:** no build-time code execution.
- **`evm-units`** (late 2025, crates.io). 7,000+ downloads. Build-time delivery of OS-specific malware. **fastC defense:** no build-time code execution.
- **rustfoundation.dev phishing** (September 2025). Spoofed login page targeting crate authors, harvesting GitHub credentials with the goal of compromising legitimate publishing pipelines. **fastC defense:** no central account system to phish — packages are GitHub repos with content hashes.
- **CVE-2026-28353 (`timeapis.io`)** (early 2026, multiple registries). Typosquat of `timeapi.io`. Build script exfiltrating `.env` files from CI. **fastC defense:** no build script can run; the typosquat would fail at the content-hash check.
- **npm postinstall campaigns** (continuous). Postinstall hooks delivering crypto miners, credential stealers, ransomware loaders. **fastC defense:** there is no `postinstall` equivalent.

None of these would have succeeded against a fastC project built per this document. That is the wedge.

## The design in one paragraph

`fastc.toml` lists dependencies as git URLs + commit hashes + content hashes. `fastc fetch` clones into `vendor/` and verifies hashes. `fastc build` compiles user source + vendored deps through the same `fastc` pipeline — no build scripts, no proc macros, no postinstall. The compiler binary ships with Sigstore signatures and SLSA Level 3 provenance. `fastc add <github-url>` shows the dep's capability requirements (from its `caps.json`) before fetching, so the user reviews "this package wants `fs.read("~/.config/")` and `net.connect("api.example.com")`" rather than reading every line. There is no central registry to compromise; `fastc.dev` is a search frontend over GitHub repos matching the `fastc-<name>` convention.

## `fastc.toml` dependency format

```toml
[dependencies]
fastc-http = { git = "github.com/Skelf-Research/fastc-http", rev = "a1b2c3d4...", sha256 = "abcd1234..." }
fastc-json = { git = "github.com/Skelf-Research/fastc-json", rev = "e5f6...", sha256 = "5678..." }

# Local path deps for development
local-helper = { path = "../local-helper", sha256 = "9abc..." }
```

Properties:

- **`git` is the only fetch protocol.** No tarball URLs, no HTTP downloads, no registry resolution.
- **`rev` is a full SHA.** Branches and tags are *not* accepted; tags are mutable, branches more so. The lockfile cements the commit.
- **`sha256` is the content hash of the resolved tree.** Computed as a stable hash over the dep's source tree at the resolved commit. The compiler refuses to build if the hash does not match.
- **No semver resolution.** Each dep pins a single commit. Multiple versions of a dep in the same project are an explicit error (re-vendor or vendor under a different name). This is Go's original model and Zig's current model.
- **No transitive dependency resolution at install time.** Transitive deps are resolved at `fastc fetch` time by recursively reading the deps' own `fastc.toml`s. If any dep's transitive set conflicts, the user is told and must pick.

The conservative design — no SAT solver, no semver, single version per dep — is intentional. Dependency resolution complexity is one of the main attack surfaces in npm and Cargo. By removing it, we remove a class of bugs and a class of UX confusion that agents struggle with.

## `fastc fetch` flow

```
$ fastc fetch
Reading fastc.toml...
  fastc-http github.com/Skelf-Research/fastc-http@a1b2c3d4
  fastc-json github.com/Skelf-Research/fastc-json@e5f6...

Cloning into vendor/fastc-http... (1.2 MiB)
Verifying content hash... ✓
Cloning into vendor/fastc-json... (0.4 MiB)
Verifying content hash... ✓

Reading transitive deps...
  fastc-http depends on fastc-core (already vendored)

Done. 2 packages, 1.6 MiB vendored.
```

Failure modes are explicit:

- Hash mismatch — build fails before any source is compiled. The mismatch is logged with the expected/actual hash.
- Git ref not found — fail with a hint to verify the commit SHA.
- Conflicting transitive versions — fail with the conflicting pair listed.

After `fetch`, the `vendor/` tree is checked into the user's repo. Subsequent builds do not re-fetch — they read from `vendor/`. This is also Go's model. The supply-chain win: a clean checkout of any fastC project on a fresh machine is reproducible without network access.

## `fastc add <github-url>` — capability-aware add flow

This is the single most compelling supply-chain UX available in any language right now and the headline feature of the package system. When a user adds a dependency:

```
$ fastc add github.com/Skelf-Research/fastc-http
Fetching github.com/Skelf-Research/fastc-http (latest signed release)...

Package: fastc-http v0.4.0
Verified Sigstore signature: ✓ (signed by skelf-research/release-bot)
SLSA L3 provenance: ✓

This package requires the following capabilities:
  - net.connect(*)           — outbound network connections, any host
  - net.dns                  — DNS resolution
  - time.read                — read system time for timeouts

This package will be vendored to: vendor/fastc-http/

Continue? [y/N]
```

The capability set is read from the dep's `caps.json` (the build artifact described in [docs/capabilities.md](capabilities.md)). The user sees the full I/O surface of the dep *before* installation — not after reading every line of `build.rs`.

This compares favourably to the Rust equivalent (`cargo add foo`) where the user has no language-level idea what `foo` can do at runtime until they read 200 transitive crates of source. fastC: one prompt, one decision.

## Forbidden: executable build steps

`fastc.toml` is parsed by `fastc`. It has no Turing-complete subset. Specifically forbidden:

- **`build.rs` / `build.zig` equivalents.** There is no place in `fastc.toml` to list a build script.
- **proc macros.** fastC has no macro system. Code generation lives in the language proper (generics + monomorphization, stage 0.9), not in user-runnable plugins.
- **Postinstall hooks.** There is no install lifecycle; `fastc fetch` clones and hashes, nothing else.
- **Custom build commands.** No `[scripts]` section.

A dep that wants to ship pre-generated code must commit that code into the repo. A dep that wants to depend on a C library must declare it via `extern "C"` and link normally — same as user code.

This is the structural rebuttal: there is no code-execution surface during install or build that the user did not write themselves.

## Reproducible builds + global cache

Because there are no build scripts and no proc macros, fastC builds are bit-for-bit reproducible by construction. Same source + same `fastc` version + same target triple produces identical C output and identical binary output (modulo timestamps in the C compiler's output, which are normalized).

This enables a global build cache:

- Keyed by `(fastc_version, dep_content_hash, target_triple)`.
- Stored at `~/.cache/fastc/builds/`.
- Shared across all fastC projects on a machine.
- Verified on retrieval: hash the cached output, compare against the recorded hash.

Practical effect: a user's first build of `fastc-http` on a fresh machine takes ~5s (one time). Subsequent builds of any project depending on `fastc-http` at the same commit take 0s on the dep — pure cache hits.

This is what Nix has been trying to achieve for a decade. fastC gets it free because the inputs forbid the dynamism that breaks reproducibility.

## Compiler binary provenance: Sigstore + SLSA L3

The compiler binary itself ships with full provenance:

- **Sigstore signing.** Every `fastc` release binary is signed by the Skelf Research release pipeline using Sigstore's keyless flow. The signature is verifiable from any client.
- **SLSA Level 3 provenance.** The build pipeline produces a SLSA L3 attestation: the source repo, the commit, the builder, the build steps, the output hash. The attestation is published alongside the release.
- **Verification on install.** `fastc-install` (the bootstrap tool) verifies the Sigstore signature and SLSA attestation before extracting the binary. A binary that fails verification is rejected.

This closes the loop. The user's threat surface narrows to:

1. The Sigstore root of trust (shared with the OpenSSF ecosystem).
2. The GitHub repo hosting fastC and the deps (assumption shared with everyone).
3. The user's own source code.

There is no registry, no install-time script, no maintainer-account compromise pathway that does not surface as a Sigstore verification failure.

## `fastc.dev` as a search frontend

There is no fastC package registry. There is a search frontend — `fastc.dev` — that indexes public GitHub repos matching the `fastc-<name>` naming convention, surfaces their `caps.json` and contract discharge stats, and shows last-audit dates. It is pkg.go.dev for fastC.

Properties:

- **No accounts.** The site does not host packages. No account system means no account to compromise.
- **No package hosting.** All packages live on GitHub (or wherever their `fastc.toml` `git = ` points). The site is a search-and-display layer.
- **Capability and contract discoverability.** A user searching for "http" sees each result's required capability set, contract discharge rate, last audit date, and dependency tree before clicking through. The point is to make the *first* decision — "is this package safe?" — at the search-result level, not after fetching.

This is the pkg.go.dev model adapted for capability-aware packages. It scales because we are not running infrastructure; GitHub is.

## What about typosquats?

Typosquatting was the delivery vector for `faster_log`, `async_println`, and `timeapis.io`. fastC's defenses:

1. **The dep URL is part of the import.** A typosquat must publish under a similar URL. `github.com/Skelf-Research/fastc-http` vs `github.com/skelf-resaerch/fastc-http` is visually distinguishable in a way `faster_log` vs `fast_log` is not.
2. **The content hash pins the commit.** Even if a user accepts a typosquat URL once, they cannot accidentally update to a malicious version — the hash mismatch fails the build.
3. **Capabilities are reviewed at add time.** A typosquat's `caps.json` will reveal `net.connect(evil.com)` or `proc.spawn` requirements that the legitimate package does not have. The prompt at `fastc add` time is the choke point.
4. **`fastc.dev` flags suspicious patterns.** A package with a near-identical name to a `fastc-core` package, very few stars, or very recent first commit is flagged in search results.

None of these defenses are bulletproof against a determined attacker with patience. They raise the cost dramatically.

## What about social engineering of legitimate maintainers?

This is harder. If `fastc-http`'s legitimate maintainer is compromised and pushes a malicious commit, the content hash will change — but the user who runs `fastc update` to pick up bug fixes will fetch and accept the new hash.

Mitigations in the design:

- **`fastc update` requires Sigstore signature verification on the new commit.** A compromised maintainer would also need to compromise the Sigstore signing flow.
- **Capability diffs are surfaced.** `fastc update` shows "this update adds capability `proc.spawn` to fastc-http. Continue?" The user reviewing changes sees that a logging library should not be acquiring process-spawn rights.
- **Contract discharge rate diffs are surfaced.** If the new version drops obligations or weakens contracts, that is visible.
- **Audit tags.** Curated `fastc-core` packages publish audit records (in-repo `AUDIT.md`) listing reviewers and dates. A `fastc update` against a package with stale audit metadata warns.

Social engineering remains a residual risk. It is one shared with every modern build system; the defenses are deeper than what cargo or zon currently offer, and the capability diff is novel.

## Comparison to existing solutions

| Defense | crates.io | Zon (Zig) | npm | fastC |
|---------|:---------:|:---------:|:---:|:-----:|
| Content-hashed deps | partial (lockfile) | yes | partial | **yes (mandatory)** |
| No executable build scripts | no | no | no | **yes** |
| Capability-aware add review | no | no | no | **yes** |
| Sigstore signing | opt-in | no | no | **mandatory on fastc-core** |
| SLSA L3 provenance | opt-in | no | no | **mandatory on compiler** |
| Global registry to phish | yes | no | yes | **no — no registry** |
| Vendor by default | no | no | no | **yes** |
| Reproducible builds | partial | partial | no | **yes (by construction)** |

The point is not that any individual property is unique — content hashing exists everywhere, Sigstore is industry-standard, vendor-first is Go. The point is the *combination*: a fastC user gets every defense by default, with no opt-in, no configuration, no audit tooling to layer on top.

## Open questions

- **Mirror / archive policy.** If a GitHub repo disappears, dependent fastC projects break. Options: official archive at `archive.fastc.dev`, recommendation to vendor deps into the user's own repo, or a peer-to-peer cache layer. Decision needed before 1.7 ships; current lean is "vendor into your repo, you already have the source."
- **Private deps.** Companies will want private fastC packages. Git over SSH already works in `fastc.toml`. Capability review at `fastc add` time still works because `caps.json` is part of the dep's source. No additional infrastructure needed for v1.
- **Cross-language deps.** A fastC project depending on a C library (via FFI). The C library is *not* a fastC dep — it is a system dependency declared in `fastc.toml` as a link directive. Document this clearly so users do not try to vendor C libraries as fastC packages.
- **Build cache poisoning.** If an attacker compromises a user's `~/.cache/fastc/builds/`, they can swap cached binaries. Mitigation: cache entries are hash-verified on retrieval. The cache stores `(key, hash)` pairs and a binary whose hash does not match the recorded hash is discarded.
