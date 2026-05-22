# fastC syntax cheat sheet (v1)

fastC is a C-like language designed for safety + agent legibility. Every example below has been verified against the fastC compiler. If you copy these patterns verbatim, your program will compile.

## Compile and run

```bash
fastc compile prog.fc -o prog.c && cc -O2 -Iruntime prog.c -o prog && ./prog
```

## A complete working program

This is the canonical fastC template. Adapt it to your task.

```fastc
use io::print_int;

fn sum(v: ref(Vec[i32])) -> i64 {
    let total: i64 = cast(i64, 0);
    let n: usize = vec::len(v);
    let i: usize = cast(usize, 0);
    while (i < n) {
        total = (total + cast(i64, vec::get(v, i)));
        i = (i + cast(usize, 1));
    }
    return total;
}

fn main() -> i32 {
    let v: Vec[i32] = vec::new(0);
    vec::push(addrm(v), 1);
    vec::push(addrm(v), 2);
    vec::push(addrm(v), 3);
    let result: i64 = sum(addr(v));
    print_int(cast(i32, result));
    return 0;
}
```

Key things to notice:
- Every `use` is at the top of the file, **never inside a function body**.
- Every `let` has an explicit type annotation.
- The integer literal `0` is i32 by default; for any other integer type you need `cast(T, 0)`.
- `vec::len`, `vec::get` take `ref(Vec[T])` (i.e. `addr(v)`); `vec::push` takes `mref(Vec[T])` (i.e. `addrm(v)`).
- Chained binary operators need parens: `(sum + cast(i64, vec::get(v, i)))`, not `sum + cast(i64, vec::get(v, i))`.
- `fn main` must return `i32`. Never `void`, never `()`, never omit the return type.

## Types

| Type | What it is | How to construct |
|---|---|---|
| `i8 i16 i32 i64` | Signed ints | Literal (always i32) or `cast(T, expr)` |
| `u8 u16 u32 u64` | Unsigned ints | `cast(T, expr)` (no unsigned literal syntax) |
| `usize isize` | Pointer-sized ints | `cast(usize, expr)` |
| `f32 f64` | Floats | Literal `3.14` is f64 by default; use `cast(f32, x)` to narrow |
| `bool` | true / false | `true` or `false` |
| `raw(T)`, `rawm(T)` | Immutable / mutable C pointers | FFI return values |
| `ref(T)`, `mref(T)` | Safe references | `addr(x)` / `addrm(x)` |
| `Vec[T]` | Heap-grown array | `vec::new(seed)` then `vec::push(addrm(v), x)` |
| `Str` | Owned string | `str::make()` then `str::push_byte(addrm(s), b)` |
| `opt(T)` | Optional | `some(x)` or `none(T)` |
| `res(T, E)` | Result | `ok(x)` or `err(e)` |

## Operators

- Arithmetic: `+ - * /` — **signed integer overflow traps at runtime**. Use `i64` accumulators for sums of many values.
- Bitwise: `& | ^ ~ << >>`
- Comparison: `== != < <= > >=`
- Boolean: `&& || !`

**Binary expressions must be parenthesized when chained.** fastC has no precedence rules:

```text
let z: i32 = ((a + b) * c);   // correct
let z: i32 = (a + b * c);     // PARSE ERROR
```

## Control flow

```text
if (cond) { ... } else { ... }

while (cond) { ... }

for (let i: i32 = 0; (i < 10); i = (i + 1)) { ... }
```

The for-loop **requires outer parens** around `(init; cond; step)`. Without them you get `expected '(' after 'for'`.

`break` and `continue` work inside loops.

## stdlib I/O (cap-free; no capability argument needed)

```text
use io::println;       // println(cstr("hello"));    — string + newline
use io::put_char;      // put_char(cast(i32, 65));  — one byte
use io::print_int;     // print_int(n);             — i32 in decimal
```

stdin and array-of-bytes reading are **not in v1's stdlib**. If your task needs to read user input, you can't do it in fastC v1.

## Capability-typed I/O (filesystem, time, env, rand)

Every cap-gated function takes a `ref(CapX)` argument. The cap is minted only in `main`:

```fastc
use caps::init;

fn read_clock(c: ref(CapTimeRead)) -> i64 {
    return time::now(c);
}

fn main() -> i32 {
    let bundle: Caps = init();
    let t: i64 = read_clock(addr(bundle.time_read));
    return 0;
}
```

Cap-gated stdlib functions:
- `time::now(c: ref(CapTimeRead)) -> i64`
- `env::get(c: ref(CapEnvRead), key: raw(u8)) -> raw(u8)`
- `fs::exists(c: ref(CapFsRead), path: raw(u8)) -> i32`
- `fs::size_bytes(c: ref(CapFsRead), path: raw(u8)) -> i64`
- `rand::next_u32(c: ref(CapRand)) -> u32`

## Reserved keywords

These are reserved at the lexer level and **cannot be used as variable or parameter names**:

```text
arr  at  cast  addr  addrm  deref  ref  mref  raw  rawm
own  slice  opt  res  cstr  bytes  sizeof  some  none  ok  err
```

The most common LLM mistake is naming a parameter `arr` (because the task says "array of i32"). Use `data`, `nums`, `xs`, or `v` instead.

## Common mistakes (inverse cheatsheet)

These are the patterns observed in failed LLM-generated fastC; the ✓ version is what compiles.

```fastc
// 1. Array types
❌  let a: [i32; 5];                 // Rust-style array type
❌  let a: [5]i32;                   // Zig-style array type
❌  let a: arr(i32, 5) = [1,2,3,4,5];// fastC has NO array literals
✓   let v: Vec[i32] = vec::new(0);   // use Vec for inline collections
    vec::push(addrm(v), 1);
    vec::push(addrm(v), 2);
    // ... etc

// 2. Bracket indexing
❌  v[i]
✓   vec::get(addr(v), i)             // for Vec[T]
✓   at(buf, i)                       // for arr(T, N) or raw(T)

// 3. Integer literal type
❌  let total: i64 = 0;
✓   let total: i64 = cast(i64, 0);
❌  let i: usize = 0;
✓   let i: usize = cast(usize, 0);

// 4. For loop
❌  for let i: i32 = 0; (i < 10); i = (i + 1) { ... }
✓   for (let i: i32 = 0; (i < 10); i = (i + 1)) { ... }

// 5. Vec by value
❌  fn count(v: Vec[i32]) -> usize { return vec::len(v); }
✓   fn count(v: ref(Vec[i32])) -> usize { return vec::len(v); }

// 6. Chained binary ops
❌  return a + b * c;
✓   return (a + (b * c));

// 7. main without return type
❌  fn main() { ... }
❌  fn main() -> void { ... }
✓   fn main() -> i32 { ... return 0; }

// 8. use inside function body
❌  fn main() -> i32 {
        use io::println;
        ...
    }
✓   use io::println;
    fn main() -> i32 { ... }

// 9. arr as identifier
❌  fn sum(arr: ref(Vec[i32])) -> i64 { ... }   // arr is reserved
✓   fn sum(nums: ref(Vec[i32])) -> i64 { ... }

// 10. Group import
❌  use io::{print_int, put_char};
✓   use io::print_int;
    use io::put_char;

// 11. Reading stdin
❌  let n: i32 = io::read_int();      // does not exist in v1
✓   (no stdin reader in v1; bake input as a literal or pass via CLI argv — also not in v1)

// 12. Method call syntax
❌  v.len()                            // no method syntax on user types
✓   vec::len(addr(v))
```

## What's NOT in fastC

- No array literal syntax (`[1, 2, 3]` doesn't parse).
- No stdin / argv reading in the v1 stdlib.
- No method-call syntax for user types (`x.foo()` doesn't work).
- No type inference on `let` — every binding needs `: type`.
- No `match` — use nested `if`.
- No exceptions — use `res(T, E)` with `ok` / `err`.
- No closures with captured values (capture-free closures work).
- No automatic integer widening — `0` is always i32, widen with `cast(T, 0)`.
