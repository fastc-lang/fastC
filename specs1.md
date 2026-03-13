Alright — let’s expand FastC into a **comprehensive transpiler spec**: not just surface syntax, but the *hard parts* you want the transpiler to handle so (a) humans + AI don’t trip, (b) generated C is optimizer-safe, and (c) you still compile with **clang/gcc** at full speed.

Below is **FastC v0.2 draft** focused on **transpiler responsibilities**, with explicit lowering rules, safety gates, and “what the compiler must prove vs insert at runtime”.

---

# FastC v0.2 — Comprehensive Transpiler-Centric Spec

## A. Core Concept

FastC is a **strict, canonicalized C dialect** whose compiler is primarily a **source-to-source rewriter**:

**FastC → (checked AST) → (lowered C AST) → C11 code → clang/gcc**

The transpiler must do just enough semantic analysis to guarantee:

* Safe FastC emits C that avoids common UB classes
* Anything that can still be UB is fenced behind `unsafe`
* Generated C is predictable, canonical, and easy to diff/cache
* C output is “boring”: plain structs, plain functions, no exotic compiler tricks

---

# 1) Transpiler Phases (Explicit Contract)

## 1.1 Lex + Parse (Unambiguous Grammar)

* No C declarators.
* One declaration form.
* Mandatory braces.
* No preprocessor in input.

**Deliverable:** deterministic parse tree with no precedence ambiguity (either strict precedence rules or parentheses-required mix rule).

## 1.2 Resolve + Typecheck (Fast, Local)

A “mostly local” pass:

* Type resolution (including type constructors like `ref(T)`, `slice(T)`)
* Nullability flow narrowing
* `unsafe` permission checking (capabilities)
* Basic borrow/exclusivity checks (within function)
* Layout validation for `repr(C)` and FFI signatures

**Deliverable:** typed AST annotated with:

* value category (lvalue/rvalue)
* ownership category (copy/move/borrow)
* safety category (safe/unsafe-required)
* potential lowering strategy

## 1.3 Lowering Pass (Core of the product)

Transforms typed AST into a C AST with:

* explicit temporaries (to preserve evaluation order)
* explicit checks (null, bounds)
* explicit drop/free calls (if using `own(T)`)
* explicit memcpy-based operations for aliasing/alignment-safe bit operations
* monomorphized instantiations (if generics enabled)

**Deliverable:** C AST that is already “final semantics”.

## 1.4 Emit C11 + Headers

* Emit `.c` and `.h` with stable ordering.
* Emit `fastc_rt.h` (tiny runtime shim) only if needed.
* Optional: emit `fastc_map.json` for source mapping (debugability).

---

# 2) More Comprehensive Language Surface (Still “C-like”)

## 2.1 Statements & Control Flow

Support:

* `if/else`, `while`, `for`, `switch`
* `break`, `continue`, `return`
* `defer { ... }` (transpiler lowers to structured cleanup)
* `goto label;` allowed only in `unsafe` OR only to locally-defined labels (configurable)

**Why `defer`:** makes resource cleanup deterministic for AI and humans.

## 2.2 Expressions

Support:

* arithmetic, comparisons, bit ops
* function calls
* struct literals
* field access `.`
* explicit `addr(x)` and `deref(p)`
* indexing only via `at(slice,i)` or `at(arr,i)` (no `[]` sugar in v0.2, optional)
* ternary `?:` disallowed (forces clarity)

## 2.3 Types (Extended)

In addition to the earlier pointer/slice/arr:

* `opt(T)` : Option type (for nullable-ish logic without pointers)
* `res(T, E)` : Result type (error handling without exceptions)
* `fn(T...) -> R` and `fnptr(fn(...) -> ...)`
* `opaque name;` for FFI opaque structs

---

# 3) Runtime Model & Lowering Rules (This is where “cover more” lives)

## 3.1 Evaluation Order (C Gotcha Eliminator)

C has tricky sequencing rules. FastC defines **left-to-right evaluation** for:

* function arguments
* binary operators
* assignment RHS before LHS store

**Transpiler rule:** introduce temporaries to enforce order in emitted C.

Example:

```c
let y: i32 = f(a()) + g(b());
```

Lower to:

```c
int _t0 = a();
int _t1 = f(_t0);
int _t2 = b();
int _t3 = g(_t2);
int y = _t1 + _t3;
```

This single feature eliminates a huge class of “C meaning differs from what AI assumed”.

## 3.2 `defer` Lowering

FastC:

```c
fn foo() -> i32 {
  let p: own(u8) = own_new(u8, 64);
  defer { drop(p); }
  if (cond) { return 1; }
  return 2;
}
```

Lowering approach:

* create a single `cleanup:` label
* route all exits through it
* or use structured blocks with `goto cleanup` (even in safe mode, this is generated)

C:

```c
int foo(void) {
  uint8_t* p = fc_alloc(64, _Alignof(uint8_t));
  int _ret = 0;
  if (!p) { fc_trap(); }
  if (cond) { _ret = 1; goto cleanup; }
  _ret = 2;
cleanup:
  fc_free(p);
  return _ret;
}
```

## 3.3 Ownership (`own(T)`) Lowering (Comprehensive)

FastC has two ownership modes (compiler flag):

* **RAII-inserted drops** (default)
* **explicit-only** (for ultra-transparency)

### 3.3.1 `own_new(T)` / `own_new_array(T, n)`

* Transpiler inserts allocation + init.
* `own(T)` is move-only.

### 3.3.2 Moves

```c
let a: own(T) = own_new(T);
let b: own(T) = a; // move
```

Lowering:

* `b = a; a = NULL;` to prevent double-free in generated code.

### 3.3.3 Borrowing from own

* `ref(T)` can be derived from `own(T)` while `own` still live.
* `mref(T)` derived requires exclusivity (no other borrows active).

This is checked *locally* and enforced by “borrow token” model.

---

# 4) Borrow/Exclusivity Checking (More Coverage, Still Fast)

We want stronger than “trust me” but weaker than Rust. Here’s a tractable model.

## 4.1 Local Borrow Token Model (Function-Scoped)

For each variable of addressable storage (`let mut x`, `own(T)`, `arr`, `struct`):

* maintain state: `unborrowed | shared(n) | unique`
  Rules:
* `ref(x)` increments shared
* `mref(x)` requires unborrowed, sets unique
* leaving scope decrements appropriately
* reborrowing allowed (unique → shared temporarily, unique suspended)

The transpiler rejects:

* creating `mref` while shared exists
* using owner while unique borrow active in conflicting ways
* returning references not tied to params (v0.2 can do a simple origin check)

**What this covers:** a big chunk of C aliasing/lifetime mistakes, cheaply.

---

# 5) Nullability Flow (More Comprehensive)

FastC allows flow-sensitive narrowing:

```c
fn use(p: nmref(i32)) -> void {
  if (is_null(p)) { return; }
  deref(p) = 3; // allowed, p treated as mref(i32) in this block
}
```

**Transpiler must:**

* Track predicates `!is_null(p)`
* Narrow type in dominated blocks
* Invalidate narrowing if `p` is reassigned

Lowering to C: same pointer variable, but safe deref is permitted only in narrowed scope.

---

# 6) Bounds Checking & Proof (Comprehensive)

## 6.1 Slice and Array Access

API:

* `at(s: slice(T), i: usize) -> mref(T)` (or `ref(T)` in const context)
* `get(s, i) -> opt(ref(T))`
* `at_unchecked(...)` unsafe

## 6.2 Proof Engine (Lightweight, fast)

The transpiler should prove bounds in these common patterns:

* `for (i=0; i < s.len; i++) at(s,i)`
* `while (i < len) ... i++`
* `if (i < len) at(s,i)` in that branch

If not proven: insert runtime check.

**Emit stable checks:**

* `if (i >= s.len) fc_trap();`

Compiler flags:

* `-fbounds=always|prove|off` (default `prove`)

---

# 7) Aliasing + Strict-Aliasing Safe Lowering (Comprehensive)

This is a core “transpiler covers more” area.

## 7.1 Typed Access Rule in Safe Mode

Safe FastC disallows:

* `cast(mref(U), p)` unless `U == T` (or declared compatible)
* union-punning
* reading `T` through `u8*` except through explicit APIs

## 7.2 Provide Canonical Safe Primitives (Lowered via memcpy)

* `bitcast(U, x)` if same size and both `pod`
* `load(T, ref(T))` normal
* `load_bytes(ref(T)) -> slice(u8)` (read-only)
* `store_bytes(mref(u8), ...)` only via `memcpy`

**Lowering pattern:**

```c
U bitcast(U, T x) {
  U out;
  memcpy(&out, &x, sizeof(U));
  return out;
}
```

This avoids aliasing UB *and* is extremely optimizer-friendly.

---

# 8) Alignment Guarantees + Unaligned APIs (Comprehensive)

Safe refs guarantee alignment. But for packed/IO buffers you need:

* `read_unaligned(T, raw(u8) p) -> T`
* `write_unaligned(T, rawm(u8) p, T v)`

Lower via `memcpy`. This also keeps UB away.

Additionally:

* `@align(N)` annotation for structs/vars
* `@packed` allowed but field borrows from packed structs require unaligned read.

---

# 9) Integer/Pointer Cast and Address Arithmetic Coverage

## 9.1 Pointer arithmetic only on raw pointers (unsafe)

APIs:

* `ptr_add(raw(T), n: isize) -> raw(T)` unsafe
* `ptr_diff(raw(T), raw(T)) -> isize` unsafe

The transpiler should:

* require `unsafe` block
* optionally insert `-fsanitize=pointer-overflow` friendly checks (optional flag)

## 9.2 Int↔Ptr only via explicit intrinsics (unsafe)

* `ptr_to_usize`, `usize_to_ptr`

No accidental casts.

---

# 10) Concurrency & Data Race Coverage (Comprehensive but pragmatic)

FastC v0.2 provides **standard concurrency primitives** rather than trying to infer thread-safety:

* `atomic(T)` wrapper types for `u32`, `usize`, etc.
* `mutex(T)`, `rwlock(T)` in stdlib
* `thread_spawn(fnptr(...))` etc (optional)

Rules:

* `mref(T)` cannot cross thread boundary (compile error) unless wrapped in safe primitives.
* `raw` can cross but is `unsafe`.

Lowering uses C11 `<stdatomic.h>` if available; fallback shim otherwise.

---

# 11) FFI Coverage: ABI, Headers, and Call Safety

## 11.1 `extern "C"` blocks

* All extern calls are `unsafe` by default.
* You can mark them `@ffi_safe` if types guarantee invariants.

## 11.2 Layout Rules

* `@repr(C)` required for any type passed by value across FFI.
* Otherwise must be passed as `ref/ raw` only.

## 11.3 Header generation

The transpiler emits:

* `fastc_<module>.h` with stable prototypes
* exported structs in `repr(C)` form
* `slice_T` typedefs for any slice types that appear in exported APIs

---

# 12) Diagnostics & AI Guidance (Transpiler must “teach”)

To be AI-friendly, the transpiler must produce:

* single-line error codes (stable)
* “fix-it” suggestions
* “unsafe required because …” explanations

Example:

* `E1207: deref of nullable nmref(i32) requires null check or unwrap()`
* Suggest: `if (is_null(p)) return;`

---

# 13) Optimization-Friendly C Emission (Faster in practice)

The transpiler should emit C that compilers optimize well:

* prefer `static inline` for small helpers in headers
* avoid unnecessary heap allocation
* avoid aliasing pessimism by preserving types
* optionally emit `restrict` on internal `mref` params when exclusivity proven

  * This is huge for performance and safe because our borrow model can justify it.

Example internal signature lowering:

* FastC `fn saxpy(x: mref(f32), y: mref(f32), n: usize)` can become

  * `void saxpy(float* restrict x, float* restrict y, size_t n)`
    if no alias.

This gives C compiler the same power as hand-tuned C but without risk.

---

# 14) “No UB in Safe Mode” Coverage Table (Transpiler-Handled)

Safe mode must either reject or lower safely:

* **Null deref** → rejected unless narrowed; `unwrap` traps
* **OOB** → proven or check inserted
* **Use-after-free** → prevented by `own` move/drop + borrow rules
* **Double free** → prevented by move semantics lowering to NULL
* **Strict aliasing** → disallow punning; provide memcpy-based primitives
* **Misalignment** → forbid via aligned refs; unaligned via memcpy primitives
* **Unsequenced modifications** → enforce eval order via temporaries
* **Pointer overflow arithmetic** → unsafe-only intrinsics
* **Data races** → require atomics/mutex; refs cannot cross threads

---

# 15) What to add next (to make it “fully comprehensive”)

If you agree, v0.3 adds transpiler coverage for the remaining “big C pain” areas:

1. **Definite initialization** (no reading uninit vars)
2. **Exhaustive switch checking** for enums + `default` restrictions
3. **`const`/immutability semantics** that map cleanly to `const` and prevent mistakes
4. **Better interprocedural restrictions**: annotate functions with `@borrows(param)` / `@returns_borrow(param)` so returning refs is safe and explicit
5. **Compile-time constants**: `const` evaluation for array sizes, offsets
6. **Build mode integration**: autogenerate clang flags, sanitizer configs, LTO presets

---

## What I’ll produce next without asking you questions

I can continue by writing the **actual spec artifacts** you’d put in a repo:

* `spec/grammar.ebnf`
* `spec/type_system.md` (pointer kinds, borrow tokens, narrowing)
* `spec/lowering.md` (each construct → C templates)
* `spec/safety.md` (unsafe gates + UB table)
* `rt/fastc_rt.h` and `rt/fastc_rt.c` (allocator, trap, memcpy helpers)
* `examples/` (idioms for AI: slices, ownership, FFI wrappers)

If you want it aligned with your “transpiler with Lark” style work: I can phrase the grammar + AST nodes + passes very similarly to how you’ve done StxScript/Clarity—just targeted at C11 emission.

