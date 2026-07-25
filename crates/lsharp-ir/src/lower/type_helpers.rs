use lsharp_syntax::ast::TypeExpr;
use lsharp_types::types::Type;

use crate::IrType;

/// L# 型 -> IR 型
pub fn type_to_ir(ty: &Type) -> IrType {
    match ty {
        Type::Con(name) => match name.as_str() {
            "Int" => IrType::I64,
            "Float" => IrType::F64,
            "Bool" => IrType::I64,   // Bool は i64 (0/1)
            "Unit" => IrType::I64,   // Unit も i64 (0)
            "String" => IrType::I64, // MVP: 文字列はポインタ (i64)
            _ => IrType::I64,
        },
        Type::Var(_) => IrType::I64,    // 未解決の型変数はデフォルト i64
        Type::Fun(_, _) => IrType::I64, // 関数ポインタ
        Type::App(_, _) => IrType::I64, // ADT ポインタ
        Type::Record(_, _) => IrType::I64, // MVP: レコードは i64
    }
}

/// 型から具体型名を抽出（静的ディスパッチ用）
pub(crate) fn type_to_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Con(name) => Some(name.clone()),
        Type::Record(name, _) => Some(name.clone()),
        Type::App(name, _) => Some(name.clone()),
        _ => None,
    }
}

/// TypeExpr から型名を抽出（静的ディスパッチ用）
pub(crate) fn type_expr_to_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(_, name) => Some(name.clone()),
        _ => None,
    }
}

pub(crate) fn is_heap_like_type_name(type_name: &str) -> bool {
    !matches!(type_name, "Int" | "Float" | "Bool" | "Unit")
}

/// TypeExpr -> IR 型（簡易変換）
pub(crate) fn type_expr_to_ir(ty: &TypeExpr) -> IrType {
    match ty {
        TypeExpr::Named(_, name) => match name.as_str() {
            "Int" => IrType::I64,
            "Float" => IrType::F64,
            "Bool" => IrType::I64,
            "Unit" => IrType::I64,
            "String" => IrType::I64,
            _ => IrType::I64,
        },
        _ => IrType::I64,
    }
}
