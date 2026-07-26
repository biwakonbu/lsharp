use lsharp_types::constraints::{ConstraintResult, eval_string_constraint};
use lsharp_types::types::ConstraintDef;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn bounded_repeat_accepts_exactly_the_generated_length_interval(
        min in 0usize..=4,
        extra in 0usize..=4,
        length in 0usize..=9,
    ) {
        let max = min + extra;
        let pattern = format!("^a{{{min},{max}}}$");
        let value = "a".repeat(length);
        let result = eval_string_constraint(&value, &ConstraintDef::Matches(pattern));

        prop_assert_eq!(
            matches!(result, ConstraintResult::Satisfied),
            (min..=max).contains(&length),
            "bounded repeat の受理範囲が不正: min={}, max={}, length={}",
            min,
            max,
            length
        );
    }

    #[test]
    fn open_ended_repeat_accepts_every_length_at_or_above_minimum(
        min in 0usize..=5,
        length in 0usize..=10,
    ) {
        let pattern = format!("^a{{{min},}}$");
        let value = "a".repeat(length);
        let result = eval_string_constraint(&value, &ConstraintDef::Matches(pattern));

        prop_assert_eq!(
            matches!(result, ConstraintResult::Satisfied),
            length >= min,
            "open-ended repeat の下限が不正: min={}, length={}",
            min,
            length
        );
    }
}
