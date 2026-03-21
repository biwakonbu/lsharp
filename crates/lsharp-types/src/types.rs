use std::collections::BTreeMap;
use std::fmt;

/// 型変数の識別子
pub type TypeVarId = u32;

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
        }
    }

    /// 型変数を置換する
    pub fn apply_subst(&self, subst: &Substitution) -> Type {
        match self {
            Type::Con(name) => Type::Con(name.clone()),
            Type::Var(id) => {
                if let Some(ty) = subst.get(*id) {
                    // 置換結果にさらに置換を適用（推移的閉包）
                    ty.apply_subst(subst)
                } else {
                    Type::Var(*id)
                }
            }
            Type::Fun(params, ret) => Type::Fun(
                params.iter().map(|p| p.apply_subst(subst)).collect(),
                Box::new(ret.apply_subst(subst)),
            ),
            Type::App(name, args) => {
                Type::App(name.clone(), args.iter().map(|a| a.apply_subst(subst)).collect())
            }
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
        }
    }
}

/// 型スキーム（多相型）: forall a b . Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScheme {
    /// 束縛された型変数
    pub vars: Vec<TypeVarId>,
    /// 本体の型
    pub ty: Type,
}

impl TypeScheme {
    /// 単相型をスキームに変換（束縛変数なし）
    pub fn mono(ty: Type) -> Self {
        TypeScheme {
            vars: Vec::new(),
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
            write!(f, ". {}", self.ty)
        }
    }
}

/// 型代入（型変数 → 型 のマッピング）
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

/// 型環境（変数名 → 型スキーム）
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
        let mut vars: Vec<TypeVarId> = self
            .bindings
            .values()
            .flat_map(|s| s.free_vars())
            .collect();
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
