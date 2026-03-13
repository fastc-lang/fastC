# Structs and Enums

FastC supports user-defined types through structs and enums.

## Structs

Structs group related data together.

### Declaration

```c
struct Point {
    x: i32,
    y: i32,
}

struct Rectangle {
    origin: Point,
    width: i32,
    height: i32,
}
```

### Creating Instances

Use struct literals:

```c
let p: Point = Point { x: 10, y: 20 };
let rect: Rectangle = Rectangle {
    origin: Point { x: 0, y: 0 },
    width: 100,
    height: 50,
};
```

### Field Access

```c
let x_coord: i32 = p.x;
let area: i32 = rect.width * rect.height;
```

### Modifying Fields

```c
let p: Point = Point { x: 0, y: 0 };
p.x = 10;
p.y = 20;
```

### Passing Structs

By value (copied):

```c
fn print_point(p: Point) {
    // p is a copy
}
```

By reference (efficient for large structs):

```c
fn move_point(p: mref(Point), dx: i32, dy: i32) {
    deref(p).x = deref(p).x + dx;
    deref(p).y = deref(p).y + dy;
}
```

### C-Compatible Layout

Use `@repr(C)` for guaranteed C-compatible memory layout:

```c
@repr(C)
struct CPoint {
    x: i32,
    y: i32,
}
```

This is required when:

- Passing structs to C functions
- Reading structs from files or network
- Interfacing with hardware

## Enums

Enums define a type with a fixed set of variants.

### Simple Enums

```c
enum Color {
    Red,
    Green,
    Blue,
}

enum Direction {
    North,
    South,
    East,
    West,
}
```

### Using Enums

```c
let color: Color = Color::Red;
let dir: Direction = Direction::North;
```

### Switch on Enums

```c
fn color_code(c: Color) -> i32 {
    switch c {
        case Color::Red:
            return 0xFF0000;
        case Color::Green:
            return 0x00FF00;
        case Color::Blue:
            return 0x0000FF;
        default:
            return 0;
    }
}
```

### Enums with Data

Enums can carry associated data:

```c
enum Result {
    Ok(i32),
    Error(i32),
}

enum Message {
    Text(slice(u8)),
    Number(i64),
    Empty,
}
```

## Examples

### State Machine

```c
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

fn next_state(current: ConnectionState, event: i32) -> ConnectionState {
    switch current {
        case ConnectionState::Disconnected:
            if event == 1 {
                return ConnectionState::Connecting;
            }
            return current;
        case ConnectionState::Connecting:
            if event == 2 {
                return ConnectionState::Connected;
            }
            if event == 3 {
                return ConnectionState::Error;
            }
            return current;
        case ConnectionState::Connected:
            if event == 4 {
                return ConnectionState::Disconnected;
            }
            return current;
        default:
            return current;
    }
}
```

### Nested Structs

```c
struct Address {
    street: slice(u8),
    city: slice(u8),
    zip: i32,
}

struct Person {
    name: slice(u8),
    age: i32,
    address: Address,
}

fn create_person() -> Person {
    return Person {
        name: c"Alice",
        age: 30,
        address: Address {
            street: c"123 Main St",
            city: c"Springfield",
            zip: 12345,
        },
    };
}
```

### Bit Flags

```c
struct Permissions {
    read: bool,
    write: bool,
    execute: bool,
}

fn can_access(perms: Permissions, need_write: bool) -> bool {
    if need_write {
        return perms.write;
    }
    return perms.read;
}
```

## Generated C Code

A FastC struct:

```c
struct Point {
    x: i32,
    y: i32,
}
```

Compiles to:

```c
typedef struct Point {
    int32_t x;
    int32_t y;
} Point;
```

A FastC enum:

```c
enum Color {
    Red,
    Green,
    Blue,
}
```

Compiles to:

```c
typedef enum Color {
    Color_Red,
    Color_Green,
    Color_Blue,
} Color;
```

## Best Practices

1. **Use structs for related data** - Group fields that belong together
2. **Use enums for fixed choices** - State machines, options, error codes
3. **Use `@repr(C)` for FFI** - When interfacing with C code
4. **Keep structs small** - Large structs are expensive to copy
5. **Pass large structs by reference** - Use `ref(T)` or `mref(T)`
