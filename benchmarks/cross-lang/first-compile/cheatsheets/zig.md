# Zig syntax cheat sheet (0.16)

Compile and run: `zig build-exe -O ReleaseFast -lc prog.zig && ./prog`

## Functions and types

```zig
fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn main() void {
    // ...
}
```

Integers: `i8 i16 i32 i64 i128`, `u8..u128`, `usize isize`. Float: `f32 f64`. `bool`. Pointers: `*T` (single item), `[*]T` (many).

## Operators

Standard precedence. `+ - * /` overflow on signed integers is undefined behavior in ReleaseFast, asserts in Debug. Use `+%`, `-%`, `*%` for wrapping. Bitwise: `& | ^ ~ << >>`. Boolean: `and or !`.

## Standard library (Zig 0.16 surface, abbreviated)

Zig 0.16 reworked the I/O subsystem. Easy path:
```zig
extern fn puts(s: [*:0]const u8) c_int;
extern fn scanf(fmt: [*:0]const u8, ...) c_int;
extern fn printf(fmt: [*:0]const u8, ...) c_int;
```
Compile with `-lc`.

For pure-Zig stdout, use `std.fs.File.stdout()` and the new `Io` interface. For pure-Zig stdin, `std.fs.File.stdin()` with a `Reader` and a buffer.

## Control flow

```zig
if (cond) { } else { }
while (cond) : (i += 1) { }
for (slice) |x| { }
switch (x) { 1 => {}, else => {} }
```

`break` / `continue` inside loops.

## Conversions

All explicit: `@as(f64, @floatFromInt(x))`, `@intCast(x)`, `@bitCast(x)`. Casts to integer with potential truncation use `@intCast` with a known target type.

## Notes

- `pub fn main() void {}` (no `!void` if no errors).
- Function-local variables: `var x: i32 = 0;` (mutable) or `const x: i32 = 0;` (immutable).
- Slices `[]T` are pointer + length pairs.
