use super::{Substitution, Type, TypeScheme};

#[test]
fn substitution_reports_empty_without_allocating_a_map() {
    let empty = Substitution::new();
    assert!(empty.is_empty());

    let mut non_empty = Substitution::new();
    non_empty.insert(0, Type::int());
    assert!(!non_empty.is_empty());
}

#[test]
fn apply_subst_preserves_mono_and_bound_scheme_semantics() {
    let mut substitution = Substitution::new();
    substitution.insert(0, Type::int());

    let mono = TypeScheme::mono(Type::Var(0));
    assert_eq!(mono.apply_subst(&substitution).ty, Type::int());

    let bound = TypeScheme {
        vars: vec![0],
        constraints: Vec::new(),
        ty: Type::Var(0),
    };
    assert_eq!(bound.apply_subst(&substitution).ty, Type::Var(0));
}

/// 長い型変数連鎖でも apply_subst がスタックを食いつぶさないこと（selfhost Lower* compile 退避用）
#[test]
fn apply_subst_resolves_long_var_chain() {
    let mut s = Substitution::new();
    for i in 0..64u32 {
        s.insert(i, Type::Var(i + 1));
    }
    s.insert(64, Type::int());
    assert_eq!(Type::Var(0).apply_subst(&s), Type::int());
}

/// 置換に変数サイクルがある場合は無限ループせず打ち切る
#[test]
fn apply_subst_var_cycle_is_safe() {
    let mut s = Substitution::new();
    s.insert(0, Type::Var(1));
    s.insert(1, Type::Var(0));
    let t = Type::Var(0).apply_subst(&s);
    assert_eq!(t, Type::Var(0));
}
