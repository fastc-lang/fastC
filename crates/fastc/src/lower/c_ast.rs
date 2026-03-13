//! C AST definitions

/// A C source file
#[derive(Debug, Clone)]
pub struct CFile {
    pub includes: Vec<String>,
    pub forward_decls: Vec<CDecl>,
    pub type_defs: Vec<CDecl>,
    pub fn_protos: Vec<CFnProto>,
    pub fn_defs: Vec<CFnDef>,
}

impl CFile {
    pub fn new() -> Self {
        Self {
            includes: Vec::new(),
            forward_decls: Vec::new(),
            type_defs: Vec::new(),
            fn_protos: Vec::new(),
            fn_defs: Vec::new(),
        }
    }
}

impl Default for CFile {
    fn default() -> Self {
        Self::new()
    }
}

/// C declarations
#[derive(Debug, Clone)]
pub enum CDecl {
    Struct { name: String, fields: Vec<CField> },
    Typedef { name: String, ty: CType },
    Enum { name: String, variants: Vec<String> },
}

/// C struct field
#[derive(Debug, Clone)]
pub struct CField {
    pub name: String,
    pub ty: CType,
}

/// C function prototype
#[derive(Debug, Clone)]
pub struct CFnProto {
    pub name: String,
    pub params: Vec<CParam>,
    pub return_type: CType,
}

/// C function definition
#[derive(Debug, Clone)]
pub struct CFnDef {
    pub name: String,
    pub params: Vec<CParam>,
    pub return_type: CType,
    pub body: Vec<CStmt>,
}

/// C function parameter
#[derive(Debug, Clone)]
pub struct CParam {
    pub name: String,
    pub ty: CType,
}

/// C types
#[derive(Debug, Clone)]
pub enum CType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Double,
    SizeT,
    PtrDiffT,
    Ptr(Box<CType>),       // T* (mutable pointer)
    ConstPtr(Box<CType>),  // const T* (immutable pointer)
    Array(Box<CType>, usize),
    Slice(Box<CType>),
    Opt(Box<CType>),       // fc_opt_T struct { bool has_value; T value; }
    Res(Box<CType>, Box<CType>), // fc_res_T_E struct { bool is_ok; union { T ok; E err; } data; }
    Named(String),
}

/// C statements
#[derive(Debug, Clone)]
pub enum CStmt {
    VarDecl {
        name: String,
        ty: CType,
        init: Option<CExpr>,
    },
    Assign {
        lhs: CExpr,
        rhs: CExpr,
    },
    If {
        cond: CExpr,
        then: Vec<CStmt>,
        else_: Option<Vec<CStmt>>,
    },
    While {
        cond: CExpr,
        body: Vec<CStmt>,
    },
    For {
        init: Option<Box<CStmt>>,
        cond: Option<CExpr>,
        step: Option<CExpr>,
        body: Vec<CStmt>,
    },
    Return(Option<CExpr>),
    Expr(CExpr),
    Block(Vec<CStmt>),
    Goto(String),
    Label(String),
    Switch {
        expr: CExpr,
        cases: Vec<(CExpr, Vec<CStmt>)>,
        default: Option<Vec<CStmt>>,
    },
    Break,
}

/// C expressions
#[derive(Debug, Clone)]
pub enum CExpr {
    IntLit(String),
    FloatLit(String),
    BoolLit(bool),
    StringLit(String),
    Ident(String),
    Binary {
        op: CBinOp,
        lhs: Box<CExpr>,
        rhs: Box<CExpr>,
    },
    Unary {
        op: CUnaryOp,
        operand: Box<CExpr>,
    },
    Call {
        func: Box<CExpr>,
        args: Vec<CExpr>,
    },
    Field {
        base: Box<CExpr>,
        field: String,
    },
    Deref(Box<CExpr>),
    AddrOf(Box<CExpr>),
    Index {
        base: Box<CExpr>,
        index: Box<CExpr>,
    },
    Cast {
        ty: CType,
        expr: Box<CExpr>,
    },
    Paren(Box<CExpr>),
    Compound {
        ty: CType,
        fields: Vec<(String, CExpr)>,
    },
}

/// C binary operators
#[derive(Debug, Clone, Copy)]
pub enum CBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// C unary operators
#[derive(Debug, Clone, Copy)]
pub enum CUnaryOp {
    Neg,
    Not,
    BitNot,
}
