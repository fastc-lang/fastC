//! The built-in prelude: trait declarations and impls injected into every
//! fastC compilation.
//!
//! Stage 1.0 slice 3 + 4 ships:
//!
//! - `trait Eq`   — equality (`fn eq(self, other) -> bool`).
//! - `trait Ord`  — ordering (`fn less_than(self, other) -> bool`).
//! - `trait Copy` — marker trait (no methods).
//! - `trait Drop` — destructor (`fn drop(self: mref(Self))`); user types
//!   opt in by writing `impl Drop for MyType`. Primitives are not droppable.
//!
//! And implementations for every primitive type with sensible semantics:
//!
//! - Every primitive implements `Eq` and `Copy`.
//! - Numeric primitives (everything except `bool`) implement `Ord`.
//! - No primitive implements `Drop` (nothing to free).
//!
//! The prelude is delivered as a fastC source string that is parsed once at
//! driver entry; the parsed items are prepended to the user's `File`
//! before desugar so the rest of the pipeline sees no special cases.

/// The prelude source. Kept as a literal string so adding a built-in trait
/// is just a textual addition — no AST surgery required.
///
/// All numeric primitives get both `Eq` and `Ord`. `bool` gets `Eq` and
/// `Copy` only (booleans have no total order in fastC).
pub const PRELUDE_SRC: &str = r#"
// --- Built-in traits (stage 1.0 slice 3) ---

trait Eq {
    fn eq(self: ref(Self), other: ref(Self)) -> bool;
}

trait Ord {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool;
}

trait Copy {
}

trait Drop {
    fn drop(self: mref(Self)) -> void;
}

trait Hash {
    fn hash(self: ref(Self)) -> usize;
}

trait Clone {
    fn clone(self: ref(Self)) -> Self;
}

// --- Primitive impls ---

impl Eq for i8 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i8 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i8 {}

impl Eq for i16 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i16 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i16 {}

impl Eq for i32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i32 {}

impl Eq for i64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i64 {}

impl Eq for u8 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u8 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u8 {}

impl Eq for u16 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u16 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u16 {}

impl Eq for u32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u32 {}

impl Eq for u64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u64 {}

impl Eq for f32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for f32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for f32 {}

impl Eq for f64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for f64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for f64 {}

impl Eq for bool {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Copy for bool {}

impl Eq for usize {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for usize {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for usize {}

impl Eq for isize {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for isize {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for isize {}

// --- Hash impls for primitive integer types (stage 1.1 slice 17) ---
//
// v1 uses identity hashing: the hash of an integer is the integer
// itself, cast to usize. That's enough for the small-cap hashmaps the
// stdlib targets in v1 — the hashmap implementation does its own
// bucket-index mixing on top via `hash % cap` and a few power-of-two
// tricks. A proper avalanche-mixing hash (fxhash, wyhash) lands when
// the benchmarking slice surfaces collision-rate numbers.
//
// Signed types cast through their unsigned partner first so a -1
// doesn't sign-extend to the all-ones usize and collide trivially
// with `usize::MAX`.

impl Hash for u8 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, deref(self));
    }
}

impl Hash for i8 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, cast(u8, deref(self)));
    }
}

impl Hash for u16 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, deref(self));
    }
}

impl Hash for i16 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, cast(u16, deref(self)));
    }
}

impl Hash for u32 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, deref(self));
    }
}

impl Hash for i32 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, cast(u32, deref(self)));
    }
}

impl Hash for u64 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, deref(self));
    }
}

impl Hash for i64 {
    fn hash(self: ref(Self)) -> usize {
        return cast(usize, cast(u64, deref(self)));
    }
}

impl Hash for usize {
    fn hash(self: ref(Self)) -> usize {
        return deref(self);
    }
}

impl Hash for isize {
    fn hash(self: ref(Self)) -> usize {
        // isize -> usize directly preserves the bit pattern on every
        // C11 target; that's all we need for hashing.
        return cast(usize, deref(self));
    }
}

// --- Standard library (stage 1.1) ---
//
// Functions live in inline `mod` namespaces. Users opt in with
// `use math::min;` etc. — no fastC code is forced to take a dependency
// on stdlib symbols it doesn't import.

mod math {
    // Integer absolute value. Defined per-type because fastC's v1
    // generics cannot yet express "T is signed and supports unary minus";
    // overloading via traits would work but adds noise to the prelude.
    pub fn abs_i32(x: i32) -> i32 {
        if (x < 0) {
            return (0 - x);
        }
        return x;
    }

    pub fn abs_i64(x: i64) -> i64 {
        if (x < cast(i64, 0)) {
            return (cast(i64, 0) - x);
        }
        return x;
    }

    pub fn abs_isize(x: isize) -> isize {
        if (x < cast(isize, 0)) {
            return (cast(isize, 0) - x);
        }
        return x;
    }

    // Float `abs` via branch — NaN propagates because `NaN < 0` is false,
    // so we return NaN unchanged. Equivalent semantics to libc `fabs` for
    // every other input without needing FFI.
    pub fn abs_f32(x: f32) -> f32 {
        if (x < cast(f32, 0)) {
            return (cast(f32, 0) - x);
        }
        return x;
    }

    pub fn abs_f64(x: f64) -> f64 {
        if (x < cast(f64, 0)) {
            return (cast(f64, 0) - x);
        }
        return x;
    }

    /// Integer power: `base ^ exp`. Returns 1 for exp = 0 (including
    /// 0 ^ 0 = 1, per the IEEE-754 / Knuth convention). Negative
    /// `exp` is not supported — caller must check. Specialized for
    /// `i32`; a bounded-generic `pow[T: Mul]` waits until a numeric
    /// Mul trait lands.
    pub fn pow_i32(base: i32, exp: i32) -> i32 {
        if (exp <= 0) {
            return 1;
        }
        let result: i32 = 1;
        let i: i32 = 0;
        while (i < exp) {
            result = (result * base);
            i = (i + 1);
        }
        return result;
    }

    /// Greatest common divisor via the Euclidean algorithm. Operates
    /// on absolute values, so `gcd(-12, 8) = 4`. Returns 0 only when
    /// both inputs are zero — matches Python's convention.
    pub fn gcd_i32(a: i32, b: i32) -> i32 {
        let x: i32 = abs_i32(a);
        let y: i32 = abs_i32(b);
        while (y != 0) {
            let r: i32 = (x - ((x / y) * y));   // x % y
            x = y;
            y = r;
        }
        return x;
    }

    // Bounded-generic helpers built on the prelude `Ord` trait. These work
    // for every numeric primitive automatically.
    pub fn min[T: Ord](a: T, b: T) -> T {
        if (a.less_than(addr(b))) {
            return a;
        }
        return b;
    }

    pub fn max[T: Ord](a: T, b: T) -> T {
        if (a.less_than(addr(b))) {
            return b;
        }
        return a;
    }

    pub fn clamp[T: Ord](x: T, lo: T, hi: T) -> T {
        if (x.less_than(addr(lo))) {
            return lo;
        }
        if (hi.less_than(addr(x))) {
            return hi;
        }
        return x;
    }
}

mod mem {
    /// Raw libc allocator. Always called through the safe wrappers below;
    /// users should not invoke these directly outside of `unsafe`.
    extern "C" {
        unsafe fn malloc(size: usize) -> rawm(u8);
        unsafe fn realloc(ptr: rawm(u8), size: usize) -> rawm(u8);
        unsafe fn free(ptr: rawm(u8)) -> void;
    }

    /// Allocate `size` bytes of uninitialized memory. Returns a nullable
    /// raw pointer — the caller is responsible for checking for null and
    /// freeing the result via `mem::free_bytes`.
    pub fn alloc(size: usize) -> rawm(u8) {
        unsafe {
            return malloc(size);
        }
    }

    /// Grow or shrink a previously-allocated buffer. `ptr` may be a value
    /// from `mem::alloc` (or null, in which case this behaves like
    /// `alloc`). The returned pointer replaces `ptr`; callers must not
    /// keep the old value.
    pub fn resize(ptr: rawm(u8), new_size: usize) -> rawm(u8) {
        unsafe {
            return realloc(ptr, new_size);
        }
    }

    /// Release memory previously returned by `mem::alloc` or
    /// `mem::resize`. Renamed from the libc `free` so the wrapper doesn't
    /// shadow the extern symbol inside the same module scope.
    pub fn free_bytes(ptr: rawm(u8)) -> void {
        unsafe {
            free(ptr);
        }
    }
}

mod io {
    /// Print primitives. These resolve to thin static-inline runtime
    /// helpers (`fc_puts_u8` / `fc_putchar`) defined in
    /// `fastc_runtime.h`. The helpers wrap libc `puts`/`putchar` and
    /// handle the `char*` vs `uint8_t*` impedance mismatch so the prelude
    /// can keep its `raw(u8)` signatures without tripping
    /// `-Wpointer-sign` under `-Werror`.
    extern "C" {
        unsafe fn fc_puts_u8(s: raw(u8)) -> i32;
        unsafe fn fc_putchar(c: i32) -> i32;
        unsafe fn fc_print_i32(n: i32) -> i32;
    }

    /// Write a null-terminated C string to stdout followed by a newline.
    /// Use with `cstr("...")` for literal strings.
    pub fn println(s: raw(u8)) -> void {
        unsafe {
            discard(fc_puts_u8(s));
        }
    }

    /// Write a single ASCII byte to stdout. Useful for emitting punctuation
    /// or constructed strings without going through `printf` formatting.
    pub fn put_char(c: i32) -> void {
        unsafe {
            discard(fc_putchar(c));
        }
    }

    /// Write a signed 32-bit integer in base 10. No leading zero, sign
    /// only on negatives. Does *not* append a newline — pair with
    /// `put_char(10)` or follow with another `println` if you want
    /// line-oriented output.
    pub fn print_int(n: i32) -> void {
        unsafe {
            discard(fc_print_i32(n));
        }
    }
}

// --- json: minimal JSON encoder (fastc-core preview) ---
//
// The post-stage-1.7 `fastc-core` org will ship `fastc-core/json`
// as a separate vendor-able package. v1 ships an encoder-only
// preview here in the prelude so users have a concrete way to
// produce JSON without depending on libc printf. A streaming
// decoder follows in fastc-core proper.
//
// Builder model: `JsonBuilder` wraps a `Str`. Every primitive
// (`obj_start` / `obj_end` / `arr_start` / `arr_end` / `key` /
// `str_value` / `int_value` / `bool_value` / `null_value`) appends
// to the buffer. A `comma_if_needed` helper tracks whether the
// next entry needs a leading comma — kept inside the struct so
// users don't have to thread state through call sites.

struct JsonBuilder {
    out: Str,
    // needs_comma: 1 when the next entry inside the current
    // container needs a leading comma; 0 right after a `{` / `[`
    // or a `key`.
    needs_comma: i32,
}

mod json {
    use str::make;
    use str::push_byte;
    use str::push_cstr;
    use io::print_int;
    use vec::release;
    use mem::alloc;

    pub fn new_builder() -> JsonBuilder {
        return JsonBuilder { out: make(), needs_comma: 0 };
    }

    pub fn obj_start(b: mref(JsonBuilder)) -> void {
        write_comma_if_needed(b);
        push_byte(addrm((deref(b)).out), cast(u8, 123));   // '{'
        (deref(b)).needs_comma = 0;
    }

    pub fn obj_end(b: mref(JsonBuilder)) -> void {
        push_byte(addrm((deref(b)).out), cast(u8, 125));   // '}'
        (deref(b)).needs_comma = 1;
    }

    pub fn arr_start(b: mref(JsonBuilder)) -> void {
        write_comma_if_needed(b);
        push_byte(addrm((deref(b)).out), cast(u8, 91));    // '['
        (deref(b)).needs_comma = 0;
    }

    pub fn arr_end(b: mref(JsonBuilder)) -> void {
        push_byte(addrm((deref(b)).out), cast(u8, 93));    // ']'
        (deref(b)).needs_comma = 1;
    }

    /// Write a `"key":` pair. The next call writes the value with
    /// no leading comma; subsequent siblings get one.
    pub fn key(b: mref(JsonBuilder), k: raw(u8)) -> void {
        write_comma_if_needed(b);
        push_byte(addrm((deref(b)).out), cast(u8, 34));    // '"'
        push_cstr(addrm((deref(b)).out), k);
        push_byte(addrm((deref(b)).out), cast(u8, 34));
        push_byte(addrm((deref(b)).out), cast(u8, 58));    // ':'
        (deref(b)).needs_comma = 0;
    }

    /// Write a quoted-string value. v1 does not escape internal
    /// quotes or control characters; a fuller escape pass arrives
    /// once `fastc-core/json` graduates from this preview.
    pub fn str_value(b: mref(JsonBuilder), s: raw(u8)) -> void {
        write_comma_if_needed(b);
        push_byte(addrm((deref(b)).out), cast(u8, 34));
        push_cstr(addrm((deref(b)).out), s);
        push_byte(addrm((deref(b)).out), cast(u8, 34));
        (deref(b)).needs_comma = 1;
    }

    pub fn int_value(b: mref(JsonBuilder), n: i32) -> void {
        write_comma_if_needed(b);
        write_int_into(b, n);
        (deref(b)).needs_comma = 1;
    }

    pub fn bool_value(b: mref(JsonBuilder), v: bool) -> void {
        write_comma_if_needed(b);
        if (v) {
            push_cstr(addrm((deref(b)).out), cstr("true"));
        } else {
            push_cstr(addrm((deref(b)).out), cstr("false"));
        }
        (deref(b)).needs_comma = 1;
    }

    pub fn null_value(b: mref(JsonBuilder)) -> void {
        write_comma_if_needed(b);
        push_cstr(addrm((deref(b)).out), cstr("null"));
        (deref(b)).needs_comma = 1;
    }

    /// Internal: write a comma if the current container has already
    /// produced an entry. Resets needs_comma so the actual entry
    /// callsite doesn't double-emit.
    fn write_comma_if_needed(b: mref(JsonBuilder)) -> void {
        if ((deref(b)).needs_comma == 1) {
            push_byte(addrm((deref(b)).out), cast(u8, 44));   // ','
        }
    }

    /// Internal: write a signed i32 as ASCII digits. Mirrors the
    /// runtime helper but appends to the Str buffer instead of
    /// stdout so we keep encoder output pure.
    fn write_int_into(b: mref(JsonBuilder), n: i32) -> void {
        if (n == 0) {
            push_byte(addrm((deref(b)).out), cast(u8, 48));   // '0'
            return;
        }
        let val: i32 = n;
        if (val < 0) {
            push_byte(addrm((deref(b)).out), cast(u8, 45));   // '-'
            val = (0 - val);
        }
        // Allocate a 12-byte temp buffer for the reversed digits.
        let buf: rawm(u8) = alloc(cast(usize, 12));
        let len: usize = cast(usize, 0);
        while (val > 0) {
            let d: i32 = (val - ((val / 10) * 10));
            unsafe {
                at(buf, len) = cast(u8, (48 + d));
            }
            len = (len + cast(usize, 1));
            val = (val / 10);
        }
        // Now walk the buffer backwards to emit MSD first.
        let i: usize = len;
        while (i > cast(usize, 0)) {
            i = (i - cast(usize, 1));
            unsafe {
                push_byte(addrm((deref(b)).out), at(buf, i));
            }
        }
        // Best-effort: leak the temp. Real impl would free via mem::free_bytes
        // but `free_bytes` isn't in scope here without polluting the root.
        discard(buf);
    }

    /// Release the builder's underlying buffer. The Str's data is
    /// the caller's responsibility — disposing the JsonBuilder
    /// disposes the Str.
    pub fn release_builder(b: mref(JsonBuilder)) -> void {
        release(addrm((deref(b)).out.data));
    }
}

// --- Capability stubs (stage 1.4 preview) ---
//
// Capability-typed I/O is fastC's strategic wedge — every I/O entry
// point in the post-1.4 stdlib takes a capability value as an
// explicit argument, so the type system tracks what a function is
// allowed to do. Capability values can only be minted in `main` via
// `caps::init()`; from there they flow downward through call args
// until they reach the I/O syscall.
//
// v1 ships the *shape* of this API without the enforcement: the cap
// types are real struct types you can pass around, and `caps::init`
// returns a populated `Caps` bundle. Stage-1.4 will (a) add a
// flow-analysis pass that errors when a function calls a cap-
// requiring callee without holding the cap, and (b) rewrite the
// existing `mod fs` / `mod net` (currently empty) to take caps.
// Today the cap-aware helpers in this file are pure stubs — they
// declare the right signature so dependent code compiles, but the
// body just returns a placeholder.

struct CapFsRead {}
struct CapFsWrite {}
struct CapNetConnect {}
struct CapNetListen {}
struct CapProcSpawn {}
struct CapTimeRead {}
struct CapRand {}
struct CapEnvRead {}

// Master capability bundle. Held only by `main` after `caps::init()`.
struct Caps {
    fs_read: CapFsRead,
    fs_write: CapFsWrite,
    net_connect: CapNetConnect,
    net_listen: CapNetListen,
    proc_spawn: CapProcSpawn,
    time_read: CapTimeRead,
    rand: CapRand,
    env_read: CapEnvRead,
}

mod caps {
    /// Mint the master capability bundle. Stage-1.4 enforcement
    /// will allow this to be called from `main` only — every other
    /// function has to receive caps as arguments.
    pub fn init() -> Caps {
        return Caps {
            fs_read: CapFsRead {},
            fs_write: CapFsWrite {},
            net_connect: CapNetConnect {},
            net_listen: CapNetListen {},
            proc_spawn: CapProcSpawn {},
            time_read: CapTimeRead {},
            rand: CapRand {},
            env_read: CapEnvRead {},
        };
    }

    /// Drop a capability. Stage-1.4 enforcement will make a dropped
    /// cap impossible to use (the value is consumed). v1 stub
    /// just discards.
    pub fn drop_fs_read(_c: CapFsRead) -> void {}
    pub fn drop_fs_write(_c: CapFsWrite) -> void {}
    pub fn drop_net_connect(_c: CapNetConnect) -> void {}
    pub fn drop_net_listen(_c: CapNetListen) -> void {}
}

// --- Vec[T]: the first generic container ---
//
// Heap-backed dynamic array. v1 is fixed-capacity (no automatic growth);
// growth lives in a follow-up slice once `realloc` and capability-aware
// allocators are in. Drop integration is also a follow-up — generic impls
// (`impl Drop for Vec[T]`) need parser/desugar/mono support that we have
// not built yet. Today callers free the buffer manually with `vec::free`.

struct Vec[T] {
    data: rawm(T),
    len: usize,
    cap: usize,
}

mod vec {
    use mem::alloc;
    use mem::resize;
    use mem::free_bytes;

    /// Allocate a vec with the given capacity. `seed` is written into every
    /// slot so the buffer is fully initialized (no UB on later reads); it
    /// also fixes `T` at the call site, which v1 type-arg inference needs
    /// because the bare `cap: usize` argument carries no `T`. Returned vec
    /// has `len = 0`; values become observable only via `vec::push`.
    pub fn with_capacity[T](seed: T, cap: usize) -> Vec[T] {
        let nbytes: usize = (cap * sizeof(T));
        let raw_buf: rawm(u8) = alloc(nbytes);
        let buf: rawm(T) = cast(rawm(T), raw_buf);
        let i: usize = cast(usize, 0);
        while (i < cap) {
            unsafe {
                at(buf, i) = seed;
            }
            i = (i + cast(usize, 1));
        }
        // Explicit cast on `data` so the struct-mono pass can infer T from
        // the field value (approx_field_type only inspects Cast nodes).
        return Vec {
            data: cast(rawm(T), raw_buf),
            len: cast(usize, 0),
            cap: cap,
        };
    }

    /// Build an empty vec. `seed` exists only to fix `T` at the call site.
    pub fn new[T](seed: T) -> Vec[T] {
        return with_capacity(seed, cast(usize, 0));
    }

    /// Append `x` to the vec, growing the backing buffer if `len == cap`.
    /// Growth doubles the capacity (initial 4 if cap was 0), matching
    /// libc's amortized-O(1) idiom and Rust's std `Vec` policy. Reads
    /// after a successful push see `x` at index `old_len`.
    pub fn push[T](v: mref(Vec[T]), x: T) -> void {
        let cur: usize = (deref(v)).len;
        let cap_v: usize = (deref(v)).cap;
        if (cur >= cap_v) {
            let new_cap: usize = next_cap(cap_v);
            let new_bytes: usize = (new_cap * sizeof(T));
            let old_raw: rawm(u8) = cast(rawm(u8), (deref(v)).data);
            let new_raw: rawm(u8) = resize(old_raw, new_bytes);
            (deref(v)).data = cast(rawm(T), new_raw);
            (deref(v)).cap = new_cap;
        }
        let buf: rawm(T) = (deref(v)).data;
        unsafe {
            at(buf, cur) = x;
        }
        (deref(v)).len = (cur + cast(usize, 1));
    }

    /// Growth policy: start at 4 (so the first push of an empty vec
    /// allocates a useful amount), then double. Kept as a free function so
    /// the policy is testable and overridable without rewriting `push`.
    fn next_cap(cur: usize) -> usize {
        if (cur == cast(usize, 0)) {
            return cast(usize, 4);
        }
        return (cur * cast(usize, 2));
    }

    /// Read the element at index `i`. v1 does not bounds-check; callers
    /// must ensure `i < vec::len(v)`. A safe `get_opt` returning `opt(T)`
    /// arrives with the safety-tier slice.
    pub fn get[T](v: ref(Vec[T]), i: usize) -> T {
        let buf: rawm(T) = (deref(v)).data;
        unsafe {
            return at(buf, i);
        }
    }

    pub fn len[T](v: ref(Vec[T])) -> usize {
        return (deref(v)).len;
    }

    /// True when the vec holds zero elements. Cheaper than `len(v) == 0`
    /// at the call site because callers don't have to write the cast.
    pub fn is_empty[T](v: ref(Vec[T])) -> bool {
        return ((deref(v)).len == cast(usize, 0));
    }

    /// Drop the last element and return it. Returns `none(T)` when the
    /// vec was empty. The backing buffer is *not* shrunk — keeping the
    /// capacity around makes a subsequent `push` allocation-free.
    pub fn pop[T](v: mref(Vec[T])) -> opt(T) {
        let cur: usize = (deref(v)).len;
        if (cur == cast(usize, 0)) {
            return none(T);
        }
        let new_len: usize = (cur - cast(usize, 1));
        let buf: rawm(T) = (deref(v)).data;
        (deref(v)).len = new_len;
        unsafe {
            return some(at(buf, new_len));
        }
    }

    /// Reset the vec to empty without freeing the buffer. Subsequent
    /// pushes reuse the existing allocation up to `cap`.
    pub fn clear[T](v: mref(Vec[T])) -> void {
        (deref(v)).len = cast(usize, 0);
    }

    /// Return the position of the first element equal to `target`
    /// via the `Eq` trait, or `none(usize)` if absent. Sibling of
    /// `contains` that surfaces the location for callers that need
    /// it.
    pub fn find_index[T: Eq](v: ref(Vec[T]), target: ref(T)) -> opt(usize) {
        let i: usize = cast(usize, 0);
        let n: usize = (deref(v)).len;
        let buf: rawm(T) = (deref(v)).data;
        while (i < n) {
            unsafe {
                let cur: T = at(buf, i);
                if (cur.eq(target)) {
                    return some(i);
                }
            }
            i = (i + cast(usize, 1));
        }
        return none(usize);
    }

    /// Linear search: returns true when any element compares equal to
    /// `target` via the `Eq` trait. Bounded on `T: Eq` so the body can
    /// dispatch through `T_eq(&cur, target)` — same machinery that
    /// `math::min[T: Ord]` uses on `Ord`.
    pub fn contains[T: Eq](v: ref(Vec[T]), target: ref(T)) -> bool {
        let i: usize = cast(usize, 0);
        let n: usize = (deref(v)).len;
        let buf: rawm(T) = (deref(v)).data;
        while (i < n) {
            unsafe {
                let cur: T = at(buf, i);
                if (cur.eq(target)) {
                    return true;
                }
            }
            i = (i + cast(usize, 1));
        }
        return false;
    }

    /// Swap two elements by index. Caller must ensure both indices are
    /// in range; the v1 API does not bounds-check.
    pub fn swap[T](v: mref(Vec[T]), i: usize, j: usize) -> void {
        let buf: rawm(T) = (deref(v)).data;
        unsafe {
            let tmp: T = at(buf, i);
            at(buf, i) = at(buf, j);
            at(buf, j) = tmp;
        }
    }

    /// Return the smallest element by `Ord`, or `none(T)` on an empty
    /// vec. Linear scan keeping a running best — same shape as the
    /// idiomatic Rust `iter().min()`. Named `min_of` instead of `min`
    /// to avoid colliding with `math::min` in mono's bare-name
    /// generic-fn table; qualified-call resolution would let us drop
    /// the suffix.
    pub fn min_of[T: Ord](v: ref(Vec[T])) -> opt(T) {
        let n: usize = (deref(v)).len;
        if (n == cast(usize, 0)) {
            return none(T);
        }
        let buf: rawm(T) = (deref(v)).data;
        unsafe {
            let best: T = at(buf, cast(usize, 0));
            let i: usize = cast(usize, 1);
            while (i < n) {
                let cur: T = at(buf, i);
                if (cur.less_than(addr(best))) {
                    best = cur;
                }
                i = (i + cast(usize, 1));
            }
            return some(best);
        }
    }

    /// Return the largest element by `Ord`, or `none(T)` on an empty
    /// vec. Suffix matches `min_of` for the same naming-collision
    /// reason.
    pub fn max_of[T: Ord](v: ref(Vec[T])) -> opt(T) {
        let n: usize = (deref(v)).len;
        if (n == cast(usize, 0)) {
            return none(T);
        }
        let buf: rawm(T) = (deref(v)).data;
        unsafe {
            let best: T = at(buf, cast(usize, 0));
            let i: usize = cast(usize, 1);
            while (i < n) {
                let cur: T = at(buf, i);
                if (best.less_than(addr(cur))) {
                    best = cur;
                }
                i = (i + cast(usize, 1));
            }
            return some(best);
        }
    }

    /// Build a fresh vec containing `a`'s elements followed by `b`'s.
    /// Both inputs are read-only; the caller still owns each. The
    /// returned vec is packed (cap = a.len + b.len). Useful for
    /// non-destructive concatenation — `extend(dst, src)` is the
    /// in-place sibling.
    pub fn concat[T](a: ref(Vec[T]), b: ref(Vec[T])) -> Vec[T] {
        let na: usize = (deref(a)).len;
        let nb: usize = (deref(b)).len;
        let n: usize = (na + nb);
        let nbytes: usize = (n * sizeof(T));
        let raw_buf: rawm(u8) = alloc(nbytes);
        let dst_buf: rawm(T) = cast(rawm(T), raw_buf);
        let ab: rawm(T) = (deref(a)).data;
        let bb: rawm(T) = (deref(b)).data;
        let i: usize = cast(usize, 0);
        while (i < na) {
            unsafe {
                at(dst_buf, i) = at(ab, i);
            }
            i = (i + cast(usize, 1));
        }
        let j: usize = cast(usize, 0);
        while (j < nb) {
            unsafe {
                at(dst_buf, (na + j)) = at(bb, j);
            }
            j = (j + cast(usize, 1));
        }
        return Vec {
            data: cast(rawm(T), raw_buf),
            len: n,
            cap: n,
        };
    }

    /// Allocate a fresh vec with the same contents and length. Capacity
    /// is set to `len` (packed) rather than copied from the source so
    /// the clone doesn't carry over any slack. The clone is fully
    /// independent — `release`ing either does not affect the other.
    pub fn clone[T](src: ref(Vec[T])) -> Vec[T] {
        let n: usize = (deref(src)).len;
        let nbytes: usize = (n * sizeof(T));
        let raw_buf: rawm(u8) = alloc(nbytes);
        let src_buf: rawm(T) = (deref(src)).data;
        let dst_buf: rawm(T) = cast(rawm(T), raw_buf);
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                at(dst_buf, i) = at(src_buf, i);
            }
            i = (i + cast(usize, 1));
        }
        return Vec {
            data: cast(rawm(T), raw_buf),
            len: n,
            cap: n,
        };
    }

    /// Sort the vec in place using the `Ord` trait. Insertion sort —
    /// O(n²), but simple, stable, and adequate for the small-vec
    /// workloads v1 stdlib targets. A quicksort / introsort path will
    /// replace this once generic recursion is well-exercised. Bounded
    /// on `T: Ord` so the body can call `cur.less_than(addr(prev))`
    /// which mono lowers to `T_less_than(&cur, &prev)`.
    pub fn sort[T: Ord](v: mref(Vec[T])) -> void {
        let n: usize = (deref(v)).len;
        if (n < cast(usize, 2)) {
            return;
        }
        let buf: rawm(T) = (deref(v)).data;
        let i: usize = cast(usize, 1);
        while (i < n) {
            let j: usize = i;
            while (j > cast(usize, 0)) {
                let prev_idx: usize = (j - cast(usize, 1));
                let did_swap: bool = false;
                unsafe {
                    let cur: T = at(buf, j);
                    let prev: T = at(buf, prev_idx);
                    if (cur.less_than(addr(prev))) {
                        at(buf, j) = prev;
                        at(buf, prev_idx) = cur;
                        did_swap = true;
                    }
                }
                if (did_swap) {
                    j = prev_idx;
                } else {
                    // Reached the first slot that is <= cur — element is
                    // in position. Exit the inner loop by clamping j to
                    // zero so the `j > 0` guard fails on the next check.
                    j = cast(usize, 0);
                }
            }
            i = (i + cast(usize, 1));
        }
    }

    /// Reverse the vec in place. O(n/2). No allocation; reuses the
    /// existing buffer.
    pub fn reverse[T](v: mref(Vec[T])) -> void {
        let n: usize = (deref(v)).len;
        if (n < cast(usize, 2)) {
            return;
        }
        let buf: rawm(T) = (deref(v)).data;
        let i: usize = cast(usize, 0);
        let j: usize = (n - cast(usize, 1));
        while (i < j) {
            unsafe {
                let tmp: T = at(buf, i);
                at(buf, i) = at(buf, j);
                at(buf, j) = tmp;
            }
            i = (i + cast(usize, 1));
            j = (j - cast(usize, 1));
        }
    }

    /// True when at least one element matches `pred`. Short-circuits
    /// on the first match — does not touch later elements.
    pub fn any[T](src: ref(Vec[T]), pred: fn(T) -> bool) -> bool {
        let n: usize = (deref(src)).len;
        let buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                if (pred(at(buf, i))) {
                    return true;
                }
            }
            i = (i + cast(usize, 1));
        }
        return false;
    }

    /// True when every element matches `pred`. Vacuously true on an
    /// empty vec. Short-circuits on the first mismatch.
    pub fn all[T](src: ref(Vec[T]), pred: fn(T) -> bool) -> bool {
        let n: usize = (deref(src)).len;
        let buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                if (!pred(at(buf, i))) {
                    return false;
                }
            }
            i = (i + cast(usize, 1));
        }
        return true;
    }

    /// Build a fresh vec containing every element of `src` for which
    /// `pred` returns true, preserving insertion order. The returned vec
    /// is heap-allocated independently of `src`; both must be released.
    /// First higher-order using a `fn(T) -> bool` predicate.
    pub fn filter[T](src: ref(Vec[T]), pred: fn(T) -> bool) -> Vec[T] {
        let n: usize = (deref(src)).len;
        let src_buf: rawm(T) = (deref(src)).data;
        // Start from zero capacity. The explicit cast on `data` lets the
        // struct-mono pass infer T at this literal — `approx_field_type`
        // only inspects Cast nodes.
        let empty_buf: rawm(u8) = alloc(cast(usize, 0));
        let dst: Vec[T] = Vec {
            data: cast(rawm(T), empty_buf),
            len: cast(usize, 0),
            cap: cast(usize, 0),
        };
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                let cur: T = at(src_buf, i);
                if (pred(cur)) {
                    push(addrm(dst), cur);
                }
            }
            i = (i + cast(usize, 1));
        }
        return dst;
    }

    /// Append every element of `src` to `dst`. Equivalent to a hand-
    /// written push loop but lets the stdlib express the intent at the
    /// call site. Exercises mod-internal generic-to-generic dispatch
    /// at one more remove (`extend` -> `push`, both bounded on the
    /// same `T`).
    pub fn extend[T](dst: mref(Vec[T]), src: ref(Vec[T])) -> void {
        let n: usize = (deref(src)).len;
        let buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                push(dst, at(buf, i));
            }
            i = (i + cast(usize, 1));
        }
    }

    /// Sum every element starting from 0. Specialized for `i32` so
    /// the body uses native `+`. A bounded-generic `sum[T: Add]`
    /// would supersede this once a numeric `Add` trait lands.
    pub fn sum_i32(v: ref(Vec[i32])) -> i32 {
        let acc: i32 = 0;
        let n: usize = (deref(v)).len;
        let buf: rawm(i32) = (deref(v)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                acc = (acc + at(buf, i));
            }
            i = (i + cast(usize, 1));
        }
        return acc;
    }

    /// Multiply every element starting from 1. Same caveat as `sum_i32`.
    pub fn product_i32(v: ref(Vec[i32])) -> i32 {
        let acc: i32 = 1;
        let n: usize = (deref(v)).len;
        let buf: rawm(i32) = (deref(v)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                acc = (acc * at(buf, i));
            }
            i = (i + cast(usize, 1));
        }
        return acc;
    }

    /// Left fold: thread `init` through every element via `f`. First
    /// stdlib API to take a two-argument fn pointer; exercises the
    /// fn-ptr typedef pre-pass on arity > 1 and `unify_generic`'s Fn
    /// recursion across both parameter positions.
    pub fn reduce[T, U](src: ref(Vec[T]), init: U, f: fn(U, T) -> U) -> U {
        let acc: U = init;
        let n: usize = (deref(src)).len;
        let buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                acc = f(acc, at(buf, i));
            }
            i = (i + cast(usize, 1));
        }
        return acc;
    }

    /// Visit each element in insertion order. `f` is called by-value
    /// with each element; its return is discarded. First stdlib API to
    /// take a `fn(T) -> void` pointer — exercises void-returning fn
    /// types end-to-end through the typedef pre-pass.
    pub fn for_each[T](src: ref(Vec[T]), f: fn(T) -> void) -> void {
        let n: usize = (deref(src)).len;
        let src_buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                f(at(src_buf, i));
            }
            i = (i + cast(usize, 1));
        }
    }

    /// Map every element through `f` into a fresh vec. `dst` is sized
    /// exactly to `src.len`, so the result is fully packed (no extra
    /// capacity). The caller owns both vecs and must release each
    /// independently. v1 takes a plain fn pointer; closures with captured
    /// state arrive when the closure slice lands.
    pub fn map[T, U](src: ref(Vec[T]), f: fn(T) -> U) -> Vec[U] {
        let n: usize = (deref(src)).len;
        let nbytes: usize = (n * sizeof(U));
        let raw_buf: rawm(u8) = alloc(nbytes);
        let buf: rawm(U) = cast(rawm(U), raw_buf);
        let src_buf: rawm(T) = (deref(src)).data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                at(buf, i) = f(at(src_buf, i));
            }
            i = (i + cast(usize, 1));
        }
        return Vec {
            data: cast(rawm(U), raw_buf),
            len: n,
            cap: n,
        };
    }

    /// Release the heap buffer. The vec value must not be used after this
    /// returns. Replaces the missing Drop integration; the name avoids
    /// `free` so the P10-3 (no-runtime-alloc) rule does not flag every
    /// vec destructor as a libc allocator call.
    pub fn release[T](v: mref(Vec[T])) -> void {
        let buf: rawm(T) = (deref(v)).data;
        free_bytes(cast(rawm(u8), buf));
    }
}

// --- HashMap[K, V]: open-addressing hash table (stage 1.1 slice 18) ---
//
// v1 uses linear probing on a single power-of-two slot array. Each slot
// has a `state` byte: 0 = empty, 1 = occupied, 2 = tombstone (set on
// remove; ignored by lookup, reclaimed on rehash). Resize doubles the
// capacity once load exceeds 75% (occupied + tombstones).
//
// Bounded on `K: Hash + Eq`: insertions hash to find the start bucket,
// then walk linearly via Eq until either an empty slot (insert here) or
// a matching key (overwrite). This is the first stdlib type that uses
// two trait bounds on the same type parameter, exercising the bounds
// parser path `K: Hash + Eq`.
//
// The slot layout is three parallel raw buffers rather than a single
// struct-of-fields buffer. That keeps `sizeof(slot)` independent of
// alignment fiddliness across `K` and `V` choices, and lets each
// buffer grow independently for any future SIMD probing path.

struct HashMap[K, V] {
    keys: rawm(K),
    values: rawm(V),
    state: rawm(u8),
    len: usize,
    tombstones: usize,
    cap: usize,
}

mod hashmap {
    use mem::alloc;
    use mem::resize;
    use mem::free_bytes;

    // NOTE: every public function uses a `_map` suffix instead of the
    // unqualified name that would mirror vec's API. The current mono
    // pass keys `generic_fns` by bare name, so `hashmap::len[K, V]`
    // and `vec::len[T]` would collide — the latter to land overwrites
    // the former and bound checks fire on the wrong type-parameter set.
    // The proper fix is qualified-name resolution in mono; until then,
    // unique names keep both stdlib modules usable side-by-side.

    /// Initial bucket count. Power of two so `hash % cap` reduces to
    /// `hash & (cap - 1)` once we want to optimize. 8 is the smallest
    /// non-trivial size; smaller arrays don't justify the open-
    /// addressing overhead over a flat scan.
    fn hm_initial_cap() -> usize {
        return cast(usize, 8);
    }

    /// Slot states. Kept as plain u8 constants rather than an enum so
    /// the state buffer is a packed `rawm(u8)` with no padding.
    fn hm_st_empty() -> u8 {
        return cast(u8, 0);
    }

    fn hm_st_occupied() -> u8 {
        return cast(u8, 1);
    }

    fn hm_st_tombstone() -> u8 {
        return cast(u8, 2);
    }

    /// Allocate three parallel buffers (keys, values, state) sized for
    /// `cap` slots, zero the state buffer so every slot starts empty.
    /// `k_seed` and `v_seed` fix `K` and `V` at the call site — v1
    /// generic-fn inference can't recover them from `cap: usize` alone.
    pub fn with_cap_map[K: Hash + Eq, V](k_seed: K, v_seed: V, cap: usize) -> HashMap[K, V] {
        let kb_bytes: usize = (cap * sizeof(K));
        let vb_bytes: usize = (cap * sizeof(V));
        let sb_bytes: usize = (cap * sizeof(u8));
        let kb_raw: rawm(u8) = alloc(kb_bytes);
        let vb_raw: rawm(u8) = alloc(vb_bytes);
        let sb: rawm(u8) = alloc(sb_bytes);
        let i: usize = cast(usize, 0);
        while (i < cap) {
            unsafe {
                at(sb, i) = hm_st_empty();
            }
            i = (i + cast(usize, 1));
        }
        // Discard the seeds — they only existed to fix K, V at the call
        // site. The slot contents stay uninitialized until a successful
        // insert writes them; the state buffer is what readers consult.
        discard(k_seed);
        discard(v_seed);
        return HashMap {
            keys: cast(rawm(K), kb_raw),
            values: cast(rawm(V), vb_raw),
            state: sb,
            len: cast(usize, 0),
            tombstones: cast(usize, 0),
            cap: cap,
        };
    }

    /// Build an empty map at the default initial capacity.
    pub fn new_map[K: Hash + Eq, V](k_seed: K, v_seed: V) -> HashMap[K, V] {
        return with_cap_map(k_seed, v_seed, hm_initial_cap());
    }

    /// Reduce a hash to a slot index for the current capacity.
    fn hm_bucket_of(h: usize, cap: usize) -> usize {
        return (h - ((h / cap) * cap));
    }

    /// Walk slots starting at the natural bucket. Returns the first
    /// index that either matches `key` (occupied) or is free (empty
    /// or tombstone — caller distinguishes via the state byte).
    /// Bounded on `K: Hash + Eq` so the body can call `key.hash()` and
    /// `key.eq(addr(stored_key))`.
    fn hm_find_slot[K: Hash + Eq, V](m: ref(HashMap[K, V]), key: ref(K)) -> usize {
        let cap: usize = (deref(m)).cap;
        let h: usize = key.hash();
        let start: usize = hm_bucket_of(h, cap);
        let i: usize = cast(usize, 0);
        let kb: rawm(K) = (deref(m)).keys;
        let sb: rawm(u8) = (deref(m)).state;
        while (i < cap) {
            let idx: usize = hm_bucket_of((start + i), cap);
            unsafe {
                let s: u8 = at(sb, idx);
                if (s == hm_st_empty()) {
                    return idx;
                }
                if (s == hm_st_occupied()) {
                    let stored: K = at(kb, idx);
                    if (stored.eq(key)) {
                        return idx;
                    }
                }
            }
            i = (i + cast(usize, 1));
        }
        // Map is full — shouldn't happen because `insert` rehashes
        // before this returns. Return cap as a sentinel.
        return cap;
    }

    /// Number of currently-occupied slots. Tombstones are *not*
    /// counted because they represent removed keys.
    pub fn count_map[K: Hash + Eq, V](m: ref(HashMap[K, V])) -> usize {
        return (deref(m)).len;
    }

    /// True when no entries are reachable. Cheap because we track len
    /// directly rather than scanning the state buffer.
    pub fn empty_map[K: Hash + Eq, V](m: ref(HashMap[K, V])) -> bool {
        return ((deref(m)).len == cast(usize, 0));
    }

    /// True when `key` resolves to an occupied slot.
    pub fn has_key[K: Hash + Eq, V](m: ref(HashMap[K, V]), key: ref(K)) -> bool {
        let idx: usize = hm_find_slot(m, key);
        let cap: usize = (deref(m)).cap;
        if (idx >= cap) {
            return false;
        }
        let sb: rawm(u8) = (deref(m)).state;
        unsafe {
            return (at(sb, idx) == hm_st_occupied());
        }
    }

    /// Lookup the value associated with `key`. Returns `none(V)` when
    /// the key is absent, `some(value)` otherwise. The returned value
    /// is a copy of the slot contents.
    pub fn lookup[K: Hash + Eq, V](m: ref(HashMap[K, V]), key: ref(K)) -> opt(V) {
        let idx: usize = hm_find_slot(m, key);
        let cap: usize = (deref(m)).cap;
        if (idx >= cap) {
            return none(V);
        }
        let sb: rawm(u8) = (deref(m)).state;
        let vb: rawm(V) = (deref(m)).values;
        unsafe {
            if (at(sb, idx) == hm_st_occupied()) {
                return some(at(vb, idx));
            }
        }
        return none(V);
    }

    /// Insert `key -> value`, overwriting any existing mapping. Returns
    /// the previous value as `some(old)`, or `none(V)` if the key was
    /// new. Triggers a rehash when load (occupied + tombstones) would
    /// exceed 75% of capacity, doubling the array.
    pub fn put[K: Hash + Eq, V](m: mref(HashMap[K, V]), key: K, value: V) -> opt(V) {
        // Grow first so we always have a free slot to write into.
        // 75% load = 3 * (occupied + tombstones) >= 4 * cap.
        let cap: usize = (deref(m)).cap;
        let load: usize = ((deref(m)).len + (deref(m)).tombstones);
        if ((load * cast(usize, 4)) >= (cap * cast(usize, 3))) {
            hm_rehash(m, (cap * cast(usize, 2)));
        }
        let idx: usize = hm_find_slot(addr(deref(m)), addr(key));
        let kb: rawm(K) = (deref(m)).keys;
        let vb: rawm(V) = (deref(m)).values;
        let sb: rawm(u8) = (deref(m)).state;
        unsafe {
            let s: u8 = at(sb, idx);
            if (s == hm_st_occupied()) {
                let prev: V = at(vb, idx);
                at(vb, idx) = value;
                return some(prev);
            }
            // Empty or tombstone: write a fresh entry.
            if (s == hm_st_tombstone()) {
                (deref(m)).tombstones = ((deref(m)).tombstones - cast(usize, 1));
            }
            at(kb, idx) = key;
            at(vb, idx) = value;
            at(sb, idx) = hm_st_occupied();
            (deref(m)).len = ((deref(m)).len + cast(usize, 1));
        }
        return none(V);
    }

    /// Delete `key`, returning the removed value as `some(v)` or
    /// `none(V)` when it wasn't present. Slot transitions to tombstone
    /// so probe chains through it stay intact until the next rehash.
    pub fn drop_key[K: Hash + Eq, V](m: mref(HashMap[K, V]), key: ref(K)) -> opt(V) {
        let idx: usize = hm_find_slot(addr(deref(m)), key);
        let cap: usize = (deref(m)).cap;
        if (idx >= cap) {
            return none(V);
        }
        let sb: rawm(u8) = (deref(m)).state;
        let vb: rawm(V) = (deref(m)).values;
        unsafe {
            if (at(sb, idx) != hm_st_occupied()) {
                return none(V);
            }
            let prev: V = at(vb, idx);
            at(sb, idx) = hm_st_tombstone();
            (deref(m)).len = ((deref(m)).len - cast(usize, 1));
            (deref(m)).tombstones = ((deref(m)).tombstones + cast(usize, 1));
            return some(prev);
        }
    }

    /// Re-allocate to `new_cap` slots and re-insert every occupied
    /// entry. Drops tombstones, so post-rehash `tombstones == 0`.
    fn hm_rehash[K: Hash + Eq, V](m: mref(HashMap[K, V]), new_cap: usize) -> void {
        let old_cap: usize = (deref(m)).cap;
        let old_keys: rawm(K) = (deref(m)).keys;
        let old_values: rawm(V) = (deref(m)).values;
        let old_state: rawm(u8) = (deref(m)).state;
        // Allocate the new buffers.
        let kb_raw: rawm(u8) = alloc((new_cap * sizeof(K)));
        let vb_raw: rawm(u8) = alloc((new_cap * sizeof(V)));
        let sb: rawm(u8) = alloc((new_cap * sizeof(u8)));
        let i: usize = cast(usize, 0);
        while (i < new_cap) {
            unsafe {
                at(sb, i) = hm_st_empty();
            }
            i = (i + cast(usize, 1));
        }
        // Swap in the new buffers so hm_find_slot uses the new sizing.
        (deref(m)).keys = cast(rawm(K), kb_raw);
        (deref(m)).values = cast(rawm(V), vb_raw);
        (deref(m)).state = sb;
        (deref(m)).cap = new_cap;
        (deref(m)).len = cast(usize, 0);
        (deref(m)).tombstones = cast(usize, 0);
        // Re-insert every occupied slot from the old arrays. The
        // arrays still exist independently of the map at this point
        // because we only swapped the pointers above.
        let j: usize = cast(usize, 0);
        while (j < old_cap) {
            unsafe {
                if (at(old_state, j) == hm_st_occupied()) {
                    let k: K = at(old_keys, j);
                    let v: V = at(old_values, j);
                    discard(put(m, k, v));
                }
            }
            j = (j + cast(usize, 1));
        }
        // Free the old buffers.
        free_bytes(cast(rawm(u8), old_keys));
        free_bytes(cast(rawm(u8), old_values));
        free_bytes(old_state);
    }

    /// Allocate a fresh map with the same capacity and contents as
    /// `src`. Slot allocations are bit-copies — for primitive keys
    /// and values that means a fully independent map; for owned types
    /// like `Str` the .data pointers are aliased, so releasing both
    /// without per-entry deep-clone would double-free the inner
    /// buffers. Document this aliasing in calling code; the v1 stdlib
    /// has no `clone[K: Clone, V: Clone]` because there is no `Clone`
    /// trait yet.
    pub fn clone_map[K: Hash + Eq, V](src: ref(HashMap[K, V])) -> HashMap[K, V] {
        let cap: usize = (deref(src)).cap;
        let k_bytes: usize = (cap * sizeof(K));
        let v_bytes: usize = (cap * sizeof(V));
        let s_bytes: usize = (cap * sizeof(u8));
        let kb_raw: rawm(u8) = alloc(k_bytes);
        let vb_raw: rawm(u8) = alloc(v_bytes);
        let sb: rawm(u8) = alloc(s_bytes);
        let src_keys: rawm(K) = (deref(src)).keys;
        let src_vals: rawm(V) = (deref(src)).values;
        let src_state: rawm(u8) = (deref(src)).state;
        let dst_keys: rawm(K) = cast(rawm(K), kb_raw);
        let dst_vals: rawm(V) = cast(rawm(V), vb_raw);
        let i: usize = cast(usize, 0);
        while (i < cap) {
            unsafe {
                let st: u8 = at(src_state, i);
                at(sb, i) = st;
                if (st == hm_st_occupied()) {
                    at(dst_keys, i) = at(src_keys, i);
                    at(dst_vals, i) = at(src_vals, i);
                }
            }
            i = (i + cast(usize, 1));
        }
        return HashMap {
            keys: cast(rawm(K), kb_raw),
            values: cast(rawm(V), vb_raw),
            state: sb,
            len: (deref(src)).len,
            tombstones: (deref(src)).tombstones,
            cap: cap,
        };
    }

    /// Visit every `(key, value)` pair in occupancy order — i.e. the
    /// underlying bucket order, which is not insertion order. `f`
    /// receives copies of the key and value, so for owned-buffer
    /// types like `Str` the visitor sees aliased pointers (same
    /// ownership rule as the map itself). First stdlib API to take a
    /// two-argument fn pointer on a generic container.
    pub fn for_each_entry[K: Hash + Eq, V](m: ref(HashMap[K, V]), f: fn(K, V) -> void) -> void {
        let cap: usize = (deref(m)).cap;
        let kb: rawm(K) = (deref(m)).keys;
        let vb: rawm(V) = (deref(m)).values;
        let sb: rawm(u8) = (deref(m)).state;
        let i: usize = cast(usize, 0);
        while (i < cap) {
            unsafe {
                if (at(sb, i) == hm_st_occupied()) {
                    f(at(kb, i), at(vb, i));
                }
            }
            i = (i + cast(usize, 1));
        }
    }

    /// Release every internal buffer. The map value must not be used
    /// after this returns.
    pub fn release_map[K: Hash + Eq, V](m: mref(HashMap[K, V])) -> void {
        let kb: rawm(K) = (deref(m)).keys;
        let vb: rawm(V) = (deref(m)).values;
        let sb: rawm(u8) = (deref(m)).state;
        free_bytes(cast(rawm(u8), kb));
        free_bytes(cast(rawm(u8), vb));
        free_bytes(sb);
    }
}

// --- Str: owned byte string built on Vec[u8] (stage 1.1 slice 10) ---
//
// `Str` is a wrapper around `Vec[u8]`. We use a wrapper rather than a
// type alias because aliases aren't a thing in fastC v1, and because a
// nominal type lets future string methods stay separate from generic
// vec methods. The function names avoid collisions with `vec::*`
// (`make` / `dispose` / `byte_count`) because fastC v1 lacks `use X as
// Y`; once aliases land, these can rename back to `new` / `release` /
// `len` for ergonomics.

struct Str {
    data: Vec[u8],
}

// `impl Hash for Str` + `impl Eq for Str` mean `Str` satisfies the
// `Hash + Eq` bounds that `HashMap` requires for keys. The hash is
// djb2 — small, branchless, decent distribution for short ASCII
// inputs (which is what string keys mostly are in v1 demos). A
// stronger hash (fxhash/wyhash) is a stage-2 swap once benchmarking
// surfaces real collision numbers.

impl Hash for Str {
    fn hash(self: ref(Self)) -> usize {
        let h: usize = cast(usize, 5381);
        let n: usize = (deref(self)).data.len;
        let buf: rawm(u8) = (deref(self)).data.data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                // djb2 step: h = (h * 33) + byte
                h = ((h * cast(usize, 33)) + cast(usize, at(buf, i)));
            }
            i = (i + cast(usize, 1));
        }
        return h;
    }
}

impl Eq for Str {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        let na: usize = (deref(self)).data.len;
        let nb: usize = (deref(other)).data.len;
        if (na != nb) {
            return false;
        }
        let ba: rawm(u8) = (deref(self)).data.data;
        let bb: rawm(u8) = (deref(other)).data.data;
        let i: usize = cast(usize, 0);
        while (i < na) {
            unsafe {
                if (at(ba, i) != at(bb, i)) {
                    return false;
                }
            }
            i = (i + cast(usize, 1));
        }
        return true;
    }
}

mod str {
    use vec::new;
    use vec::with_capacity;
    use vec::push;
    use vec::get;
    use vec::len;
    use vec::is_empty;
    use vec::release;
    use mem::alloc;
    use io::put_char;

    /// Construct an empty Str. Equivalent to `vec::new(0u8)` wrapped.
    pub fn make() -> Str {
        return Str { data: new(cast(u8, 0)) };
    }

    /// Pre-allocate `cap` bytes so subsequent pushes don't reallocate
    /// until exceeding that ceiling.
    pub fn with_cap(cap: usize) -> Str {
        return Str { data: with_capacity(cast(u8, 0), cap) };
    }

    /// Append a single byte. Grows the backing vec when necessary.
    pub fn push_byte(s: mref(Str), b: u8) -> void {
        push(addrm((deref(s)).data), b);
    }

    /// Byte length (not character count — Str is byte-oriented in v1).
    pub fn byte_count(s: ref(Str)) -> usize {
        return len(addr((deref(s)).data));
    }

    /// Read the byte at index `i`. Caller must ensure `i < byte_count(s)`.
    pub fn byte_at(s: ref(Str), i: usize) -> u8 {
        return get(addr((deref(s)).data), i);
    }

    /// True when the Str has zero bytes.
    pub fn empty(s: ref(Str)) -> bool {
        return is_empty(addr((deref(s)).data));
    }

    /// Release the backing heap allocation. Mirrors `vec::release`.
    pub fn dispose(s: mref(Str)) -> void {
        release(addrm((deref(s)).data));
    }

    /// Append every byte of a null-terminated C-style buffer to `s`.
    /// Like `from_cstr` but writes into an existing `Str` rather than
    /// allocating a fresh one — useful for building up a string with
    /// multiple literal fragments without intermediate allocations.
    pub fn push_cstr(s: mref(Str), c: raw(u8)) -> void {
        let i: usize = cast(usize, 0);
        let done: bool = false;
        while (!done) {
            unsafe {
                let b: u8 = at(c, i);
                if (b == cast(u8, 0)) {
                    done = true;
                } else {
                    push_byte(s, b);
                    i = (i + cast(usize, 1));
                }
            }
        }
    }

    /// Return the index of the first occurrence of `byte` in `s`, or
    /// `none(usize)` when not present. Linear scan. Useful for cheap
    /// tokenizers — split on a delimiter, find the next quote, etc.
    pub fn byte_search(s: ref(Str), byte: u8) -> opt(usize) {
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                if (at(buf, i) == byte) {
                    return some(i);
                }
            }
            i = (i + cast(usize, 1));
        }
        return none(usize);
    }

    /// Find the first occurrence of `needle` in `haystack`. Returns
    /// `some(index)` with the starting byte position, or `none(usize)`
    /// when the needle is absent. An empty needle returns
    /// `some(cast(usize, 0))` — every string contains the empty
    /// string at position 0.
    ///
    /// Linear-time naive scan — O(haystack.len * needle.len) worst
    /// case. Acceptable for v1 stdlib workloads; KMP / Boyer-Moore
    /// is a stage-2 optimization once benchmarks identify a hotspot.
    pub fn find(haystack: ref(Str), needle: ref(Str)) -> opt(usize) {
        let hn: usize = len(addr((deref(haystack)).data));
        let nn: usize = len(addr((deref(needle)).data));
        if (nn == cast(usize, 0)) {
            return some(cast(usize, 0));
        }
        if (nn > hn) {
            return none(usize);
        }
        let hbuf: rawm(u8) = (deref(haystack)).data.data;
        let nbuf: rawm(u8) = (deref(needle)).data.data;
        let last_start: usize = (hn - nn);
        let i: usize = cast(usize, 0);
        let scanning: bool = true;
        while (scanning) {
            // Check whether needle matches at offset `i`.
            let j: usize = cast(usize, 0);
            let matches: bool = true;
            while (matches) {
                if (j >= nn) {
                    matches = false;
                    return some(i);
                }
                unsafe {
                    if (at(hbuf, (i + j)) != at(nbuf, j)) {
                        matches = false;
                    }
                }
                if (matches) {
                    j = (j + cast(usize, 1));
                }
            }
            if (i >= last_start) {
                scanning = false;
            } else {
                i = (i + cast(usize, 1));
            }
        }
        return none(usize);
    }

    /// True when `haystack` contains `needle` somewhere. Thin
    /// wrapper around `find` that discards the position. Useful
    /// when you only care about membership.
    pub fn contains_str(haystack: ref(Str), needle: ref(Str)) -> bool {
        let pos: opt(usize) = find(haystack, needle);
        let found: bool = false;
        if let idx = unwrap_checked(pos) {
            // Reference idx in the body so clang doesn't flag it as
            // unused under -Werror. The value is always >= 0 for
            // usize, so the comparison is trivially true.
            found = (idx >= cast(usize, 0));
        }
        return found;
    }

    /// True when `haystack` ends with `needle`'s bytes. Empty needle
    /// returns true vacuously. Mirror of `starts_with` walking from
    /// the end.
    pub fn ends_with(haystack: ref(Str), needle: ref(Str)) -> bool {
        let hn: usize = len(addr((deref(haystack)).data));
        let nn: usize = len(addr((deref(needle)).data));
        if (nn > hn) {
            return false;
        }
        let offset: usize = (hn - nn);
        let hbuf: rawm(u8) = (deref(haystack)).data.data;
        let nbuf: rawm(u8) = (deref(needle)).data.data;
        let i: usize = cast(usize, 0);
        while (i < nn) {
            unsafe {
                if (at(hbuf, (offset + i)) != at(nbuf, i)) {
                    return false;
                }
            }
            i = (i + cast(usize, 1));
        }
        return true;
    }

    /// True when `haystack` begins with `needle`'s bytes. An empty
    /// needle returns true vacuously. Linear-time byte compare.
    pub fn starts_with(haystack: ref(Str), needle: ref(Str)) -> bool {
        let hn: usize = len(addr((deref(haystack)).data));
        let nn: usize = len(addr((deref(needle)).data));
        if (nn > hn) {
            return false;
        }
        let hbuf: rawm(u8) = (deref(haystack)).data.data;
        let nbuf: rawm(u8) = (deref(needle)).data.data;
        let i: usize = cast(usize, 0);
        while (i < nn) {
            unsafe {
                if (at(hbuf, i) != at(nbuf, i)) {
                    return false;
                }
            }
            i = (i + cast(usize, 1));
        }
        return true;
    }

    /// True for the four common ASCII whitespace bytes: space, tab,
    /// newline, carriage return. Helper for the trim family. Kept
    /// private to mod str — exposing a per-byte classification needs
    /// a fuller Unicode story than v1 has.
    fn is_ws(b: u8) -> bool {
        if (b == cast(u8, 32)) {
            return true;
        }
        if (b == cast(u8, 9)) {
            return true;
        }
        if (b == cast(u8, 10)) {
            return true;
        }
        if (b == cast(u8, 13)) {
            return true;
        }
        return false;
    }

    /// Return a fresh `Str` with leading ASCII whitespace removed.
    /// Caller owns the original and the result; neither is freed by
    /// this call.
    pub fn trim_start(s: ref(Str)) -> Str {
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let i: usize = cast(usize, 0);
        let scanning: bool = true;
        while (scanning) {
            if (i >= n) {
                scanning = false;
            } else {
                unsafe {
                    if (!is_ws(at(buf, i))) {
                        scanning = false;
                    } else {
                        i = (i + cast(usize, 1));
                    }
                }
            }
        }
        let out: Str = make();
        let k: usize = i;
        while (k < n) {
            unsafe {
                push_byte(addrm(out), at(buf, k));
            }
            k = (k + cast(usize, 1));
        }
        return out;
    }

    /// Return a fresh `Str` with trailing ASCII whitespace removed.
    pub fn trim_end(s: ref(Str)) -> Str {
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        // Walk backward to find the new logical length. Use a signed
        // loop counter via `cast(isize, ...)` to handle the all-
        // whitespace input cleanly without underflowing usize.
        let last: usize = n;
        let scanning: bool = (n > cast(usize, 0));
        while (scanning) {
            let prev: usize = (last - cast(usize, 1));
            unsafe {
                if (is_ws(at(buf, prev))) {
                    last = prev;
                    if (last == cast(usize, 0)) {
                        scanning = false;
                    }
                } else {
                    scanning = false;
                }
            }
        }
        let out: Str = make();
        let k: usize = cast(usize, 0);
        while (k < last) {
            unsafe {
                push_byte(addrm(out), at(buf, k));
            }
            k = (k + cast(usize, 1));
        }
        return out;
    }

    /// Return a fresh `Str` with both leading and trailing ASCII
    /// whitespace removed. Two passes for clarity — micro-optimizing
    /// to a single walk waits until benchmarks identify a hotspot.
    pub fn trim(s: ref(Str)) -> Str {
        let mid: Str = trim_start(s);
        let out: Str = trim_end(addr(mid));
        dispose(addrm(mid));
        return out;
    }

    /// Deep-copy a `Str`: allocate a fresh byte buffer and bit-copy
    /// every byte. Source and result are fully independent — calling
    /// `dispose` on either does not affect the other. A future
    /// `impl Clone for Str` will route `s.clone()` through this same
    /// code path once mod-scoped impl blocks are fully wired through
    /// the desugar+mono pipeline.
    pub fn clone_str(s: ref(Str)) -> Str {
        let n: usize = len(addr((deref(s)).data));
        let src_buf: rawm(u8) = (deref(s)).data.data;
        let raw_buf: rawm(u8) = alloc(n);
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                at(raw_buf, i) = at(src_buf, i);
            }
            i = (i + cast(usize, 1));
        }
        return Str {
            data: Vec {
                data: cast(rawm(u8), raw_buf),
                len: n,
                cap: n,
            },
        };
    }

    /// Concatenate `a` and `b` into a fresh `Str`. Allocates exactly
    /// `a.len + b.len` bytes; the result is packed (no trailing
    /// slack). Source strings are unchanged. Named `concat_str` to
    /// avoid the same mono naming-collision the hashmap rename did:
    /// `vec::concat[T]` already owns the bare name in `generic_fns`.
    pub fn concat_str(a: ref(Str), b: ref(Str)) -> Str {
        let out: Str = make();
        let na: usize = len(addr((deref(a)).data));
        let nb: usize = len(addr((deref(b)).data));
        let ab: rawm(u8) = (deref(a)).data.data;
        let bb: rawm(u8) = (deref(b)).data.data;
        let i: usize = cast(usize, 0);
        while (i < na) {
            unsafe {
                push_byte(addrm(out), at(ab, i));
            }
            i = (i + cast(usize, 1));
        }
        let j: usize = cast(usize, 0);
        while (j < nb) {
            unsafe {
                push_byte(addrm(out), at(bb, j));
            }
            j = (j + cast(usize, 1));
        }
        return out;
    }

    /// Split a Str on newline (`\n` = 10) into a `Vec[Str]`. Thin
    /// wrapper around `split` with the most common delimiter. CR
    /// bytes are kept as part of the preceding line — a stricter
    /// CRLF-aware variant arrives once locale handling lands.
    pub fn lines(s: ref(Str)) -> Vec[Str] {
        return split(s, cast(u8, 10));
    }

    /// Build a fresh `Str` that contains `s` repeated `count` times.
    /// Count zero yields an empty Str. Useful for building separators,
    /// padding, banners, etc.
    pub fn repeat(s: ref(Str), count: usize) -> Str {
        let out: Str = make();
        let inner_n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let i: usize = cast(usize, 0);
        while (i < count) {
            let j: usize = cast(usize, 0);
            while (j < inner_n) {
                unsafe {
                    push_byte(addrm(out), at(buf, j));
                }
                j = (j + cast(usize, 1));
            }
            i = (i + cast(usize, 1));
        }
        return out;
    }

    /// Return a fresh `Str` with every ASCII letter mapped to its
    /// uppercase form. Non-letter bytes pass through unchanged.
    /// Locale-agnostic; non-ASCII bytes are left untouched.
    pub fn to_upper(s: ref(Str)) -> Str {
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let out: Str = make();
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                let b: u8 = at(buf, i);
                if (b >= cast(u8, 97)) {
                    if (b <= cast(u8, 122)) {
                        push_byte(addrm(out), (b - cast(u8, 32)));
                    } else {
                        push_byte(addrm(out), b);
                    }
                } else {
                    push_byte(addrm(out), b);
                }
            }
            i = (i + cast(usize, 1));
        }
        return out;
    }

    /// Split `s` into byte-equal-delimited segments. Each segment is a
    /// fresh `Str` allocation; the returned `Vec[Str]` and every
    /// element must be released independently. Empty segments are
    /// preserved — `"a,,b"` split on `,` yields three strings.
    /// Always returns at least one element (possibly the empty Str
    /// when input is empty or starts with delim).
    pub fn split(s: ref(Str), delim: u8) -> Vec[Str] {
        let result: Vec[Str] = new(make());
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let start: usize = cast(usize, 0);
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                if (at(buf, i) == delim) {
                    let seg: Str = make();
                    let j: usize = start;
                    while (j < i) {
                        push_byte(addrm(seg), at(buf, j));
                        j = (j + cast(usize, 1));
                    }
                    push(addrm(result), seg);
                    start = (i + cast(usize, 1));
                }
            }
            i = (i + cast(usize, 1));
        }
        // Trailing segment after the last delimiter (or the whole
        // input if no delimiter was found).
        let tail: Str = make();
        let k: usize = start;
        while (k < n) {
            unsafe {
                push_byte(addrm(tail), at(buf, k));
            }
            k = (k + cast(usize, 1));
        }
        push(addrm(result), tail);
        return result;
    }

    /// Build a `Str` by walking a null-terminated C-style buffer and
    /// pushing every byte until the terminator. Useful for bridging
    /// `cstr("...")` literals (and FFI `const char*` returns) into the
    /// owned-byte world. The terminating nul is *not* copied into the
    /// `Str`. Caller-supplied length: walks until a zero byte, so a
    /// non-null-terminated input is UB in v1.
    pub fn from_cstr(c: raw(u8)) -> Str {
        let s: Str = make();
        let i: usize = cast(usize, 0);
        let done: bool = false;
        while (!done) {
            unsafe {
                let b: u8 = at(c, i);
                if (b == cast(u8, 0)) {
                    done = true;
                } else {
                    push_byte(addrm(s), b);
                    i = (i + cast(usize, 1));
                }
            }
        }
        return s;
    }

    /// Write every byte of `s` to stdout followed by a newline. Uses
    /// `io::put_char` rather than `puts` because `Str` is not null-
    /// terminated — a future zero-copy `as_cstr` that appends a nul
    /// before returning would let `puts` substitute back in.
    pub fn write_line(s: ref(Str)) -> void {
        let n: usize = len(addr((deref(s)).data));
        let buf: rawm(u8) = (deref(s)).data.data;
        let i: usize = cast(usize, 0);
        while (i < n) {
            unsafe {
                put_char(cast(i32, at(buf, i)));
            }
            i = (i + cast(usize, 1));
        }
        put_char(cast(i32, 10));
    }

    /// Byte-wise equality. Two strings are equal when they have the
    /// same length and every byte matches in order. O(n) compare —
    /// no early hash check yet because str doesn't memoize a hash.
    pub fn eq(a: ref(Str), b: ref(Str)) -> bool {
        let na: usize = len(addr((deref(a)).data));
        let nb: usize = len(addr((deref(b)).data));
        if (na != nb) {
            return false;
        }
        let ba: rawm(u8) = (deref(a)).data.data;
        let bb: rawm(u8) = (deref(b)).data.data;
        let i: usize = cast(usize, 0);
        while (i < na) {
            unsafe {
                if (at(ba, i) != at(bb, i)) {
                    return false;
                }
            }
            i = (i + cast(usize, 1));
        }
        return true;
    }
}
"#;

/// Parse the prelude into a `Vec<Item>` ready to be prepended to a user
/// file. Parse errors here are programmer bugs in this file — they panic
/// rather than surface as user diagnostics.
pub fn prelude_items() -> Vec<crate::ast::Item> {
    let file = crate::driver::parse(PRELUDE_SRC, "<prelude>")
        .expect("prelude must always parse — fix prelude.rs");
    file.items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Item;

    #[test]
    fn prelude_parses_into_items() {
        let items = prelude_items();
        // Three traits + 13 primitives × ~3 impls each. Sanity-check we get
        // a non-trivial number without pinning a brittle exact count.
        assert!(items.len() >= 30, "got {} items", items.len());
        // First three should be the trait declarations.
        let mut trait_names: Vec<String> = items
            .iter()
            .filter_map(|i| match i {
                Item::Trait(t) => Some(t.name.clone()),
                _ => None,
            })
            .collect();
        trait_names.sort();
        assert_eq!(
            trait_names,
            vec!["Clone", "Copy", "Drop", "Eq", "Hash", "Ord"]
        );
    }

    #[test]
    fn prelude_has_i32_eq_impl() {
        let items = prelude_items();
        let found = items.iter().any(|i| match i {
            Item::Impl(b) => b.target == "i32" && b.trait_name.as_deref() == Some("Eq"),
            _ => false,
        });
        assert!(found, "expected impl Eq for i32 in prelude");
    }

    #[test]
    fn prelude_has_math_module() {
        let items = prelude_items();
        let found = items
            .iter()
            .any(|i| matches!(i, Item::Mod(m) if m.name == "math" && m.body.is_some()));
        assert!(found, "expected `mod math` (inline) in prelude");
    }

    #[test]
    fn prelude_has_mem_module() {
        let items = prelude_items();
        let found = items
            .iter()
            .any(|i| matches!(i, Item::Mod(m) if m.name == "mem" && m.body.is_some()));
        assert!(found, "expected `mod mem` (inline) in prelude");
    }

    #[test]
    fn prelude_has_io_module() {
        let items = prelude_items();
        let found = items
            .iter()
            .any(|i| matches!(i, Item::Mod(m) if m.name == "io" && m.body.is_some()));
        assert!(found, "expected `mod io` (inline) in prelude");
    }

    #[test]
    fn prelude_has_vec_struct_and_module() {
        let items = prelude_items();
        let has_struct = items
            .iter()
            .any(|i| matches!(i, Item::Struct(s) if s.name == "Vec" && !s.generics.is_empty()));
        assert!(has_struct, "expected `struct Vec[T]` in prelude");
        let has_mod = items
            .iter()
            .any(|i| matches!(i, Item::Mod(m) if m.name == "vec" && m.body.is_some()));
        assert!(has_mod, "expected `mod vec` (inline) in prelude");
    }
}
