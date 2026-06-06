# C interop

## Does fastC ingest C source?

No. fastC emits C; it does not parse C. The deliberate trade is that ingesting arbitrary C would require trusting arbitrary C, undermining the capability and provenance story. fastC integrates with C libraries via explicit header declarations (`extern "C"`), not by reading their source.

Zig is better than fastC at consuming arbitrary C — `@cImport` will pull in a system header and let you call its functions without writing FFI declarations. We accept that loss on purpose because every byte of C that gets compiled into a fastC binary is a byte that bypasses fastC's invariants. The `extern "C"` boundary is the explicit handshake point.

## How do I call a C library?

Declare the functions you need inside an `extern "C"` block:

```fastc
extern "C" {
    unsafe fn fopen(path: raw(u8), mode: raw(u8)) -> rawm(u8);
    unsafe fn fread(ptr: rawm(u8), size: usize, nmemb: usize, stream: rawm(u8)) -> usize;
    unsafe fn fclose(stream: rawm(u8)) -> i32;
}
```

Then wrap them with a safe API that takes the relevant capability:

```fastc
fn read_file(_c: ref(CapFsRead), path: raw(u8)) -> opt(Str) {
    unsafe {
        // ... wrap fopen/fread/fclose
    }
}
```

The cap argument is unused at runtime — the compiler erases it. But its presence in the type signature is what forces the caller to hold the capability.

## How do I expose a fastC API to C?

`fastc compile prog.fc -o prog.c --emit-header` produces `prog.h` alongside the `.c` file. That header declares every `pub fn` from your module with C linkage. Drop it into a C project and call the functions as if they were native C.

## What about C++? Objective-C?

Out of scope. fastC emits C11. Wrap a C++ library in an `extern "C"` shim if you need it.

## A real worked example

[`fastc-core-sqlite`](../language/fastc-core.md#sqlite) is the canonical fastC-to-C FFI binding. It opens a libsqlite3 handle through an `extern "C"` block, wraps the API with cap-typed safe entry points (`open(path: Str, c: ref(CapFsWrite)) -> res(Db, SqliteError)`), and runs queries through the typed `Cursor` interface. The opaque-pointer wrapper lives in `runtime/sqlite_shim.h`. Read the package's [public surface](https://github.com/Skelf-Research/fastc-core-sqlite) for the full pattern.

The same shape applies to any C library: `extern "C"` declarations, an `unsafe` wrapper, a cap-typed public API. The cost is structural — every C dep is one `extern "C"` block; every external effect needs a cap.
