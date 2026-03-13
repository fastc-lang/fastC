# Grammar (Stub)

This is a *formal grammar stub* meant to lock in unambiguous parsing. It is intentionally conservative and may expand as the language stabilizes.

## Goals

- No C declarator ambiguity.
- No implicit precedence surprises.
- A small, regular surface for reliable agentic code generation.

## Notation

- The grammar uses EBNF‑style notation.
- Terminals are shown as literal tokens (for example, `fn`, `{`, `}`).
- `?` denotes optional, `*` denotes repetition, `+` denotes one or more.

## Grammar Design Rules

These rules are part of the syntax contract and exist to keep parsing unambiguous and agent‑safe:

- No C declarator syntax. All types use prefix constructors like `ref(T)` and `slice(T)`.
- No implicit casts or type coercions.
- Conditions must be `bool`; there is no implicit integer‑to‑bool conversion.
- No comma operator.
- No postfix `++` or `--`.
- No ternary operator.
- Assignment is statement‑only (and the `for` update clause accepts assignment syntax).
- Operator precedence is not relied upon; expressions allow at most one binary operator unless parenthesized.
- Expression statements are restricted to function calls or `discard(...)`.

## Lexical Tokens (Sketch)

- `Ident` → `[A-Za-z_][A-Za-z0-9_]*`
- `IntLit` → decimal integer literal
- `FloatLit` → decimal float literal
- `StringLit` → double‑quoted string literal
- `BoolLit` → `true` | `false`

## Top‑Level

```
File        = Item* EOF ;

Item        = FnDecl
            | StructDecl
            | EnumDecl
            | OpaqueDecl
            | ExternBlock
            | ConstDecl
            ;
```

## Declarations

```
FnDecl      = UnsafeOpt "fn" Ident "(" ParamList? ")" ReturnType Block ;
FnProto     = UnsafeOpt "fn" Ident "(" ParamList? ")" ReturnType ";" ;
ParamList   = Param ("," Param)* ;
Param       = Ident ":" Type ;
ReturnType  = "->" ReturnTypeAtom ;
ReturnTypeAtom = "void" | Type ;

StructDecl  = ReprAttr? "struct" Ident "{" FieldList? "}" ;
FieldList   = Field ("," Field)* ","? ;
Field       = Ident ":" Type ;

EnumDecl    = ReprAttr? "enum" Ident "{" VariantList? "}" ;
VariantList = Variant ("," Variant)* ","? ;
Variant     = Ident ("(" TypeList ")")? ;

ExternBlock = "extern" StringLit "{" ExternItem* "}" ;
ExternItem  = FnProto | StructDecl | EnumDecl | OpaqueDecl ;

ConstDecl   = "const" Ident ":" Type "=" ConstExpr ";" ;
OpaqueDecl  = "opaque" Ident ";" ;

ReprAttr    = "@repr" "(" ReprKind ")" ;
ReprKind    = "C"
            | "i8" | "u8"
            | "i16" | "u16"
            | "i32" | "u32"
            | "i64" | "u64"
            ;

UnsafeOpt   = "unsafe"? ;
```

## Statements

```
Stmt        = LetStmt
            | AssignStmt
            | IfStmt
            | WhileStmt
            | ForStmt
            | SwitchStmt
            | ReturnStmt
            | BreakStmt
            | ContinueStmt
            | DeferStmt
            | CallStmt
            | DiscardStmt
            | UnsafeBlock
            | Block
            ;

Block       = "{" Stmt* "}" ;

LetStmt     = "let" Ident ":" Type "=" Expr ";" ;
IfStmt      = "if" "(" Expr ")" Block ("else" (IfStmt | Block))? ;
WhileStmt   = "while" "(" Expr ")" Block ;
ForStmt     = "for" "(" ForInit? ";" ForCond? ";" ForStep? ")" Block ;
ForInit     = LetInit | AssignStep | CallExpr ;
ForCond     = Expr ;
ForStep     = AssignStep | CallExpr ;
LetInit     = "let" Ident ":" Type "=" Expr ;
SwitchStmt  = "switch" "(" Expr ")" "{" Case* DefaultCase? "}" ;
Case        = "case" ConstExpr ":" Stmt* ;
DefaultCase = "default" ":" Stmt* ;
ReturnStmt  = "return" Expr? ";" ;
BreakStmt   = "break" ";" ;
ContinueStmt= "continue" ";" ;
DeferStmt   = "defer" Block ;
CallStmt    = CallExpr ";" ;
DiscardStmt = "discard" "(" Expr ")" ";" ;
UnsafeBlock = "unsafe" Block ;
AssignStmt  = LValue "=" Expr ";" ;
AssignStep  = LValue "=" Expr ;

LValue      = LValueBase ("." Ident)* ;
LValueBase  = Ident
            | "deref" "(" Expr ")"
            | "at" "(" Expr "," Expr ")"
            ;
```

## Types

```
Type        = PrimitiveType
            | Ident
            | "ref" "(" Type ")"
            | "mref" "(" Type ")"
            | "raw" "(" Type ")"
            | "rawm" "(" Type ")"
            | "own" "(" Type ")"
            | "slice" "(" Type ")"
            | "arr" "(" Type "," ConstExpr ")"
            | "opt" "(" Type ")"
            | "res" "(" Type "," Type ")"
            | FnType
            ;

TypeList    = Type ("," Type)* ;

FnType      = UnsafeOpt "fn" "(" TypeList? ")" "->" ReturnTypeAtom ;

PrimitiveType = "i8" | "i16" | "i32" | "i64"
              | "u8" | "u16" | "u32" | "u64"
              | "f32" | "f64"
              | "bool" | "usize" | "isize" ;
```

## Expressions (Single Binary Operator)

Expressions allow **at most one binary operator** at each non‑parenthesized level. Combining binary operators requires parentheses.

```
Expr        = BinaryExpr | Unary ;
BinaryExpr  = Unary BinOp Unary ;
BinOp       = "||" | "&&" | "|" | "^" | "&"
            | "==" | "!=" | "<" | "<=" | ">" | ">="
            | "<<" | ">>" | "+" | "-" | "*" | "/" | "%" ;

Unary       = ("!" | "-" | "~") Unary
            | Postfix
            ;

Postfix     = Primary (Call | FieldAccess)* ;
Call        = "(" ArgList? ")" ;
FieldAccess = "." Ident ;

ArgList     = Expr ("," Expr)* ;

Primary     = IntLit
            | FloatLit
            | BoolLit
            | Ident
            | "(" Expr ")"
            | StructLiteral
            | CastExpr
            | CStrLit
            | BytesLit
            ;

StructLiteral = Ident "{" FieldInitList? "}" ;
FieldInitList = FieldInit ("," FieldInit)* ","? ;
FieldInit   = Ident ":" Expr ;

CastExpr    = "cast" "(" Type "," Expr ")" ;

CallExpr    = Primary CallTail ;
CallTail    = (FieldAccess)* Call (Call | FieldAccess)* ;

CStrLit     = "cstr" "(" StringLit ")" ;
BytesLit    = "bytes" "(" StringLit ")" ;
```

## Disallowed or Restricted Forms

- No implicit casts.
- No assignment expressions.
- No ternary operator.
- No postfix `++` or `--`.
- No pointer arithmetic in safe code.

## Const Expressions

Const expressions are used for `const` initializers, `case` labels, and array sizes. They are a strict subset of expressions.

```
ConstExpr   = ConstBinary | ConstUnary ;
ConstBinary = ConstUnary ConstBinOp ConstUnary ;
ConstBinOp  = "||" | "&&" | "|" | "^" | "&"
            | "==" | "!=" | "<" | "<=" | ">" | ">="
            | "<<" | ">>" | "+" | "-" | "*" | "/" | "%" ;

ConstUnary  = ("!" | "-" | "~") ConstUnary
            | ConstPrimary
            ;

ConstPrimary = IntLit
             | FloatLit
             | BoolLit
             | Ident
             | "(" ConstExpr ")"
             | CastConstExpr
             | CStrLit
             | BytesLit
             ;

CastConstExpr = "cast" "(" Type "," ConstExpr ")" ;
```

## Notes

- `addr(x)` and `deref(p)` are treated as normal function calls for parsing.
- The grammar intentionally avoids `[]` indexing; indexing is `at(x, i)`.
- Operator sets may be narrowed further to avoid confusion in agent‑generated code.
- String literals as expressions appear only inside `cstr("...")` and `bytes("...")`. `extern "C"` still uses a string literal token in declarations.
