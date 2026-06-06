# Context Command

The `context` command dumps the project's public type surface in a format
optimized for AI context windows. Signatures only — no function bodies — so
agents can ingest the whole shape of a fastC project without burning tokens
on implementation details.

It walks every `pub` item (functions, structs, traits) across the source
tree and prints either a compact markdown outline (the default) or a JSON
shape that mirrors `fastc explain`.

## Usage

```bash
fastc context <INPUT> [OPTIONS]
```

## Arguments

| Argument  | Description                                |
|-----------|--------------------------------------------|
| `<INPUT>` | Input FastC source file (or project root)  |

## Options

| Option              | Description |
|---------------------|-------------|
| `--format <FORMAT>` | Output format: `markdown` (default) or `json` |
| `--module <NAME>`   | Restrict output to a single module path (e.g. `vec`, `cli`) |
| `-h, --help`        | Print help |

## Markdown Output

The markdown form groups items under `## Module <path>` headers. Each item
is a bullet line; annotations and contracts hang as sub-bullets.

```
## Module `<path>`

- `fn name(params) -> Return`
  - @<annotation>
  - @purity(<level>)
  - @complexity(<bigo>)
  - @requires(<expr>)
  - @ensures(<expr>)
- `struct Name`
  - `field: Type`
- `trait Name` (N methods)
```

Items in the root (no `mod`) appear under `## Module (root)`.

## JSON Output

`--format=json` emits the same shape as [`fastc explain`](explain.md): a
top-level object with `functions` and `modules` arrays. This makes it
easy for an MCP / agent toolchain to consume `context` and `explain`
through a single parser.

## Worked Example

Project (`cli.fc`):

```c
mod cli {
    //! module = "cli"
    //! depends = ["fs"]

    pub struct Args {
        argc: i32,
        argv: ref(slice(ref(slice(u8)))),
    }

    /// Parse argv into the Args record.
    @purity(pure)
    @complexity(O(n))
    pub fn parse(argc: i32, argv: ref(slice(ref(slice(u8))))) -> Args {
        return Args { argc: argc, argv: argv };
    }
}

pub fn main() -> i32 {
    return 0;
}
```

Running:

```bash
fastc context cli.fc
```

Output:

```
# Project Surface

## Module `(root)`

- `fn main() -> i32`

## Module `cli`

- `struct Args`
  - `argc: i32`
  - `argv: ref(slice(ref(slice(u8))))`
- `fn parse(argc: i32, argv: ref(slice(ref(slice(u8))))) -> Args`
  - @purity(Pure)
  - @complexity(O(n))
```

Restricting to the `cli` module:

```bash
fastc context cli.fc --module=cli
```

drops the `(root)` section and prints only the `cli` module surface.

JSON form:

```bash
fastc context cli.fc --format=json
```

emits the same object schema as `fastc explain` — see
[Explain output schema](explain.md#output-schema).

## When to Use It

- Priming an LLM tool call with a fastC project's public surface, with
  bodies stripped so the context window stays compact.
- Reviewing API surface drift: pipe two checkouts through `context` and
  diff the markdown.
- Generating a quick "what's exported" reference for human readers without
  spinning up the full mkdocs site.

The MCP server's `context` tool wraps the exact same logic — see
[MCP Server](mcp.md).

## Token Efficiency

The markdown form intentionally:

- Drops function bodies (callers are inferred from signatures + contracts).
- Inlines parameter types instead of listing them one per line.
- Suppresses `null` annotation slots — only what's actually annotated
  shows up.
- Renders `requires` / `ensures` as single-line expressions.

In practice this lands at roughly 5-10x fewer tokens than feeding the
raw source through a tokenizer, and the LLM gets a clean structural
view of the project.

## See Also

- [Explain](explain.md) — JSON-only sibling, function-level detail
- [Diff](diff.md) — pairs with `context` for review-bot workflows
- [MCP Server](mcp.md) — exposes `context` as an MCP tool
- [Modules](../language/modules.md) — module path syntax used in headers
