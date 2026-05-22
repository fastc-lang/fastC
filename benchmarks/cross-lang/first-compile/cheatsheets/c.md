# C syntax cheat sheet (C11)

Compile and run: `gcc -O2 prog.c -o prog && ./prog`

## Functions and types

```c
#include <stdio.h>
#include <stdint.h>

int add(int a, int b) {
    return a + b;
}
```

Integers: `int8_t int16_t int32_t int64_t` (via `<stdint.h>`), `size_t` (via `<stddef.h>`). Float: `float`, `double`. `bool` via `<stdbool.h>`. Pointers: `T*`.

## Operators

Standard C precedence applies. `+ - * /` overflow on signed integers is undefined behavior. Bitwise: `& | ^ ~ << >>`. Boolean: `&& || !`.

## Standard library (frequently used)

- `printf("%d\n", x)`, `puts("hello")` from `<stdio.h>`.
- `scanf("%d", &x)` from `<stdio.h>`.
- `malloc(n)`, `free(p)` from `<stdlib.h>`.
- `strlen`, `strcmp`, `memcpy` from `<string.h>`.
- `atoi`, `atol` from `<stdlib.h>`.

## Control flow

```c
if (cond) { } else { }
while (cond) { }
for (int i = 0; i < 10; i++) { }
switch (x) { case 1: ...; break; default: ...; }
```

`break` / `continue` inside loops; `break` inside `switch`.

## Notes

- Variables need explicit types: `int x = 0;`.
- Arrays decay to pointers; pass size explicitly.
- Strings are null-terminated `char*`.
- `scanf` returns the number of successful conversions; check it for safety.
