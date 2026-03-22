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
    Record { fields: Vec<FieldInfo> },
    Adt { variants: Vec<VariantInfo> },
    Alias { target: String },
    Constrained { base: String, constraints: Vec<String> },
    Trait { methods: Vec<String> },
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
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_serialization() {
        let knowledge = Knowledge {
            project: ProjectInfo {
                name: "test-project".to_string(),
                version: "0.1.0".to_string(),
            },
            functions: vec![FunctionInfo {
                name: "add".to_string(),
                params: vec![
                    ParamInfo {
                        name: "x".to_string(),
                        ty: "Int".to_string(),
                        description: Some("left operand".to_string()),
                    },
                    ParamInfo {
                        name: "y".to_string(),
                        ty: "Int".to_string(),
                        description: Some("right operand".to_string()),
                    },
                ],
                return_type: "Int".to_string(),
                doc: Some("Add two integers".to_string()),
                module: Some("Math".to_string()),
                is_private: false,
            }],
            types: vec![TypeInfo {
                name: "Point".to_string(),
                kind: TypeKind::Record {
                    fields: vec![
                        FieldInfo { name: "x".to_string(), ty: "Float".to_string() },
                        FieldInfo { name: "y".to_string(), ty: "Float".to_string() },
                    ],
                },
                type_params: vec![],
                doc: Some("A 2D point".to_string()),
            }],
            dependencies: vec![DependencyInfo {
                from: "Main".to_string(),
                to: "Math".to_string(),
                kind: DependencyKind::Import,
            }],
        };

        let json = knowledge.to_json().unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("add"));
        assert!(json.contains("Point"));
        assert!(json.contains("Main"));
    }

    #[test]
    fn test_type_kind_variants() {
        let adt = TypeKind::Adt {
            variants: vec![
                VariantInfo { name: "Some".to_string(), fields: vec!["a".to_string()] },
                VariantInfo { name: "None".to_string(), fields: vec![] },
            ],
        };
        let json = serde_json::to_string(&adt).unwrap();
        assert!(json.contains("Some"));
        assert!(json.contains("None"));
    }

    #[test]
    fn test_constrained_type() {
        let constrained = TypeKind::Constrained {
            base: "Int".to_string(),
            constraints: vec![">= 0".to_string(), "<= 100".to_string()],
        };
        let json = serde_json::to_string(&constrained).unwrap();
        assert!(json.contains(">= 0"));
    }
}
