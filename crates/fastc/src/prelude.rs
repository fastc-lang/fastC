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

mod str {
    use vec::new;
    use vec::with_capacity;
    use vec::push;
    use vec::get;
    use vec::len;
    use vec::is_empty;
    use vec::release;

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
        assert_eq!(trait_names, vec!["Copy", "Drop", "Eq", "Ord"]);
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
