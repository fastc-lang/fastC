# fastC syntax cheat sheet (v1)

Compile and run: `fastc compile prog.fc -o prog.c && cc -O2 -Iruntime prog.c -o prog && ./prog`

## Functions and types

```
fn add(a: i32, b: i32) -> i32 {
    return (a + b);
}
```

Integers: `i8 i16 i32 i64` (signed), `u8 u16 u32 u64` (unsigned), `usize isize`. Float: `f32 f64`. `bool`. Pointers: `raw(T)` (immutable), `rawm(T)` (mutable). `Str` for owned strings (`Vec[u8]` underneath).

## Operators

`+ - * /` are overflow-checked on signed integers (use i64 to avoid traps on intermediate sums). Bitwise: `& | ^ ~ << >>`. Comparison: `== != < <= > >=`. Boolean: `&&` `||` `!`.

**Binary expressions require parentheses when chained:** `(a + b) * c`, not `a + b * c` — fastC has a strict no-precedence-rules-required policy.

## Casts and literals

All conversions are explicit. `cast(f64, x)` to cast int→float, `cast(i32, large_i64)`, etc. Literals: `42`, `3.14`, `true`, `cstr("hi")` for a null-terminated `raw(u8)`.

## Control flow

```
if (cond) { ... } else { ... }
while (cond) { ... }
for let i: i32 = 0; (i < 10); i = (i + 1) { ... }
```

`break` / `continue` work inside `while` and `for`.

## stdlib essentials

- `use io::println;` → `println(cstr("hello"));` prints with newline.
- `use io::put_char;` → `put_char(cast(i32, byte));` writes one byte.
- `use io::print_int;` → `print_int(n);` writes an i32 in decimal.
- `mem::alloc(size: usize) -> rawm(u8)` (libc malloc wrap).
- Vec: `let v: Vec[i32] = vec::new(0); vec::push(addrm(v), 7);`

## Capabilities (only matters if your task does I/O beyond stdout)

I/O surfaces like `fs::*` and `time::*` take a capability argument:
```
let bundle: Caps = caps::init();
let n: i64 = time::now(addr(bundle.time_read));
```
Stdin/stdout via `io::*` are cap-free.

## What's NOT in fastC

- No type inference: every `let` needs `: type`.
- No method-style `x.foo()` for user types (only built-in `vec::len(v)` style).
- No `match` (use nested `if`).
- No exceptions (use `res(T, E)`).
- No closures with captured values (capture-free closures work but most callsites just pass top-level fns).
