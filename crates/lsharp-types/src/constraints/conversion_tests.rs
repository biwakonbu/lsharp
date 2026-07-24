use super::*;
use crate::types::{ConstrainedTypeInfo, ConstraintDef, Type};
use std::collections::HashMap;

fn make_registry() -> HashMap<String, ConstrainedTypeInfo> {
    let mut registry = HashMap::new();

    // Natural: Int (>= 0)
    registry.insert(
        "Natural".to_string(),
        ConstrainedTypeInfo {
            name: "Natural".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Gte(0)],
        },
    );

    // Percentage: Natural (<= 100)
    // 継承後: (>= 0, <= 100)
    registry.insert(
        "Percentage".to_string(),
        ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        },
    );

    // Priority: Percentage (<= 10)
    // 継承後: (>= 0, <= 100, <= 10)
    registry.insert(
        "Priority".to_string(),
        ConstrainedTypeInfo {
            name: "Priority".to_string(),
            base_type: Type::Con("Percentage".to_string()),
            constraints: vec![ConstraintDef::Lte(10)],
        },
    );

    // Port: Int (range 1 65535)
    registry.insert(
        "Port".to_string(),
        ConstrainedTypeInfo {
            name: "Port".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Range(1, 65535)],
        },
    );

    registry
}

#[test]
fn test_upcast_child_to_parent() {
    // Percentage -> Natural: Percentage は Natural の子型なのでアップキャスト（安全）
    let registry = make_registry();
    let info = analyze_conversion("Percentage", "Natural", &registry);
    assert_eq!(info.kind, ConversionKind::Upcast);
    assert!(info.extra_checks.is_empty());
}

#[test]
fn test_upcast_grandchild_to_grandparent() {
    // Priority -> Natural: Priority は Natural の孫型なのでアップキャスト（安全）
    let registry = make_registry();
    let info = analyze_conversion("Priority", "Natural", &registry);
    assert_eq!(info.kind, ConversionKind::Upcast);
}

#[test]
fn test_downcast_parent_to_child() {
    // Natural -> Percentage: ダウンキャスト（制約追加チェックが必要）
    let registry = make_registry();
    let info = analyze_conversion("Natural", "Percentage", &registry);
    assert_eq!(info.kind, ConversionKind::Downcast);
    assert!(!info.extra_checks.is_empty());
}

#[test]
fn test_downcast_parent_to_grandchild() {
    // Natural -> Priority: ダウンキャスト（さらに制約が厳しい）
    let registry = make_registry();
    let info = analyze_conversion("Natural", "Priority", &registry);
    assert_eq!(info.kind, ConversionKind::Downcast);
}

#[test]
fn test_incompatible_unrelated_types() {
    // Port -> Priority: 無関係な制約付き型は変換不可
    let registry = make_registry();
    let info = analyze_conversion("Port", "Priority", &registry);
    assert_eq!(info.kind, ConversionKind::Incompatible);
}

#[test]
fn test_same_type_conversion() {
    // Natural -> Natural: 同じ型はアップキャスト（自明に安全）
    let registry = make_registry();
    let info = analyze_conversion("Natural", "Natural", &registry);
    assert_eq!(info.kind, ConversionKind::Upcast);
}

#[test]
fn test_unknown_type_conversion() {
    // Unknown -> Natural: 不明型はアップキャスト（制約なし -> 制約なし相当）
    let registry = make_registry();
    let info = analyze_conversion("Unknown", "Natural", &registry);
    // 不明型は制約が空で、Naturalの制約をパスしない → ダウンキャスト or Incompatible
    // 空制約の子がNaturalの親制約を満たせるかどうかで判定
    assert!(matches!(
        info.kind,
        ConversionKind::Downcast | ConversionKind::Incompatible
    ));
}
