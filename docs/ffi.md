# FFI

FastC is designed to interoperate with C without an ABI barrier.

## Extern Blocks

- Use `extern "C"` to declare foreign functions.
- All extern calls are `unsafe`.
- Extern blocks contain prototypes and type declarations only (no bodies).
- Extern declarations are treated as `unsafe fn` for call‑site checking.

## Layout and ABI

- Types passed by value across FFI must use `@repr(C)`.
- Opaque types are declared as `opaque Name;` and used behind pointers.
- References (`ref`, `mref`) lower to C pointers.
- `slice(T)` lowers to a struct containing `T* data` and `size_t len`.
- `opt(T)` and `res(T, E)` are not permitted in extern signatures.

## Primitive Mappings

- `bool` lowers to `_Bool`.
- `usize` lowers to `size_t` and `isize` lowers to `ptrdiff_t`.
- Enums have a fixed representation (`i32`) unless explicitly annotated.

## Naming and Calling Convention

- Exported symbols use unmangled C names by default.
- All extern declarations default to the C calling convention.

## Minimal Interop Test Matrix

The following tests define the minimum bar for “working and testable C output.” Each test should compile and link with a plain C11 toolchain.

- **Struct layout**: verify `@repr(C)` struct field offsets and size against a C struct in a shared header.
- **Enum representation**: verify enum size and values match the declared underlying type (`i32` by default).
- **Bool ABI**: verify `bool` maps to `_Bool` and matches `sizeof(_Bool)`.
- **Pointer‑sized integers**: verify `usize/isize` map to `size_t/ptrdiff_t`.
- **Slices**: verify `slice(T)` lowers to `{ T* data; size_t len; }` and passes across FFI.
- **Calling convention**: verify that extern calls match C signatures and stack discipline.
- **Name stability**: verify that exported names are unmangled and match generated headers.

## Header Generation

The transpiler emits a C header for exported APIs:

- Function prototypes
- Struct and enum definitions that are `@repr(C)`
- Slice typedefs used in public signatures

## Ownership in FFI

- `own(T)` is not passed by value across FFI.
- Transfer of ownership is expressed via raw pointers and explicit ownership conventions.
