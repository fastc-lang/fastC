# fastc-core

fastc-core is the curated package set that ships alongside fastC v1.0.
Each package's public API lives in two places:

1. **In the v1.0 compiler's built-in prelude** — every fastC program
   can write `use cli::has_flag;` (or any other module) without
   installing anything.
2. **In a public preview repo at `github.com/Skelf-Research/fastc-core-<name>`**
   — these repos host the canonical API documentation and will become
   installable via `fastc add` when the v1.1 vendor-consumption flow
   ships.

Until v1.1 lands, the v1.0 prelude is the implementation; the GitHub
repos are the spec. Both surfaces are kept in lock-step so code
written against the prelude today moves to the vendor-installed
form unchanged.

## The eleven packages

| Package | Purpose | Cap |
|---|---|---|
| [`cli`](#cli) | argv access + flag parsing | none |
| [`log`](#log) | structured leveled logging | none |
| [`json`](#json) | JSON encode + integer-field decode | none |
| [`toml`](#toml) | read-only flat-table TOML | none |
| [`http`](#http) | HTTP/1.1 client | `CapNetConnect` |
| [`time`](#time) | wall-clock + ISO 8601 | `CapTimeRead` |
| [`base64`](#base64) | RFC 4648 encode/decode | none |
| [`uuid`](#uuid) | RFC 4122 v4 + parse/format | `CapRand` for v4 |
| [`crypto-primitives`](#crypto-primitives) | SHA-256, HMAC, constant-time compare | `CapRand` for `random_bytes` |
| [`regex`](#regex) | Thompson NFA, no backreferences | none |
| [`sqlite`](#sqlite) | FFI to libsqlite3 | `CapFsWrite` |

Five packages need no capability token — they are pure data transforms
or read-only views of process state. The other six gate I/O through
`Cap*` tokens minted in `main` via `caps::init()`; see
[Capabilities](capabilities.md) for the threading rules.

## `cli`

Argv access and flag parsing.

```c
use cli::count;          // () -> i32
use cli::arg_at;         // (i: i32) -> raw(u8) — null when OOB
use cli::program_name;   // () -> raw(u8)

use cli::has_flag;       // (name: raw(u8)) -> bool
use cli::flag_value;     // (name: raw(u8)) -> raw(u8) — null if absent
use cli::flag_int;       // (name: raw(u8), fallback: i32) -> i32
use cli::is_null;        // (p: raw(u8)) -> bool
```

Two flag forms — `--name=value` and `--name value`. First match wins.

```c
use cli::flag_int;
use cli::has_flag;

fn main() -> i32 {
    let n: i32 = flag_int(cstr("count"), 1);
    if (has_flag(cstr("verbose"))) {
        // verbose path
    }
    return n;
}
```

Repo: <https://github.com/Skelf-Research/fastc-core-cli>

## `log`

Structured leveled logging. Four levels (`debug` / `info` / `warn` /
`error`) plus `kv_*` helpers for typed key-value pairs.

```c
use log::debug;
use log::info;
use log::warn;
use log::error;
use log::kv_int;
use log::kv_str;
```

`kv_*` calls emit on the same line as the next level call:

```c
use log::info;
use log::kv_int;
use log::kv_str;

fn main() -> i32 {
    kv_str(cstr("user"), cstr("alice"));
    kv_int(cstr("requests"), 42);
    info(cstr("hourly stats"));
    return 0;
}
```

Produces `user="alice" requests=42 [INFO] hourly stats`.

All log functions allocate nothing — safe inside `@noalloc` regions.

Repo: <https://github.com/Skelf-Research/fastc-core-log>

## `json`

A builder-style encoder plus a small decoder slice. The v1 decoder
covers the "pull an integer out of an HTTP response" case; a full
DOM ships when there's user demand for it.

```c
use json::new_builder;
use json::obj_start;
use json::obj_end;
use json::key;
use json::int_value;
use json::str_value;

use json::find_int;     // (text: raw(u8), key: raw(u8), fallback: i64) -> i64
```

Encoding:

```c
fn render() -> Str {
    let b: JsonBuilder = new_builder();
    obj_start(addrm(b));
        key(addrm(b), cstr("id"));   int_value(addrm(b), 42);
        key(addrm(b), cstr("name")); str_value(addrm(b), cstr("alice"));
    obj_end(addrm(b));
    return (deref(addr(b))).out;
}
```

Decoding (top-level integer field):

```c
let resp: raw(u8) = cstr("{\"id\": 42, \"count\": 7}");
let id: i64 = find_int(resp, cstr("id"), cast(i64, -1));
```

Repo: <https://github.com/Skelf-Research/fastc-core-json>

## `toml`

Read-only flat-table parser scoped to the root table. Skips
`[section]` headers entirely. Honors `# ...` comments. The 80% case
is "pull a port or timeout out of `fastc.toml`-shaped config" — that
is what v1 covers.

```c
use toml::find_int;     // (text: raw(u8), key: raw(u8), fallback: i64) -> i64
use toml::find_bool;    // (text: raw(u8), key: raw(u8), fallback: bool) -> bool
```

```c
let cfg: raw(u8) = cstr("port = 8080\ndebug = true\n");
let port: i64 = find_int(cfg, cstr("port"), cast(i64, -1));     // 8080
let debug: bool = find_bool(cfg, cstr("debug"), false);          // true
```

Out of scope in v1: arrays of tables, inline tables, dotted-key
paths, date / time values, multi-line strings.

Repo: <https://github.com/Skelf-Research/fastc-core-toml>

## `http`

HTTP/1.1 client. v1 covers `GET` and the status code; bodies,
headers, methods, redirects, and TLS land in follow-up slices.

```c
use http::get_status;
// (cap: ref(CapNetConnect), host: raw(u8), port: i32, path: raw(u8)) -> i32
```

Returns the 3-digit HTTP status or `-1` on any error.

```c
use http::get_status;
use caps::init;

fn main() -> i32 {
    let bundle: Caps = init();
    let status: i32 = get_status(
        addr(bundle.net_connect),
        cstr("127.0.0.1"),
        8088,
        cstr("/"),
    );
    return status;
}
```

`CapNetConnect` is the wedge — a function without `c: ref(CapNetConnect)`
in its signature structurally cannot reach the network. The token is
minted only in `main` via `caps::init()`; library code never
fabricates it.

WASI: the runtime ships link-compatible stubs that return `-1` until
the `wasi:sockets` Preview 2 surface stabilizes.

Repo: <https://github.com/Skelf-Research/fastc-core-http>

## `time`

Wall-clock + ISO 8601.

```c
use time::now;            // (cap: ref(CapTimeRead)) -> i64 — epoch seconds
use time::now_ms;         // (cap: ref(CapTimeRead)) -> i64 — epoch milliseconds

use time::format_iso8601; // (epoch_secs: i64) -> Str — RFC 3339, always UTC
use time::parse_iso8601;  // (s: Str) -> opt(i64) — None on malformed input

use time::Duration;
use time::Duration::from_secs;   // (s: i64) -> Duration
use time::Duration::from_millis; // (ms: i64) -> Duration
```

`now` / `now_ms` need `ref(CapTimeRead)` — wall-clock reads are an
observable side channel. `format_iso8601`, `parse_iso8601`, and the
`Duration` constructors are pure.

```c
use time::now;
use time::format_iso8601;
use caps::time_read;

fn main() -> i32 {
    let t: i64 = now(&time_read);
    let stamp: Str = format_iso8601(t);
    return 0;
}
```

Both clock readers clamp negative system clocks to `0` so downstream
`i64` math is monotonic-ish even on broken hosts.

Repo: <https://github.com/Skelf-Research/fastc-core-time>

## `base64`

RFC 4648 encode / decode. Standard alphabet and URL-safe alphabet.

```c
use base64::encode;       // (bytes: slice(u8)) -> Str
use base64::decode;       // (s: Str) -> opt(Vec[u8])
use base64::encode_url;   // (bytes: slice(u8)) -> Str
use base64::decode_url;   // (s: Str) -> opt(Vec[u8])
```

| Function pair | Alphabet | Padding |
|---|---|---|
| `encode` / `decode` | §4 standard (`A-Za-z0-9+/`) | `=` required |
| `encode_url` / `decode_url` | §5 URL-safe (`A-Za-z0-9-_`) | none |

```c
use base64::encode;
use base64::decode;

fn main() -> i32 {
    let raw: slice(u8) = b"fastC v1.0";
    let s: Str = encode(raw);             // "ZmFzdEMgdjEuMA=="
    let back: opt(Vec[u8]) = decode(s);
    return 0;
}
```

Decoders return `None` on any invalid input — characters outside
the chosen alphabet, malformed padding, or truncated quanta.

Pure data transform — no capability token.

Repo: <https://github.com/Skelf-Research/fastc-core-base64>

## `uuid`

RFC 4122 v4 generation + parse / format.

```c
use uuid::Uuid;          // struct { bytes: arr(u8, 16) }

use uuid::v4;            // (cap: ref(CapRand)) -> Uuid
use uuid::nil;           // () -> Uuid
use uuid::parse;         // (s: Str) -> opt(Uuid)
use uuid::format;        // (u: Uuid) -> Str — canonical lowercase hyphenated
```

```c
use uuid::v4;
use uuid::format;
use caps::CapRand;

fn main(cap: ref(CapRand)) -> i32 {
    let u: Uuid = v4(cap);
    let s: Str = format(u);
    return 0;
}
```

`v4` needs `ref(CapRand)` — every entropy draw flows through a
capability surface and is auditable. `nil`, `parse`, and `format`
are pure.

Repo: <https://github.com/Skelf-Research/fastc-core-uuid>

## `crypto-primitives`

SHA-256, HMAC, constant-time compare, secure random.

```c
use crypto_primitives::sha256;
//   (data: slice(u8)) -> arr(u8, 32)

use crypto_primitives::hmac_sha256;
//   (key: slice(u8), data: slice(u8)) -> arr(u8, 32)

use crypto_primitives::constant_time_compare;
//   (a: slice(u8), b: slice(u8)) -> bool

use crypto_primitives::random_bytes;
//   (cap: ref(CapRand), n: usize) -> Vec[u8]
```

```c
use crypto_primitives::sha256;
use crypto_primitives::constant_time_compare;

fn main() -> i32 {
    let msg: slice(u8) = b"hello, fastc";
    let a: arr(u8, 32) = sha256(msg);
    let b: arr(u8, 32) = sha256(msg);
    if (constant_time_compare(a[..], b[..])) {
        return 0;
    }
    return 1;
}
```

`sha256`, `hmac_sha256`, `constant_time_compare` are pure.
`random_bytes` needs `ref(CapRand)` — drawing entropy without it is
a compile-time error, not a runtime one.

These primitives are intended for general-purpose use inside fastC
programs. The implementation has not been independently audited;
users with FIPS 140-3 or NIST-validation obligations should wrap a
vetted native library through fastC's FFI instead.

Repo: <https://github.com/Skelf-Research/fastc-core-crypto-primitives>

## `regex`

Thompson NFA. No backreferences. Linear-time guarantee
(`O(n * m)` in input and pattern), no catastrophic backtracking.

```c
use regex::Regex;          // opaque
use regex::Match;          // { start: usize, end: usize }
use regex::RegexError;

use regex::compile;        // (pattern: Str) -> res(Regex, RegexError)
use regex::match_one;      // (re: ref(Regex), text: Str) -> opt(Match)
use regex::match_all;      // (re: ref(Regex), text: Str) -> Vec[Match]
use regex::replace;        // (re: ref(Regex), text: Str, replacement: Str) -> Str
use regex::release;        // (re: Regex) -> void
```

Supported constructs: literals, `.`, `[a-z]`, `[^a-z]`, `^`, `$`,
`*`, `+`, `?`, `{n,m}`, `(...)`, `|`. Anything else is a
`RegexError` from `compile`.

```c
use regex::compile;
use regex::match_all;
use regex::release;

fn main() -> i32 {
    let re: Regex = compile(cstr("[a-z]+"))?;
    let hits: Vec[Match] = match_all(ref(re), cstr("the quick brown fox"));
    release(re);
    return 0;
}
```

Backreferences are deliberately out of scope — they turn matching
into an NP-hard problem and are the source class behind every
production ReDoS incident. Code that genuinely needs them can call
PCRE through FFI and pay the variance explicitly.

Pure — no capability token.

Repo: <https://github.com/Skelf-Research/fastc-core-regex>

## `sqlite`

FFI to system libsqlite3. Pass `-lsqlite3` is handled by the build
driver when any module imports `sqlite::*`.

```c
use sqlite::Db;
use sqlite::Cursor;
use sqlite::Row;
use sqlite::SqliteError;

use sqlite::open;       // (path: Str, cap: ref(CapFsWrite)) -> res(Db, SqliteError)
use sqlite::exec;       // (db: ref(Db), sql: Str) -> res(i32, SqliteError)
use sqlite::query;      // (db: ref(Db), sql: Str) -> res(Cursor, SqliteError)
use sqlite::next;       // (cursor: mref(Cursor)) -> opt(Row)
use sqlite::get_int;    // (row: ref(Row), col: i32) -> i64
use sqlite::get_text;   // (row: ref(Row), col: i32) -> Str
use sqlite::close;      // (db: Db) -> void
```

```c
use sqlite::open;
use sqlite::exec;
use sqlite::query;
use sqlite::next;
use sqlite::get_text;
use sqlite::close;
use caps::init;

fn main() -> i32 {
    let bundle: Caps = init();
    let db: Db = open(":memory:", addr(bundle.fs_write))?;
    exec(addr(db), "CREATE TABLE users (id INTEGER, name TEXT)")?;
    exec(addr(db), "INSERT INTO users VALUES (1, 'ada')")?;

    let mut cur: Cursor = query(addr(db), "SELECT id, name FROM users")?;
    loop {
        match next(mref(cur)) {
            Some(row) => { /* read row */ }
            None => break,
        }
    }
    close(db);
    return 0;
}
```

`open` requires `ref(CapFsWrite)` — never `CapFsRead`. SQLite writes
to a rollback journal even for read-only queries the moment a
transaction needs durability, so the cap gate trips at `open` to
match the engine's real I/O profile.

Repo: <https://github.com/Skelf-Research/fastc-core-sqlite>

## How to consume — today

In v1.0, every fastc-core module lives in the prelude. Imports
just work:

```c
use cli::has_flag;
use log::info;
use json::find_int;
```

No `fastc.toml` entries. No lockfile churn. No `fastc add` step.
The compiler binary is the implementation, and the public preview
repos document the spec.

## How to consume — v1.1 vendor flow

When the `fastc add` consumption flow ships in v1.1, `fastc.toml`
will accept:

```toml
[dependencies]
fastc-core-cli = { git = "https://github.com/Skelf-Research/fastc-core-cli", rev = "v0.1.0", sha256 = "..." }
fastc-core-http = { git = "https://github.com/Skelf-Research/fastc-core-http", rev = "v0.1.0", sha256 = "..." }
```

The `rev` pins a tag or commit; the `sha256` is the integrity
digest recorded in `fastc.lock` and re-verified on every build.
The exact same `use cli::has_flag;` source compiles against either
the prelude-bundled v1.0 implementation or the v1.1 vendored package
without source changes.

## Why "one curated answer per domain"

fastC commits to one curated stdlib answer per domain instead of
an ecosystem with eleven competing JSON libraries. The reasoning
is concrete:

1. **Decision-load drops to zero.** A new fastC program needs a
   logger, a JSON encoder, a TOML config reader, and an HTTP
   client. The answer is `log`, `json`, `toml`, `http`. There is
   no shortlist to evaluate, no benchmark spreadsheet to maintain,
   no Reddit thread to read.
2. **LLM-write paths simplify.** When the model has one correct
   import for "open a sqlite database", code generation collapses
   to a single path. The error surface narrows from "did the model
   pick the right crate" to "did the model pick the right
   function".
3. **Capability audits stay feasible.** Every fastc-core package
   declares its `Cap*` requirements up front in this table.
   Auditing the I/O surface of a fastC program is a finite,
   tractable task; auditing the I/O surface of a Cargo dependency
   tree is not.

The trade-off is real — a domain whose 80% case isn't covered by
the v1 surface (JSON streaming, regex backreferences, TLS) needs a
follow-up package or an FFI call. The curated set is the floor, not
the ceiling.

## Cross-links

- [Capabilities](capabilities.md) — `CapNetConnect`, `CapTimeRead`,
  `CapFsWrite`, `CapRand` and the fabrication-check rules
- [Modules](modules.md) — `use mod::item;` imports and the v1.3
  header surface
- CLI: [`fastc add`](../cli/add.md) — the v1.1 vendor consumption flow
- CLI: [`fastc lock`](../cli/lock.md) — `fastc.lock` integrity surface
