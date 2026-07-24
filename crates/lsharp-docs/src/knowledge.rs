//! Knowledge JSON 出力
//!
//! `--emit knowledge` フラグでコンパイラが出力する
//! 型情報・関数情報・制約・依存関係の JSON を生成する。

use serde::Serialize;

/// Knowledge 出力のルート構造
#[derive(Debug, Clone, Serialize)]
pub struct Knowledge {
    /// プロジェクト情報
    pub project: ProjectInfo,
    /// 関数定義一覧
    pub functions: Vec<FunctionInfo>,
    /// 型定義一覧
    pub types: Vec<TypeInfo>,
    /// モジュール依存関係
    pub dependencies: Vec<DependencyInfo>,
}

/// プロジェクト情報
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
}

/// 関数情報
#[derive(Debug, Clone, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
    pub doc: Option<String>,
    pub module: Option<String>,
    pub is_private: bool,
}

/// パラメータ情報
#[derive(Debug, Clone, Serialize)]
pub struct ParamInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub description: Option<String>,
}

/// 型情報
#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub type_params: Vec<String>,
    pub doc: Option<String>,
}

/// 型の種別
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Record {
        fields: Vec<FieldInfo>,
    },
    Adt {
        variants: Vec<VariantInfo>,
    },
    Alias {
        target: String,
    },
    Constrained {
        base: String,
        constraints: Vec<String>,
    },
    Trait {
        methods: Vec<String>,
    },
}

/// レコードフィールド情報
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// ADT バリアント情報
#[derive(Debug, Clone, Serialize)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<String>,
}

/// モジュール依存関係
#[derive(Debug, Clone, Serialize)]
pub struct DependencyInfo {
    pub from: String,
    pub to: String,
    pub kind: DependencyKind,
}

/// 依存関係の種別
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Import,
    OpenImport,
    SelectiveImport { symbols: Vec<String> },
}

impl Knowledge {
    /// Knowledge を JSON 文字列に変換
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod tests;
