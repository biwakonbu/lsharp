use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// 型変数の識別子
pub type TypeVarId = u32;

/// 種 (Kind) -- 型の型
///
/// HKT をサポートするための種システム。
/// `*` は具体型の種、`* -> *` は型コンストラクタの種。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// 具体型の種 (*)
    Star,
    /// 型コンストラクタの種 (k1 -> k2)
    Arrow(Box<Kind>, Box<Kind>),
}

impl Kind {
    /// * (具体型)
    pub fn star() -> Self {
        Kind::Star
    }

    /// * -> * (1引数型コンストラクタ)
    pub fn unary() -> Self {
        Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))
    }

    /// * -> * -> * (2引数型コンストラクタ)
    pub fn binary() -> Self {
        Kind::Arrow(
            Box::new(Kind::Star),
            Box::new(Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))),
        )
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Star => write!(f, "*"),
            Kind::Arrow(k1, k2) => match k1.as_ref() {
                Kind::Arrow(_, _) => write!(f, "({k1}) -> {k2}"),
                _ => write!(f, "{k1} -> {k2}"),
            },
        }
    }
}

/// 型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// 具体型 (Int, Float, String, Bool, Unit)
    Con(String),
    /// 型変数 (推論用)
    Var(TypeVarId),
    /// 関数型 (引数型 -> 戻り値型)
    Fun(Vec<Type>, Box<Type>),
    /// 型適用 (Option Int, Result String Int)
    App(String, Vec<Type>),
    /// レコード型
    Record(String, Vec<(String, Type)>),
}

impl Type {
    pub fn int() -> Self {
        Type::Con("Int".to_string())
    }
    pub fn float() -> Self {
        Type::Con("Float".to_string())
    }
    pub fn string() -> Self {
        Type::Con("String".to_string())
    }
    pub fn bool() -> Self {
        Type::Con("Bool".to_string())
    }
    pub fn unit() -> Self {
        Type::Con("Unit".to_string())
    }

    /// この型に含まれる自由型変数を収集
    pub fn free_vars(&self) -> Vec<TypeVarId> {
        match self {
            Type::Con(_) => Vec::new(),
            Type::Var(id) => vec![*id],
            Type::Fun(params, ret) => {
                let mut vars: Vec<TypeVarId> = params.iter().flat_map(|p| p.free_vars()).collect();
                vars.extend(ret.free_vars());
                vars.sort();
                vars.dedup();
                vars
            }
            Type::App(_, args) => {
                let mut vars: Vec<TypeVarId> = args.iter().flat_map(|a| a.free_vars()).collect();
                vars.sort();
                vars.dedup();
                vars
            }
            Type::Record(_, fields) => {
                let mut vars: Vec<TypeVarId> =
                    fields.iter().flat_map(|(_, t)| t.free_vars()).collect();
                vars.sort();
                vars.dedup();
                vars
            }
        }
    }

    /// 型変数を置換する
    pub fn apply_subst(&self, subst: &Substitution) -> Type {
        match self {
            Type::Con(name) => Type::Con(name.clone()),
            // 型変数の連鎖 (t0 -> t1 -> ... -> 具体型) を再帰で辿ると
            // 深い if 連鎖や compose の蓄積でスタックオーバーフローになるため、ループで潰す。
            Type::Var(id) => {
                let mut id = *id;
                let mut seen: BTreeSet<TypeVarId> = BTreeSet::new();
                loop {
                    if !seen.insert(id) {
                        // 置換マップに変数サイクルがある場合のフェイルセーフ（通常は occurs check で防ぐ）
                        return Type::Var(id);
                    }
                    match subst.get(id) {
                        None => return Type::Var(id),
                        Some(Type::Var(next)) => id = *next,
                        Some(other) => return other.clone().apply_subst(subst),
                    }
                }
            }
            Type::Fun(params, ret) => Type::Fun(
                params.iter().map(|p| p.apply_subst(subst)).collect(),
                Box::new(ret.apply_subst(subst)),
            ),
            Type::App(name, args) => Type::App(
                name.clone(),
                args.iter().map(|a| a.apply_subst(subst)).collect(),
            ),
            Type::Record(name, fields) => Type::Record(
                name.clone(),
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), t.apply_subst(subst)))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Con(name) => write!(f, "{name}"),
            Type::Var(id) => write!(f, "t{id}"),
            Type::Fun(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::App(name, args) => {
                write!(f, "({name}")?;
                for a in args {
                    write!(f, " {a}")?;
                }
                write!(f, ")")
            }
            Type::Record(name, fields) => {
                write!(f, "{{{name}")?;
                for (n, t) in fields {
                    write!(f, " {n}: {t}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// 型スキーム（多相型）: forall a b . Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScheme {
    /// 束縛された型変数
    pub vars: Vec<TypeVarId>,
    /// トレイト制約
    pub constraints: Vec<TraitConstraint>,
    /// 本体の型
    pub ty: Type,
}

impl TypeScheme {
    /// 単相型をスキームに変換（束縛変数なし）
    pub fn mono(ty: Type) -> Self {
        TypeScheme {
            vars: Vec::new(),
            constraints: Vec::new(),
            ty,
        }
    }

    /// 自由型変数（束縛されていない型変数）
    pub fn free_vars(&self) -> Vec<TypeVarId> {
        self.ty
            .free_vars()
            .into_iter()
            .filter(|v| !self.vars.contains(v))
            .collect()
    }

    /// 置換を適用（束縛変数は除外）
    pub fn apply_subst(&self, subst: &Substitution) -> TypeScheme {
        let restricted = subst.without(&self.vars);
        TypeScheme {
            vars: self.vars.clone(),
            constraints: self.constraints.clone(),
            ty: self.ty.apply_subst(&restricted),
        }
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.vars.is_empty() {
            write!(f, "{}", self.ty)
        } else {
            write!(f, "forall")?;
            for v in &self.vars {
                write!(f, " t{v}")?;
            }
            if !self.constraints.is_empty() {
                write!(f, " where")?;
                for (i, c) in self.constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, " {} t{}", c.trait_name, c.type_var)?;
                }
            }
            write!(f, ". {}", self.ty)
        }
    }
}

/// 型代入（型変数 -> 型 のマッピング）
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    map: BTreeMap<TypeVarId, Type>,
}

impl Substitution {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// 型変数に型を割り当てる
    pub fn insert(&mut self, var: TypeVarId, ty: Type) {
        self.map.insert(var, ty);
    }

    /// 型変数の割り当てを取得
    pub fn get(&self, var: TypeVarId) -> Option<&Type> {
        self.map.get(&var)
    }

    /// 指定された型変数を除外した置換を返す
    pub fn without(&self, vars: &[TypeVarId]) -> Substitution {
        Substitution {
            map: self
                .map
                .iter()
                .filter(|(k, _)| !vars.contains(k))
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        }
    }

    /// 2つの置換を合成 (self の後に other を適用)
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();
        // other の置換に self を適用
        for (var, ty) in &other.map {
            result.insert(*var, ty.apply_subst(self));
        }
        // self の置換を追加（other にない場合のみ）
        for (var, ty) in &self.map {
            result.map.entry(*var).or_insert_with(|| ty.clone());
        }
        result
    }
}

/// 型変数の生成器
#[derive(Debug, Default)]
pub struct TypeVarGen {
    next_id: TypeVarId,
}

impl TypeVarGen {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// 新しい型変数を生成
    pub fn fresh(&mut self) -> Type {
        let id = self.next_id;
        self.next_id += 1;
        Type::Var(id)
    }

    /// 新しい型変数 ID を生成
    pub fn fresh_id(&mut self) -> TypeVarId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// 型環境（変数名 -> 型スキーム）
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    bindings: BTreeMap<String, TypeScheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
        }
    }

    /// 変数を束縛する
    pub fn insert(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    /// 変数を環境から除去する
    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    /// 変数の型スキームを取得
    pub fn get(&self, name: &str) -> Option<&TypeScheme> {
        self.bindings.get(name)
    }

    /// 環境を拡張した新しい環境を返す
    pub fn extend(&self, name: String, scheme: TypeScheme) -> TypeEnv {
        let mut env = self.clone();
        env.insert(name, scheme);
        env
    }

    /// 環境の自由型変数
    pub fn free_vars(&self) -> Vec<TypeVarId> {
        let mut vars: Vec<TypeVarId> = self.bindings.values().flat_map(|s| s.free_vars()).collect();
        vars.sort();
        vars.dedup();
        vars
    }

    /// 置換を適用
    pub fn apply_subst(&self, subst: &Substitution) -> TypeEnv {
        TypeEnv {
            bindings: self
                .bindings
                .iter()
                .map(|(k, v)| (k.clone(), v.apply_subst(subst)))
                .collect(),
        }
    }
}

/// レコード型情報
#[derive(Debug, Clone)]
pub struct RecordInfo {
    pub name: String,
    pub type_params: Vec<TypeVarId>,
    pub fields: Vec<(String, Type)>,
}

/// トレイト制約
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitConstraint {
    pub trait_name: String,
    pub type_var: TypeVarId,
}

/// トレイト定義情報
#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub type_param: TypeVarId,
    pub methods: Vec<(String, TypeScheme)>,
}

/// トレイト実装情報
#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub trait_name: String,
    pub type_name: String,
    pub methods: Vec<(String, Type)>,
}

/// 制約付き型情報
#[derive(Debug, Clone)]
pub struct ConstrainedTypeInfo {
    pub name: String,
    pub base_type: Type,
    pub constraints: Vec<ConstraintDef>,
}

/// 制約定義（実行時検証用）
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintDef {
    /// 下限 (>= N)
    Gte(i64),
    /// 上限 (<= N)
    Lte(i64),
    /// 範囲 (range lo hi)
    Range(i64, i64),
    /// 正規表現 (matches "pattern")
    Matches(String),
    /// 最小長 (min-length N)
    MinLength(usize),
    /// 最大長 (max-length N)
    MaxLength(usize),
    /// 値リスト (one-of [v1 v2 ...])
    OneOf(Vec<i64>),
    /// 述語関数 (satisfies fn-name)
    Satisfies(String),
}

#[cfg(test)]
mod apply_subst_tests {
    use super::{Substitution, Type};

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
}
