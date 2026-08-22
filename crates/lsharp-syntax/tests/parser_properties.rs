use proptest::prelude::*;

fn expression_strategy() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        (0i64..128).prop_map(|value| value.to_string()),
        Just("true".to_string()),
        Just("false".to_string()),
    ];

    leaf.prop_recursive(3, 32, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(left, right)| format!("(+ {left} {right})")),
            (inner.clone(), inner.clone())
                .prop_map(|(left, right)| format!("(if true {left} {right})")),
            inner
                .clone()
                .prop_map(|value| format!("(let [x {value}] x)")),
            (inner.clone(), inner).prop_map(|(first, second)| format!("(do {first} {second})")),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn parser_never_panics_on_generated_balanced_forms(expression in expression_strategy()) {
        let source = format!("(defn main [] {expression})");
        let result = std::panic::catch_unwind(|| lsharp_syntax::parse(&source));

        prop_assert!(result.is_ok(), "parser panicked for generated source: {source}");
        if let Err(error) = result.expect("panic was checked above") {
            prop_assert!(
                error.code().starts_with("LS"),
                "parse failure must expose a stable LS code: {error}"
            );
        }
    }
}
