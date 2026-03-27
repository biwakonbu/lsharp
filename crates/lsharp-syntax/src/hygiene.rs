//! P10-4: 衛生マクロシステム (Sets of Scopes 方式)
//!
//! Typed Racket の Sets of Scopes モデルに基づく名前解決。
//! 各識別子は名前に加えてスコープの集合を持ち、マクロ展開時に
//! スコープを付与・除去することで名前の衝突を防ぐ。
//!
//! ## 主要概念
//! - `ScopeId`: マクロ展開ごとに一意に振られるスコープ識別子
//! - `ScopeSet`: 識別子が属するスコープの集合
//! - `HygienicIdent`: 名前 + スコープ集合の組 (衛生的な識別子)
//! - `unhygienic`: escape hatch (anaphoric macro 用)
//!
//! ## 参考文献
//! - Matthew Flatt, "Binding as Sets of Scopes" (POPL 2016)

use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// スコープ識別子 (マクロ展開ごとに一意)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub u64);

impl fmt::Display for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scope#{}", self.0)
    }
}

/// スコープの集合 (BTreeSet で順序を保証)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeSet {
    scopes: BTreeSet<ScopeId>,
}

impl ScopeSet {
    /// 空のスコープ集合を作成
    pub fn empty() -> Self {
        Self {
            scopes: BTreeSet::new(),
        }
    }

    /// 単一スコープの集合を作成
    pub fn singleton(scope: ScopeId) -> Self {
        let mut scopes = BTreeSet::new();
        scopes.insert(scope);
        Self { scopes }
    }

    /// スコープを追加した新しい集合を返す
    pub fn add(&self, scope: ScopeId) -> Self {
        let mut new_scopes = self.scopes.clone();
        new_scopes.insert(scope);
        Self { scopes: new_scopes }
    }

    /// スコープを除去した新しい集合を返す
    pub fn remove(&self, scope: ScopeId) -> Self {
        let mut new_scopes = self.scopes.clone();
        new_scopes.remove(&scope);
        Self { scopes: new_scopes }
    }

    /// スコープを含むかチェック
    pub fn contains(&self, scope: &ScopeId) -> bool {
        self.scopes.contains(scope)
    }

    /// スコープ集合が他の集合の部分集合かチェック
    pub fn is_subset_of(&self, other: &ScopeSet) -> bool {
        self.scopes.is_subset(&other.scopes)
    }

    /// スコープ集合の要素数
    pub fn len(&self) -> usize {
        self.scopes.len()
    }

    /// 空かチェック
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    /// スコープ集合のフリップ (マクロ展開時に使用)
    /// 指定スコープが含まれていれば除去、なければ追加
    pub fn flip(&self, scope: ScopeId) -> Self {
        if self.contains(&scope) {
            self.remove(scope)
        } else {
            self.add(scope)
        }
    }

    /// 2つのスコープ集合の共通部分
    pub fn intersection(&self, other: &ScopeSet) -> Self {
        Self {
            scopes: self.scopes.intersection(&other.scopes).copied().collect(),
        }
    }
}

impl fmt::Display for ScopeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope_strs: Vec<String> = self.scopes.iter().map(|s| s.to_string()).collect();
        write!(f, "{{{}}}", scope_strs.join(", "))
    }
}

/// 衛生的な識別子: 名前 + スコープ集合
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HygienicIdent {
    /// 識別子の名前
    pub name: String,
    /// 付与されたスコープの集合
    pub scopes: ScopeSet,
    /// unhygienic フラグ (anaphoric macro 用)
    /// true の場合、スコープ情報を無視して名前のみで解決する
    pub unhygienic: bool,
}

impl HygienicIdent {
    /// 通常の衛生的な識別子を作成
    pub fn new(name: String, scopes: ScopeSet) -> Self {
        Self {
            name,
            scopes,
            unhygienic: false,
        }
    }

    /// 非衛生的な識別子を作成 (anaphoric macro 用)
    pub fn new_unhygienic(name: String) -> Self {
        Self {
            name,
            scopes: ScopeSet::empty(),
            unhygienic: true,
        }
    }

    /// スコープを追加した新しい識別子を返す
    pub fn add_scope(&self, scope: ScopeId) -> Self {
        if self.unhygienic {
            return self.clone();
        }
        Self {
            name: self.name.clone(),
            scopes: self.scopes.add(scope),
            unhygienic: false,
        }
    }

    /// スコープをフリップした新しい識別子を返す
    pub fn flip_scope(&self, scope: ScopeId) -> Self {
        if self.unhygienic {
            return self.clone();
        }
        Self {
            name: self.name.clone(),
            scopes: self.scopes.flip(scope),
            unhygienic: false,
        }
    }

    /// 2つの識別子が同一の束縛を参照するかチェック
    /// unhygienic の場合は名前のみで比較
    pub fn refers_to_same_binding(&self, other: &HygienicIdent) -> bool {
        if self.unhygienic || other.unhygienic {
            self.name == other.name
        } else {
            self.name == other.name && self.scopes == other.scopes
        }
    }
}

impl fmt::Display for HygienicIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unhygienic {
            write!(f, "{}(unhygienic)", self.name)
        } else if self.scopes.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}@{}", self.name, self.scopes)
        }
    }
}

/// スコープ ID ジェネレータ
#[derive(Debug, Clone)]
pub struct ScopeAllocator {
    next_id: u64,
}

impl ScopeAllocator {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// 新しいスコープ ID を割り当て
    pub fn alloc(&mut self) -> ScopeId {
        let id = ScopeId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Default for ScopeAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// 衛生的な名前解決テーブル
/// 名前 → [(ScopeSet, 束縛先)] のマッピング
/// 解決時は、参照側のスコープ集合に対して最大の部分集合を持つ束縛を選択する
#[derive(Debug, Clone)]
pub struct HygienicBindingTable {
    /// 名前 → 束縛のリスト (各束縛はスコープ集合と値のペア)
    bindings: HashMap<String, Vec<(ScopeSet, String)>>,
}

impl HygienicBindingTable {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// 束縛を追加
    pub fn bind(&mut self, ident: &HygienicIdent, value: String) {
        let entry = self.bindings.entry(ident.name.clone()).or_default();
        // 同じスコープ集合の束縛があれば上書き
        if let Some(existing) = entry.iter_mut().find(|(s, _)| s == &ident.scopes) {
            existing.1 = value;
        } else {
            entry.push((ident.scopes.clone(), value));
        }
    }

    /// 名前解決: 参照側のスコープ集合に対して最大の部分集合を持つ束縛を選択
    ///
    /// Sets of Scopes の解決ルール:
    /// 1. 名前が一致する束縛を全て収集
    /// 2. 各束縛のスコープ集合が参照側のスコープ集合の部分集合であるものをフィルタ
    /// 3. 最大の部分集合を持つ束縛を選択 (一意でなければ曖昧エラー)
    pub fn resolve(&self, ident: &HygienicIdent) -> Option<&str> {
        // unhygienic の場合は最後に追加された束縛を返す
        if ident.unhygienic {
            return self
                .bindings
                .get(&ident.name)
                .and_then(|entries| entries.last())
                .map(|(_, v)| v.as_str());
        }

        let entries = self.bindings.get(&ident.name)?;

        // 参照側スコープ集合の部分集合である束縛をフィルタ
        let candidates: Vec<_> = entries
            .iter()
            .filter(|(scope_set, _)| scope_set.is_subset_of(&ident.scopes))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // 最大の部分集合を持つ束縛を選択
        let mut best = &candidates[0];
        for candidate in &candidates[1..] {
            if candidate.0.len() > best.0.len() {
                best = candidate;
            }
        }

        Some(&best.1)
    }

    /// 全束縛のリストを取得 (デバッグ用)
    pub fn all_bindings(&self) -> Vec<(&str, &ScopeSet, &str)> {
        let mut result = Vec::new();
        for (name, entries) in &self.bindings {
            for (scope_set, value) in entries {
                result.push((name.as_str(), scope_set, value.as_str()));
            }
        }
        result
    }
}

impl Default for HygienicBindingTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
        let ref_ident =
            HygienicIdent::new("x".to_string(), ScopeSet::empty().add(s0).add(s1).add(s2));
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
}
