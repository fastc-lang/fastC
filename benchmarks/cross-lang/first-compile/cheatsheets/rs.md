# Rust syntax cheat sheet (Rust 1.85+)

Compile and run: `rustc -O prog.rs -o prog && ./prog`

## Functions and types

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Integers: `i8 i16 i32 i64 i128`, `u8..u128`, `usize isize`. Float: `f32 f64`. `bool`. Pointers/refs: `&T` (shared), `&mut T` (mutable). Owned strings: `String` (heap) or `&str` (borrowed).

## Operators

Standard precedence. `+ - * /` panic on signed overflow in debug, wrap in release (or use `.wrapping_add(...)` / `.checked_add(...)`). Bitwise: `& | ^ ! << >>`. Boolean: `&& || !`.

## Standard library (frequently used)

- `println!("{}", x)`, `print!("{}", x)` from `std`.
- Read line: `let mut s = String::new(); std::io::stdin().read_line(&mut s)?;`.
- Parse: `s.trim().parse::<i64>().unwrap()`.
- Vec: `let mut v: Vec<i32> = Vec::new(); v.push(7);`.

## Control flow

```rust
if cond { } else { }
while cond { }
for i in 0..10 { }
match x { 1 => ..., _ => ... }
```

## Notes

- Last expression in a function body is the return value (no `return` keyword needed unless early-exit).
- `fn main()` is `()`-returning; for `?`-using `main`, declare `fn main() -> std::io::Result<()> { ... Ok(()) }`.
- Mutable bindings need `mut`: `let mut x = 0;`.
