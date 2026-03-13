# Safety

FastC separates **safe** and **unsafe** code. Safe code is prevented from invoking common C undefined behavior by construction or by inserted runtime checks.

## Safe Code Guarantees

In safe code, the transpiler enforces or inserts checks for:

- Null dereference
- Out‑of‑bounds access
- Use‑after‑free and double free for `own(T)`
- Misaligned loads and stores
- Strict‑aliasing violations
- Unsequenced side effects
- Signed overflow, division by zero, and invalid shift counts

## Unsafe Code

Unsafe code is explicitly marked with `unsafe { ... }` and allows:

- Raw pointer dereference
- Pointer arithmetic
- Reinterpretation of bytes as another type
- FFI calls with unchecked invariants

Unsafe blocks must document the invariants they assume. The transpiler does not attempt to prove them.

## Unsafe Functions

- `unsafe fn` marks a function as unsafe to call.
- Calls to an `unsafe fn` require an `unsafe` block.

## Runtime Checks

- Bounds checks are inserted when the transpiler cannot prove safety.
- Null checks are inserted for `opt(T)` and for raw pointers when used under `unsafe` helper APIs.
- Failures call `fc_trap()`.

## Numeric Semantics

- Signed overflow traps in safe code.
- Unsigned overflow wraps.
- Division by zero traps.
- Shift counts outside the type width trap.

## Aliasing and Bitcasts

- Typed references must only access their declared type.
- Bitcasts are performed via `memcpy`‑style lowering to avoid strict‑aliasing UB.
- Unaligned accesses are supported only through explicit unaligned APIs.
