# Diff Command

The `diff` command computes a semantic, AST-level diff between two fastC
sources. It reports added / removed `pub` items and signature changes —
ignoring whitespace, comment edits, and body churn so PR review hooks
only see what actually moves the API surface.

## Usage

```bash
fastc diff <OLD> <NEW> [OPTIONS]
```

## Arguments

| Argument | Description                              |
|----------|------------------------------------------|
| `<OLD>`  | Path to the previous source              |
| `<NEW>`  | Path to the new source                   |

## Options

| Option              | Description |
|---------------------|-------------|
| `--format <FORMAT>` | Output format: `markdown` (default) or `json` |
| `--include-bodies`  | Reserved for a future body-aware diff slice — currently a no-op |
| `-h, --help`        | Print help |

## Markdown Output

The markdown form has three sections, each suppressed when empty:

```
# Semantic diff

## Added (+N)
- name

## Removed (-N)
- name

## Changed (N)
- **name**
  - was: `signature`
  - now: `signature`
```

If nothing semantically changed, output collapses to:

```
# Semantic diff

_No semantic changes._
```

## JSON Output

```json
{
  "added":   ["name", "..."],
  "removed": ["name", "..."],
  "changed": [
    { "name": "name", "old": "signature", "new": "signature" }
  ]
}
```

The `signature` strings are compact textual renderings of the parameter
list and return type — same form `fastc explain` uses.

## Worked Example

Old (`v1.fc`):

```c
pub fn read(buf: mref(slice(u8))) -> usize {
    return 0;
}

pub fn close() {}
```

New (`v2.fc`):

```c
pub fn read(cap: ref(CapFsRead), buf: mref(slice(u8))) -> usize {
    return 0;
}

pub fn open(path: ref(slice(u8))) -> i32 {
    return 0;
}
```

Running:

```bash
fastc diff v1.fc v2.fc
```

Output:

```
# Semantic diff

## Added (+1)

- open

## Removed (-1)

- close

## Changed (1)

- **read**
  - was: `fn read(buf: mref(slice(u8))) -> usize`
  - now: `fn read(cap: ref(CapFsRead), buf: mref(slice(u8))) -> usize`
```

JSON form:

```bash
fastc diff v1.fc v2.fc --format=json
```

```json
{
  "added": ["open"],
  "removed": ["close"],
  "changed": [
    {
      "name": "read",
      "old": "fn read(buf: mref(slice(u8))) -> usize",
      "new": "fn read(cap: ref(CapFsRead), buf: mref(slice(u8))) -> usize"
    }
  ]
}
```

## What Counts as a Change

Today the diff considers a function "changed" when its signature summary
shifts — parameter names, parameter types, or return type. Body edits
are ignored. The diff walks `pub fn` items only; private functions are
intentionally invisible.

Future revisions will extend the signature summary to cover annotation
changes (`@purity`, `@panics`, `@requires`, `@ensures`) and module
header drift. The schema is additive — existing fields keep meaning.

## When to Use It

- **PR review hooks**: post a comment summarizing the API surface delta
  before a human reviewer reads the diff.
- **CI gates**: fail the build if `## Removed` is non-empty on a
  minor-version branch.
- **Changelog drafting**: a starting point for the "what changed" section
  of a release note.
- **Refactor verification**: ensure a pure rename / extract didn't
  accidentally shift a public signature.

## Limitations

- `--include-bodies` is reserved but currently does nothing. The body-aware
  slice will land alongside structured fixit support.
- The diff is one-file-at-a-time; project-wide diffing requires shelling
  the command per file pair (or use the MCP `diff` tool from an agent).
- Items renamed in place are reported as one Added + one Removed pair,
  not as a Renamed event. Future versions may add a heuristic match.

## See Also

- [Explain](explain.md) — per-fn JSON the signatures are derived from
- [Context](context.md) — pairs nicely with `diff` for review bots
- [MCP Server](mcp.md) — exposes `diff` as an MCP tool over JSON-RPC
