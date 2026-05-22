# Go syntax cheat sheet (Go 1.21+)

Compile and run: `go build -o prog main.go && ./prog`

## Functions and types

```go
package main

import "fmt"

func add(a int32, b int32) int32 {
    return a + b
}

func main() {
    // ...
}
```

Integers: `int8 int16 int32 int64`, `uint8..uint64`, `int` (platform native), `uintptr`. Float: `float32 float64`. `bool`. Strings: `string` (immutable). Pointers: `*T`.

## Operators

Standard precedence. `+ - * /` on signed integers panics on division-by-zero only; integer overflow wraps silently. Bitwise: `& | ^ &^ << >>`. Boolean: `&& || !`.

## Standard library (frequently used)

- `fmt.Println(x)`, `fmt.Printf("%d\n", x)`.
- Read line: `var s string; fmt.Scanln(&s)` or `bufio.NewScanner(os.Stdin)`.
- Parse: `strconv.Atoi(s)`, `strconv.ParseInt(s, 10, 64)`.
- Slices: `var v []int32; v = append(v, 7)`.
- `os.Exit(n)` from `"os"`.

## Control flow

```go
if cond { } else { }
for cond { }
for i := 0; i < 10; i++ { }
for i, v := range slice { }
switch x { case 1: ; default: }
```

`break` / `continue` inside loops; `break` inside `switch`.

## Notes

- Every Go program is `package main` with a `main()` function.
- Variables declared with `var x int = 0` or short form `x := 0`.
- Errors are values, not exceptions: most stdlib functions return `(T, error)`.
- Slices grow via `append`; backing arrays are heap-allocated.
- `fmt.Println` adds a trailing newline; `fmt.Print` does not.
