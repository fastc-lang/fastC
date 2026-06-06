# Explain Command

The `explain` command emits a machine-readable JSON summary of every function in
a fastC source file. It is the Stage 1.6 agent-facing artifact, designed for
Claude Code / Cursor / Codex consumption without re-parsing the source.

For each function it reports the signature, capability tokens, contracts,
v1.3 annotations, and doc comments. A top-level `modules` array carries any
`//!` module headers found in inline `mod` declarations.

## Usage

```bash
fastc explain <INPUT>
```

## Arguments

| Argument  | Description                   |
|-----------|-------------------------------|
| `<INPUT>` | Input FastC source file (.fc) |

## Options

`fastc explain` takes no flags today. The schema is fixed at one shape
(JSON to stdout) so agent integrations can pipe the output without
parsing flags.

## Output Schema

The top level is a single JSON object with two arrays:

```json
{
  "functions": [ ... ],
  "modules":   [ ... ]
}
```

### Function entries

| Field          | Type            | Description |
|----------------|-----------------|-------------|
| `name`         | string          | Function name as written |
| `module`       | string \| null  | Dotted module path (`foo::bar`) or `null` for the root |
| `params`       | array           | List of `{name, type}` objects |
| `return`       | string          | Return type, rendered as fastC text |
| `annotations`  | array of string | Free-form `@annotation` names |
| `caps`         | array of string | Capability tokens this function accepts (`CapFsRead`, `CapNetConnect`, ...) |
| `requires`     | array of string | `@requires(...)` clauses, rendered as expressions |
| `ensures`      | array of string | `@ensures(...)` clauses, rendered as expressions |
| `mem`          | string \| null  | `arena=NAME` when `@mem(arena=...)` is set, else `null` |
| `panics`       | string \| null  | `never`, `always`, `on(expr)`, or `null` |
| `purity`       | string \| null  | `pure`, `effect`, `io`, or `null` |
| `complexity`   | string \| null  | Big-O class (`O(1)`, `O(n)`, `O(n log n)`, ...) or `null` |
| `is_test`      | bool            | True for `#[test]` functions |
| `is_unsafe`    | bool            | True for `unsafe fn` |
| `doc_comments` | array of string | `///` lines attached to this function |

### Module entries

A module entry appears for every inline `mod` declaration that carries a
`//!` header. Bodies without a header are skipped.

| Field        | Type            | Description |
|--------------|-----------------|-------------|
| `path`       | string          | Dotted module path |
| `module`     | string \| null  | Header `module = "..."` value |
| `owns`       | array of string | `owns` list |
| `arch`       | string \| null  | Architecture tag |
| `depends`    | array of string | `depends` list |
| `threading`  | string \| null  | Threading model tag |
| `invariants` | array of string | Free-form invariant strings |

## Stability

The schema is **additive**: new fields may appear in future releases, but
existing fields keep their meaning. Agents should treat unknown fields as
ignorable and use `null` (or an empty array) as the absence sentinel.

## Worked Example

Source (`fs.fc`):

```c
mod fs {
    //! owns = ["fd"]
    //! invariants = ["fd >= 0"]

    /// Read bytes from a capability-gated handle.
    @purity(io)
    @complexity(O(n))
    pub fn read(cap: ref(CapFsRead), buf: mref(slice(u8))) -> usize
        requires(buf.len > 0)
    {
        return 0;
    }
}
```

Running:

```bash
fastc explain fs.fc
```

Output:

```json
{
  "functions": [
    {
      "name": "read",
      "module": "fs",
      "params": [
        { "name": "cap", "type": "ref(CapFsRead)" },
        { "name": "buf", "type": "mref(slice(u8))" }
      ],
      "return": "usize",
      "annotations": [],
      "caps": ["CapFsRead"],
      "requires": ["(buf.len > 0)"],
      "ensures": [],
      "mem": null,
      "panics": null,
      "purity": "io",
      "complexity": "O(n)",
      "is_test": false,
      "is_unsafe": false,
      "doc_comments": [" Read bytes from a capability-gated handle."]
    }
  ],
  "modules": [
    {
      "path": "fs",
      "module": null,
      "owns": ["fd"],
      "arch": null,
      "depends": [],
      "threading": null,
      "invariants": ["fd >= 0"]
    }
  ]
}
```

## When to Use It

- Feeding a fastC project surface into an LLM tool call.
- Building editor integrations that need fn signatures + contracts without
  re-implementing the parser.
- Driving CI checks: e.g. fail the build if a `pub fn` lost its
  `@requires` clause between commits.

## See Also

- [Context](context.md) — markdown / JSON dump of the project's pub surface
- [MCP Server](mcp.md) — exposes `explain` as an MCP tool over JSON-RPC
- [Annotations](../language/annotations.md) — the annotation grammar the schema mirrors
- [Modules](../language/modules.md) — `//!` module header reference
