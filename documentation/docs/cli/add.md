# Add Command

The `add` command fetches a dependency from a git URL, inspects its capability
surface, verifies its content hash, and records it in `fastc.toml` plus
`fastc.lock`.

fastC is vendor-first: there is no central registry. Every dependency is a git
URL pinned to a commit and anchored by a tree sha256. `fastc add` is the
front door for that workflow.

## Usage

```bash
fastc add <URL> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<URL>` | Git URL to fetch (e.g. `https://github.com/fastc-lang/fastc-core-cli` or `git@github.com:org/repo`) |

## Options

| Option | Description |
|--------|-------------|
| `--rev <REV>` | Pin to a specific git commit or tag. If omitted, the resolved HEAD of the default branch is recorded. |
| `--name <NAME>` | Override the dep entry name in `fastc.toml`. Defaults to the dep's own `[package].name` value. |
| `--yes` | Skip the interactive confirmation prompt. Useful for CI and scripted setups; do not use as a default. |
| `-h, --help` | Print help |

## Behavior

`fastc add` runs through a fixed sequence:

1. **Fetch.** Clones the URL into the shared cache under
   `~/Library/Caches/fastc/` (macOS) or `~/.cache/fastc/` (Linux). If a
   `--rev` is supplied, that commit is checked out. Otherwise the resolved
   HEAD is recorded as the rev.
2. **Probe the manifest.** The dep itself must ship a `fastc.toml`. `fastc
   add` reads the dep's `[package].name` and `[package].version` so the
   recorded entry reflects what the dep calls itself.
3. **Compute the content hash.** Walks the fetched tree (sans `.git/`),
   hashes the contents, and prints the lowercase hex sha256.
4. **Scan the capability surface.** Greps the dep's `.fc` files for
   `ref(Cap*)` and `mref(Cap*)` positions. The set of `Cap*` types it
   references is the capability surface — what the dep can do once you've
   passed it the right tokens. Surface-level scan, not full parsing, so it
   works even on deps with compile errors and errs toward over-warning.
5. **Confirm.** Prints the package metadata, resolved rev, sha256, and
   capabilities, then asks `Add `<name>` to fastc.toml? [y/N]`. High-impact
   capabilities (`CapNetConnect`, `CapProcSpawn`, `CapFsWrite`) trigger an
   extra warning line. Skipped under `--yes`.
6. **Write `fastc.toml`.** Appends the new entry under `[dependencies]`
   with `git`, `rev`, and `sha256` fields populated.
7. **Update `fastc.lock`.** Runs the equivalent of `fastc lock` so the
   new dep — and every existing dep — ends up anchored against the cached
   tree.

## Worked Example

```bash
fastc add https://github.com/fastc-lang/fastc-core-cli --rev v0.1.0
```

Console output:

```
Adding dependency from https://github.com/fastc-lang/fastc-core-cli
  fetched to /Users/you/Library/Caches/fastc/git/fastc-core-cli@v0.1.0

  package: fastc-core-cli 0.1.0
  git:     https://github.com/fastc-lang/fastc-core-cli
  rev:     v0.1.0
  sha256:  9b1f3c5e7d2a8b4f6c0e1d2a3b4c5d6e7f8091a2b3c4d5e6f7a8b9c0d1e2f3a4b
  caps:    CapEnvRead, CapFsRead

Add `fastc-core-cli` to fastc.toml? [y/N] y
Updated /Users/you/proj/fastc.toml
Locking dependency: fastc-core-cli
  sha256: 9b1f3c5e7d2a8b4f6c0e1d2a3b4c5d6e7f8091a2b3c4d5e6f7a8b9c0d1e2f3a4b
Updated fastc.lock
```

After `fastc add`, the `[dependencies]` table looks like:

```toml
[dependencies]
fastc-core-cli = { git = "https://github.com/fastc-lang/fastc-core-cli", rev = "v0.1.0", sha256 = "9b1f3c5e7d2a8b4f6c0e1d2a3b4c5d6e7f8091a2b3c4d5e6f7a8b9c0d1e2f3a4b" }
```

And `fastc.lock` gains a matching entry:

```toml
[[package]]
name = "fastc-core-cli"
version = "0.1.0"
source = "git+https://github.com/fastc-lang/fastc-core-cli?rev=v0.1.0"
resolved = "v0.1.0"
sha256 = "9b1f3c5e7d2a8b4f6c0e1d2a3b4c5d6e7f8091a2b3c4d5e6f7a8b9c0d1e2f3a4b"
```

## Naming a Dep Differently

If two upstreams ship packages with the same `[package].name`, use `--name`:

```bash
fastc add https://github.com/forks/fastc-core-cli --rev v0.2.0-fork --name fastc-core-cli-fork
```

The entry in `fastc.toml` will key on `fastc-core-cli-fork`, leaving the
upstream `fastc-core-cli` slot free.

## Non-Interactive Use

For CI and scripted bootstraps, pass `--yes`:

```bash
fastc add https://github.com/fastc-lang/fastc-core-cli --rev v0.1.0 --yes
```

The capability summary is still printed to stderr — it's recorded in the CI
log even when no human is at the prompt — but no input is required.

## Refusal Conditions

`fastc add` will refuse to proceed if:

- The current directory (or any ancestor) has no `fastc.toml`.
- The fetched dep has no `fastc.toml` of its own. A dep without a manifest
  can't declare a name, version, or capability surface, so there's nothing
  honest to record.
- The git fetch itself fails (network, auth, bad URL).

It does **not** refuse on missing sigstore bundles or high-impact
capabilities — those are flagged but ultimately your call. The strict
posture is `fastc build --vendor-strict`, which turns missing integrity
fields into hard errors at build time.

## See Also

- [Lock](lock.md) — re-anchoring and verifying the integrity of every dep.
- [fastc-core](../language/fastc-core.md) — the standard library shipped as
  individually-addable git deps.
