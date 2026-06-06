# Fix Command

The `fix` command applies mechanical fixes to a fastC source file. Today it
runs the universal mechanical fix — `fastc fmt` — and exposes the
infrastructure for per-diagnostic structured fix-its from `fastc check`.

The Fix-it infrastructure (`Fixit` struct + applier) ships in v1.x; the
per-diagnostic backfill — wrap-in-unsafe, add-missing-use, missing-semicolon
suggestions — is incremental work landing one site at a time.

## Usage

```bash
fastc fix <INPUT> [OPTIONS]
```

## Arguments

| Argument  | Description                   |
|-----------|-------------------------------|
| `<INPUT>` | Input FastC source file (.fc) |

## Options

| Option                          | Description |
|---------------------------------|-------------|
| `--dry-run`                     | Print the diff to stdout without writing changes back |
| `--output-format <FORMAT>`      | `text` (default) or `json` — the structured envelope |
| `-h, --help`                    | Print help |

## Behavior

`fastc fix` runs the formatter pass over the input and compares the result
with the on-disk text:

- If the file is already up-to-date, prints `<path>: already up-to-date.`
  to stderr and exits 0.
- With `--dry-run`, prints a unified-style line diff and exits 0 without
  touching the file.
- Otherwise, overwrites the input file with the fixed text and prints
  `<path>: applied fixes.` to stderr.

The JSON output format wraps the same operation in a structured envelope
that editor integrations and the MCP `fix` tool consume directly.

## JSON Envelope

```json
{
  "status": "ok",
  "applied": [
    {
      "label": "fmt: normalize whitespace",
      "file": "src/main.fc",
      "span": { "start": 0, "end": 42 }
    }
  ],
  "skipped": []
}
```

| Field     | Type   | Description |
|-----------|--------|-------------|
| `status`  | string | `ok`, `noop`, or `error` |
| `applied` | array  | Per-fixit summary: human label, file path, byte span replaced |
| `skipped` | array  | Fixits that overlapped or fell outside source bounds |

When the input is already canonical, the envelope reports
`"status": "noop"` with empty `applied` / `skipped` arrays.

## Worked Example

A file with sloppy formatting (`bad.fc`):

```c
fn add(a:i32,b:i32)->i32{return a+b;}
```

Dry-run:

```bash
fastc fix bad.fc --dry-run
```

Output:

```
--- bad.fc
+++ bad.fc (after fastc fix)
- 1: fn add(a:i32,b:i32)->i32{return a+b;}
+ 1: fn add(a: i32, b: i32) -> i32 {
```

Applying the fix:

```bash
fastc fix bad.fc
```

prints `bad.fc: applied fixes.` to stderr, and the file is now:

```c
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

JSON envelope of the same run:

```bash
fastc fix bad.fc --output-format=json
```

```json
{
  "status": "ok",
  "applied": [
    { "label": "fmt: normalize source", "file": "bad.fc", "span": { "start": 0, "end": 37 } }
  ],
  "skipped": []
}
```

## How Structured Fix-its Work

The infrastructure lives in `crates/fastc/src/fixit.rs`. A `Fixit` carries
three things:

```rust
pub struct Fixit {
    pub span: Span,            // byte range to replace
    pub replacement: String,   // text to substitute (may be empty)
    pub label: String,         // human-readable description
}
```

`apply_all` sorts a batch by span end descending so earlier edits don't
shift later spans, then applies them in order. Overlapping fixits skip
the later one — the first wins. This means a diagnostic emitter can opt
into structured fixits by attaching `Some(Fixit { ... })` to its
`.with_help(...)` site, and `fastc fix` will pick them up automatically.

## When to Use It

- **Pre-commit hook**: drop `fastc fix --dry-run` into `pre-commit` and
  fail the commit if formatting drifted.
- **Editor save-on-format**: editors can invoke `fastc fix --output-format=json`
  on save and apply the returned spans without re-reading the file.
- **CI gates**: pair `fastc fix --dry-run` with `fastc fmt --check` so
  the build catches both formatting drift and structured fix-it
  opportunities.
- **Agent loops**: the MCP `fix` tool wraps this command so an LLM can
  request a mechanical cleanup pass between diagnose / re-check cycles.

## Limitations

- Today's universal fix is formatter-only. Per-diagnostic Fixits land
  incrementally — the v1.x infrastructure ships, the backfill follows.
- `apply_all` is whole-file; the diff is line-aligned, not character-
  precise. For surgical edits, the MCP `fix` tool returns spans the
  editor can apply directly.

## See Also

- [Annotations](../language/annotations.md) — annotation surface the structured envelope reflects
- [MCP Server](mcp.md) — exposes `fix` as an MCP tool over JSON-RPC
- [Compile](compile.md) — the `fastc fmt` companion this command wraps
