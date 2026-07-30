mod canonical_contract_check;
pub mod constraints;
pub mod evidence;
pub mod infer;
pub mod intent;
pub mod metadata_check;
pub mod metadata_contract;
pub mod metadata_migration;
pub(crate) mod regex;
pub mod types;
pub mod validation;
mod validation_identity;
pub mod validation_input;
pub mod validation_output;
pub mod validation_source;

#[cfg(test)]
mod diagnostic_tests {
    use crate::infer::{TypeError, TypeErrorCode};
    use crate::types::{Kind, Type};
    use lsharp_syntax::span::Span;

    #[test]
    fn infer_error_types_remain_exported_from_infer_module() {
        let error = crate::infer::TypeError::UndefinedVar {
            name: "missing".to_string(),
            span: Span::new(1, 2),
        };
        assert_eq!(error.code(), "LS1001");
    }

    #[test]
    fn type_errors_expose_stable_codes_and_spans_for_all_variants() {
        let span = Span::new(3, 8);
        let errors = vec![
            (
                TypeError::Mismatch {
                    expected: Type::int(),
                    found: Type::bool(),
                    span,
                    error_code: TypeErrorCode::IfCondition,
                },
                "LS1002",
            ),
            (
                TypeError::Mismatch {
                    expected: Type::int(),
                    found: Type::bool(),
                    span,
                    error_code: TypeErrorCode::ArgMismatch,
                },
                "LS1004",
            ),
            (
                TypeError::InfiniteType {
                    var: 0,
                    ty: Type::int(),
                    span,
                },
                "LS1003",
            ),
            (
                TypeError::UndefinedVar {
                    name: "missing".to_string(),
                    span,
                },
                "LS1001",
            ),
            (
                TypeError::UndefinedConstructor {
                    name: "Missing".to_string(),
                    span,
                },
                "LS1005",
            ),
            (
                TypeError::ArityMismatch {
                    expected: 1,
                    found: 2,
                    span,
                },
                "LS1004",
            ),
            (
                TypeError::UndefinedRecord {
                    name: "Missing".to_string(),
                    span,
                },
                "LS1006",
            ),
            (
                TypeError::UndefinedField {
                    record_name: "Point".to_string(),
                    field_name: "z".to_string(),
                    span,
                },
                "LS1007",
            ),
            (
                TypeError::RecursiveAlias {
                    name: "Loop".to_string(),
                    span,
                },
                "LS1008",
            ),
            (
                TypeError::UndefinedAlias {
                    name: "Missing".to_string(),
                    span,
                },
                "LS1009",
            ),
            (
                TypeError::UndefinedTrait {
                    name: "Missing".to_string(),
                    span,
                },
                "LS1010",
            ),
            (
                TypeError::MissingImpl {
                    trait_name: "Show".to_string(),
                    type_name: "Int".to_string(),
                    span,
                },
                "LS1011",
            ),
            (
                TypeError::MismatchWithAlias {
                    expected: Type::int(),
                    found: Type::bool(),
                    alias_name: "Number".to_string(),
                    expanded: Type::int(),
                    span,
                    error_code: TypeErrorCode::General,
                },
                "LS1002",
            ),
            (
                TypeError::KindMismatch {
                    type_name: "Int".to_string(),
                    trait_name: "Functor".to_string(),
                    expected_kind: Kind::unary(),
                    actual_kind: Kind::star(),
                    span,
                },
                "LS1013",
            ),
        ];

        for (error, expected_code) in errors {
            assert_eq!(error.code(), expected_code);
            assert_eq!(error.span(), Some(span));
        }
    }
}
