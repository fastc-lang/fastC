//! Runtime check insertion for safety

use super::{CBinOp, CExpr, CStmt, CType};

/// Insert a bounds check
pub fn bounds_check(index: CExpr, len: CExpr) -> CStmt {
    CStmt::If {
        cond: CExpr::Binary {
            op: CBinOp::Ge,
            lhs: Box::new(index),
            rhs: Box::new(len),
        },
        then: vec![CStmt::Expr(CExpr::Call {
            func: Box::new(CExpr::Ident("fc_trap".to_string())),
            args: vec![],
        })],
        else_: None,
    }
}

/// Insert a null check
#[allow(dead_code)]
pub fn null_check(ptr: CExpr) -> CStmt {
    CStmt::If {
        cond: CExpr::Binary {
            op: CBinOp::Eq,
            lhs: Box::new(ptr),
            rhs: Box::new(CExpr::Ident("NULL".to_string())),
        },
        then: vec![CStmt::Expr(CExpr::Call {
            func: Box::new(CExpr::Ident("fc_trap".to_string())),
            args: vec![],
        })],
        else_: None,
    }
}

/// Insert a signed overflow check using __builtin_add_overflow
pub fn overflow_check_add(lhs: CExpr, rhs: CExpr, result_var: &str, ty: CType) -> (CStmt, CStmt) {
    overflow_check_with_builtin("__builtin_add_overflow", lhs, rhs, result_var, ty)
}

/// Insert a signed overflow check using __builtin_sub_overflow
pub fn overflow_check_sub(lhs: CExpr, rhs: CExpr, result_var: &str, ty: CType) -> (CStmt, CStmt) {
    overflow_check_with_builtin("__builtin_sub_overflow", lhs, rhs, result_var, ty)
}

/// Insert a signed overflow check using __builtin_mul_overflow
pub fn overflow_check_mul(lhs: CExpr, rhs: CExpr, result_var: &str, ty: CType) -> (CStmt, CStmt) {
    overflow_check_with_builtin("__builtin_mul_overflow", lhs, rhs, result_var, ty)
}

/// Helper for overflow checks with builtin functions
fn overflow_check_with_builtin(
    builtin: &str,
    lhs: CExpr,
    rhs: CExpr,
    result_var: &str,
    ty: CType,
) -> (CStmt, CStmt) {
    let decl = CStmt::VarDecl {
        name: result_var.to_string(),
        ty,
        init: None,
    };

    let check = CStmt::If {
        cond: CExpr::Call {
            func: Box::new(CExpr::Ident(builtin.to_string())),
            args: vec![
                lhs,
                rhs,
                CExpr::AddrOf(Box::new(CExpr::Ident(result_var.to_string()))),
            ],
        },
        then: vec![CStmt::Expr(CExpr::Call {
            func: Box::new(CExpr::Ident("fc_trap".to_string())),
            args: vec![],
        })],
        else_: None,
    };

    (decl, check)
}

/// Insert a division by zero check
pub fn div_zero_check(divisor: CExpr) -> CStmt {
    CStmt::If {
        cond: CExpr::Binary {
            op: CBinOp::Eq,
            lhs: Box::new(divisor),
            rhs: Box::new(CExpr::IntLit("0".to_string())),
        },
        then: vec![CStmt::Expr(CExpr::Call {
            func: Box::new(CExpr::Ident("fc_trap".to_string())),
            args: vec![],
        })],
        else_: None,
    }
}
