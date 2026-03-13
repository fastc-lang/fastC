# FastC Grammar Reference

A simplified BNF-style grammar for FastC.

## Notation

| Symbol | Meaning |
|--------|---------|
| `::=` | Definition |
| `|` | Alternative |
| `[ ]` | Optional |
| `{ }` | Zero or more |
| `( )` | Grouping |
| `"text"` | Literal |

## Program Structure

```
program         ::= { item }

item            ::= fn_decl
                  | struct_decl
                  | enum_decl
                  | const_decl
                  | extern_block
                  | mod_decl
                  | use_decl
                  | opaque_decl
```

## Functions

```
fn_decl         ::= [attribute] ["pub"] ["unsafe"] "fn" IDENT
                    "(" [param_list] ")" ["->" type] block

param_list      ::= param { "," param }
param           ::= IDENT ":" type

block           ::= "{" { statement } "}"
```

## Types

```
type            ::= primitive_type
                  | pointer_type
                  | array_type
                  | slice_type
                  | optional_type
                  | result_type
                  | fn_type
                  | user_type

primitive_type  ::= "i8" | "i16" | "i32" | "i64"
                  | "u8" | "u16" | "u32" | "u64"
                  | "f32" | "f64"
                  | "bool"
                  | "usize" | "isize"
                  | "void"

pointer_type    ::= "ref" "(" type ")"
                  | "mref" "(" type ")"
                  | "raw" "(" type ")"
                  | "rawm" "(" type ")"

array_type      ::= "arr" "(" type "," INT_LIT ")"

slice_type      ::= "slice" "(" type ")"

optional_type   ::= "opt" "(" type ")"

result_type     ::= "res" "(" type "," type ")"

fn_type         ::= "fn" "(" [type_list] ")" ["->" type]

user_type       ::= IDENT
```

## Statements

```
statement       ::= let_stmt
                  | assign_stmt
                  | return_stmt
                  | if_stmt
                  | while_stmt
                  | for_stmt
                  | switch_stmt
                  | block
                  | expr_stmt
                  | unsafe_block

let_stmt        ::= "let" IDENT ":" type "=" expr ";"

assign_stmt     ::= expr "=" expr ";"

return_stmt     ::= "return" [expr] ";"

if_stmt         ::= "if" "(" expr ")" block ["else" (block | if_stmt)]
                  | "if" "let" IDENT "=" expr block ["else" block]

while_stmt      ::= "while" "(" expr ")" block

for_stmt        ::= "for" "let" IDENT ":" type "=" expr ";"
                    expr ";" assign block

switch_stmt     ::= "switch" "(" expr ")" "{" { case_clause } [default_clause] "}"

case_clause     ::= "case" expr ":" block

default_clause  ::= "default" ":" block

unsafe_block    ::= "unsafe" block

expr_stmt       ::= expr ";"
```

## Expressions

```
expr            ::= literal
                  | IDENT
                  | call_expr
                  | field_expr
                  | index_expr
                  | unary_expr
                  | binary_expr
                  | cast_expr
                  | "(" expr ")"

literal         ::= INT_LIT
                  | FLOAT_LIT
                  | BOOL_LIT
                  | STRING_LIT
                  | CHAR_LIT
                  | array_lit
                  | struct_lit

call_expr       ::= expr "(" [arg_list] ")"
arg_list        ::= expr { "," expr }

field_expr      ::= expr "." IDENT

unary_expr      ::= ("-" | "!" | "~") expr

binary_expr     ::= expr binary_op expr

binary_op       ::= "+" | "-" | "*" | "/" | "%"
                  | "==" | "!=" | "<" | "<=" | ">" | ">="
                  | "&&" | "||"
                  | "&" | "|" | "^" | "<<" | ">>"

cast_expr       ::= "cast" "(" type "," expr ")"

array_lit       ::= "[" [expr { "," expr }] "]"

struct_lit      ::= IDENT "{" [field_init { "," field_init }] "}"

field_init      ::= IDENT ":" expr
```

## Declarations

```
struct_decl     ::= [attribute] ["pub"] "struct" IDENT "{" { field_decl } "}"

field_decl      ::= IDENT ":" type ","

enum_decl       ::= ["pub"] "enum" IDENT "{" { variant_decl } "}"

variant_decl    ::= IDENT ","

const_decl      ::= ["pub"] "const" IDENT ":" type "=" expr ";"

extern_block    ::= "extern" STRING_LIT "{" { extern_fn } "}"

extern_fn       ::= ["unsafe"] "fn" IDENT "(" [param_list] ["," "..."] ")"
                    ["->" type] ";"

mod_decl        ::= "mod" IDENT ";"
                  | "mod" IDENT "{" { item } "}"

use_decl        ::= "use" path ";"

opaque_decl     ::= "opaque" IDENT ";"

attribute       ::= "@" IDENT ["(" attr_args ")"]

attr_args       ::= IDENT { "," IDENT }
```

## Builtin Functions

```
builtin         ::= "addr" "(" expr ")"           // Take address
                  | "deref" "(" expr ")"          // Dereference
                  | "at" "(" expr "," expr ")"    // Index access
                  | "len" "(" expr ")"            // Array/slice length
                  | "some" "(" expr ")"           // Create Some
                  | "none" "(" type ")"           // Create None
                  | "ok" "(" expr ")"             // Create Ok
                  | "err" "(" expr ")"            // Create Err
                  | "is_some" "(" expr ")"        // Check Some
                  | "is_none" "(" expr ")"        // Check None
                  | "is_ok" "(" expr ")"          // Check Ok
                  | "is_err" "(" expr ")"         // Check Err
                  | "unwrap" "(" expr ")"         // Unwrap value
                  | "unwrap_checked" "(" expr ")" // Unwrap in if-let
                  | "cast" "(" type "," expr ")"  // Type cast
                  | "discard" "(" expr ")"        // Discard value
                  | "slice_from" "(" expr ")"    // Array to slice
```

## Lexical Elements

```
IDENT           ::= [a-zA-Z_][a-zA-Z0-9_]*

INT_LIT         ::= [0-9]+
                  | "0x" [0-9a-fA-F]+
                  | "0b" [01]+
                  | "0o" [0-7]+

FLOAT_LIT       ::= [0-9]+ "." [0-9]+ [("e"|"E") ["+"|"-"] [0-9]+]

BOOL_LIT        ::= "true" | "false"

STRING_LIT      ::= '"' { char } '"'
                  | 'c"' { char } '"'    // C string literal

CHAR_LIT        ::= "'" char "'"

COMMENT         ::= "//" { any } newline
                  | "/*" { any } "*/"
```

## Keywords

```
Keywords:
  fn, let, return, if, else, while, for, switch, case, default,
  struct, enum, const, pub, mod, use, extern, unsafe, opaque,
  true, false, void
```

## Operator Precedence

From lowest to highest:

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | `||` | Left |
| 2 | `&&` | Left |
| 3 | `|` | Left |
| 4 | `^` | Left |
| 5 | `&` | Left |
| 6 | `==` `!=` | Left |
| 7 | `<` `<=` `>` `>=` | Left |
| 8 | `<<` `>>` | Left |
| 9 | `+` `-` | Left |
| 10 | `*` `/` `%` | Left |
| 11 | `-` `!` `~` (unary) | Right |
| 12 | `.` `()` `[]` | Left |

## See Also

- [Types](../language/types.md) - Type system details
- [Functions](../language/functions.md) - Function syntax
- [Control Flow](../language/control-flow.md) - Statement syntax

