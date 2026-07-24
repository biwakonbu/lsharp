//! 衛生マクロの回帰テスト

use super::*;

#[test]
fn test_scope_id_display() {
    let id = ScopeId(42);
    assert_eq!(format!("{id}"), "scope#42");
}

#[test]
fn test_scope_set_operations() {
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);
    let s2 = ScopeId(2);

    let empty = ScopeSet::empty();
    assert!(empty.is_empty());

    let set1 = empty.add(s0).add(s1);
    assert_eq!(set1.len(), 2);
    assert!(set1.contains(&s0));
    assert!(set1.contains(&s1));
    assert!(!set1.contains(&s2));

    let set2 = set1.remove(s0);
    assert_eq!(set2.len(), 1);
    assert!(!set2.contains(&s0));
    assert!(set2.contains(&s1));
}

#[test]
fn test_scope_set_flip() {
    let s0 = ScopeId(0);
    let set = ScopeSet::empty();

    let flipped1 = set.flip(s0);
    assert!(flipped1.contains(&s0));

    let flipped2 = flipped1.flip(s0);
    assert!(!flipped2.contains(&s0));
}

#[test]
fn test_scope_set_subset() {
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);
    let s2 = ScopeId(2);

    let small = ScopeSet::empty().add(s0);
    let big = ScopeSet::empty().add(s0).add(s1).add(s2);

    assert!(small.is_subset_of(&big));
    assert!(!big.is_subset_of(&small));
    assert!(ScopeSet::empty().is_subset_of(&small));
}

#[test]
fn test_hygienic_ident_basic() {
    let s0 = ScopeId(0);
    let ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    assert_eq!(ident.name, "x");
    assert!(ident.scopes.contains(&s0));
    assert!(!ident.unhygienic);
}

#[test]
fn test_hygienic_ident_add_scope() {
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);
    let ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    let with_scope = ident.add_scope(s1);
    assert!(with_scope.scopes.contains(&s0));
    assert!(with_scope.scopes.contains(&s1));
}

#[test]
fn test_unhygienic_ident() {
    let ident = HygienicIdent::new_unhygienic("it".to_string());
    assert!(ident.unhygienic);
    assert!(ident.scopes.is_empty());

    // unhygienic は名前のみで比較
    let s0 = ScopeId(0);
    let other = HygienicIdent::new("it".to_string(), ScopeSet::singleton(s0));
    assert!(ident.refers_to_same_binding(&other));
}

#[test]
fn test_unhygienic_scope_operations_noop() {
    let ident = HygienicIdent::new_unhygienic("it".to_string());
    let s0 = ScopeId(0);

    // unhygienic 識別子へのスコープ追加は無視される
    let with_scope = ident.add_scope(s0);
    assert!(with_scope.unhygienic);
    assert!(with_scope.scopes.is_empty());
}

#[test]
fn test_scope_allocator() {
    let mut alloc = ScopeAllocator::new();
    let s0 = alloc.alloc();
    let s1 = alloc.alloc();
    let s2 = alloc.alloc();
    assert_eq!(s0, ScopeId(0));
    assert_eq!(s1, ScopeId(1));
    assert_eq!(s2, ScopeId(2));
}

#[test]
fn test_binding_table_simple() {
    let mut table = HygienicBindingTable::new();
    let s0 = ScopeId(0);
    let ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    table.bind(&ident, "local_x".to_string());

    // 同じスコープで解決できる
    let result = table.resolve(&ident);
    assert_eq!(result, Some("local_x"));
}

#[test]
fn test_binding_table_scope_mismatch() {
    let mut table = HygienicBindingTable::new();
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);

    let bind_ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    table.bind(&bind_ident, "local_x".to_string());

    // 異なるスコープでは解決できない
    let ref_ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s1));
    let result = table.resolve(&ref_ident);
    assert_eq!(result, None);
}

#[test]
fn test_binding_table_subset_resolution() {
    // Sets of Scopes の核心: 参照側のスコープ集合の部分集合で解決
    let mut table = HygienicBindingTable::new();
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);
    let s2 = ScopeId(2);

    // スコープ {s0} で束縛
    let bind1 = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    table.bind(&bind1, "outer_x".to_string());

    // スコープ {s0, s1} で束縛 (シャドウイング)
    let bind2 = HygienicIdent::new("x".to_string(), ScopeSet::empty().add(s0).add(s1));
    table.bind(&bind2, "inner_x".to_string());

    // 参照 {s0, s1, s2} → inner_x を解決 ({s0, s1} が最大部分集合)
    let ref_ident = HygienicIdent::new("x".to_string(), ScopeSet::empty().add(s0).add(s1).add(s2));
    assert_eq!(table.resolve(&ref_ident), Some("inner_x"));

    // 参照 {s0, s2} → outer_x を解決 ({s0} が最大部分集合)
    let ref_ident2 = HygienicIdent::new("x".to_string(), ScopeSet::empty().add(s0).add(s2));
    assert_eq!(table.resolve(&ref_ident2), Some("outer_x"));
}

#[test]
fn test_binding_table_unhygienic_resolution() {
    let mut table = HygienicBindingTable::new();
    let s0 = ScopeId(0);

    let bind = HygienicIdent::new("it".to_string(), ScopeSet::singleton(s0));
    table.bind(&bind, "anaphoric_it".to_string());

    // unhygienic 識別子はスコープを無視して解決
    let ref_ident = HygienicIdent::new_unhygienic("it".to_string());
    assert_eq!(table.resolve(&ref_ident), Some("anaphoric_it"));
}

#[test]
fn test_hygienic_macro_scenario() {
    // マクロ展開の衛生性シナリオ:
    // (defmacro swap [a b] '(let [tmp ~a] (do (set! ~a ~b) (set! ~b tmp))))
    // ユーザーコードで (let [tmp 1] (swap tmp x)) を展開した場合、
    // マクロ内の tmp とユーザーの tmp は異なるスコープを持つべき

    let mut alloc = ScopeAllocator::new();
    let mut table = HygienicBindingTable::new();

    let user_scope = alloc.alloc(); // scope#0: ユーザーコード
    let macro_scope = alloc.alloc(); // scope#1: マクロ展開

    // ユーザーの tmp: スコープ {user_scope}
    let user_tmp = HygienicIdent::new("tmp".to_string(), ScopeSet::singleton(user_scope));
    table.bind(&user_tmp, "user_tmp_value".to_string());

    // マクロの tmp: スコープ {user_scope, macro_scope}
    let macro_tmp = HygienicIdent::new(
        "tmp".to_string(),
        ScopeSet::empty().add(user_scope).add(macro_scope),
    );
    table.bind(&macro_tmp, "macro_tmp_value".to_string());

    // マクロ内から tmp を参照: スコープ {user_scope, macro_scope}
    let macro_ref = HygienicIdent::new(
        "tmp".to_string(),
        ScopeSet::empty().add(user_scope).add(macro_scope),
    );
    assert_eq!(table.resolve(&macro_ref), Some("macro_tmp_value"));

    // ユーザーから tmp を参照: スコープ {user_scope}
    let user_ref = HygienicIdent::new("tmp".to_string(), ScopeSet::singleton(user_scope));
    assert_eq!(table.resolve(&user_ref), Some("user_tmp_value"));
}

#[test]
fn test_ident_display() {
    let s0 = ScopeId(0);
    let ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));
    assert!(format!("{ident}").contains("x@"));

    let empty_ident = HygienicIdent::new("y".to_string(), ScopeSet::empty());
    assert_eq!(format!("{empty_ident}"), "y");

    let unhyg = HygienicIdent::new_unhygienic("it".to_string());
    assert!(format!("{unhyg}").contains("unhygienic"));
}

#[test]
fn test_scope_set_intersection() {
    let s0 = ScopeId(0);
    let s1 = ScopeId(1);
    let s2 = ScopeId(2);

    let set1 = ScopeSet::empty().add(s0).add(s1);
    let set2 = ScopeSet::empty().add(s1).add(s2);

    let intersection = set1.intersection(&set2);
    assert_eq!(intersection.len(), 1);
    assert!(intersection.contains(&s1));
}

#[test]
fn test_binding_table_overwrite() {
    let mut table = HygienicBindingTable::new();
    let s0 = ScopeId(0);
    let ident = HygienicIdent::new("x".to_string(), ScopeSet::singleton(s0));

    table.bind(&ident, "first".to_string());
    table.bind(&ident, "second".to_string());

    assert_eq!(table.resolve(&ident), Some("second"));
}
