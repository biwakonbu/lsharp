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
mod tests;
