//! Typed AST -> IR 変換 (Lowering)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;
use lsharp_types::infer::ExprTypeKey;
use lsharp_types::types::Type;

#[cfg(test)]
use crate::GcTypeKind;
use crate::{GcTypeDef, IrType};

mod context;
mod decl;
mod expr;
mod heap_helpers;
mod pattern;
mod pattern_wasmgc;
mod program;
mod state;
mod type_helpers;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod heap_helpers_tests;
#[cfg(test)]
mod state_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type_helpers_tests;

pub(crate) use context::FuncCtx;
pub(crate) use heap_helpers::{emit_tag_pointer, emit_untag_pointer};
#[cfg(test)]
pub(crate) use heap_helpers::emit_write_heap_header;
pub use heap_helpers::{
    HEAP_TAG_ADT, HEAP_TAG_CLOSURE, HEAP_TAG_HASHMAP, HEAP_TAG_RECORD, HEAP_TAG_REF,
    HEAP_TAG_STRING, HEAP_TAG_VECTOR,
};
pub(crate) use type_helpers::{
    is_heap_like_type_name, type_expr_to_ir, type_expr_to_name, type_to_name,
};
pub use type_helpers::type_to_ir;

/// Lowering エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum LowerError {
    #[error("未サポートの式: {msg}")]
    Unsupported { msg: String, span: Option<Span> },

    #[error("未定義の関数: {name}")]
    UndefinedFunction { name: String, span: Option<Span> },

    #[error("GC root slot lifetime の不整合: {error}")]
    RootLifetime {
        #[source]
        error: crate::root_lifetime::RootLifetimeError,
    },
}

/// Lowering が値表現を選ぶ backend。
///
/// `Linear` は既存の tagged pointer / linear-memory 表現を維持し、
/// `WasmGc` はレコード値を `IrType::Ref` として WasmGC emitter へ渡す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowerBackend {
    Linear,
    WasmGc,
}

impl LowerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "LS3001",
            Self::UndefinedFunction { .. } => "LS3002",
            Self::RootLifetime { .. } => "LS3003",
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Unsupported { span, .. } | Self::UndefinedFunction { span, .. } => *span,
            Self::RootLifetime { .. } => None,
        }
    }
}

/// Lowering コンテキスト
pub struct Lower {
    /// 値表現を選択する backend。
    pub(crate) backend: LowerBackend,
    /// 関数名 -> 関数インデックスのマッピング
    pub(crate) func_indices: HashMap<String, u32>,
    /// import 関数の数（ユーザー関数のインデックスオフセット）
    pub(crate) import_count: u32,
    /// 型推論結果
    pub(crate) type_results: HashMap<String, Type>,
    /// 式レベルの型推論結果
    pub(crate) expr_type_results: HashMap<ExprTypeKey, Type>,
    /// レコード型名 -> GC 型インデックス
    pub(crate) record_type_indices: HashMap<String, u32>,
    /// レコード型名 -> フィールド名リスト（順序保持）
    pub(crate) record_fields: HashMap<String, Vec<String>>,
    /// GC 型定義のリスト
    pub(crate) gc_types: Vec<GcTypeDef>,
    /// トレイト実装メソッドの解決テーブル
    /// (trait_name, type_name, method_name) -> 関数名
    pub(crate) trait_method_impls: HashMap<(String, String, String), String>,
    /// トレイトメソッド名 -> トレイト名の逆引きテーブル（静的ディスパッチ用）
    /// method_name -> Vec<trait_name>
    pub(crate) trait_method_names: HashMap<String, Vec<String>>,
    /// ADT バリアント名 -> (GC 型インデックス, タグ値)
    pub(crate) adt_variant_indices: HashMap<String, (u32, i32)>,
    /// ADT 型名 -> バリアント情報リスト [(name, gc_idx, tag, field_count)]
    pub(crate) adt_type_info: HashMap<String, Vec<(String, u32, i32, usize)>>,
    /// ADT 型名 -> WasmGC struct 型インデックス
    pub(crate) adt_type_indices: HashMap<String, u32>,
    /// ADT バリアント名 -> payload field の WasmGC 型
    pub(crate) adt_variant_field_types: HashMap<String, Vec<IrType>>,
    /// ADT バリアント名 -> 共通 GC struct 内の payload field offset
    pub(crate) adt_variant_field_offsets: HashMap<String, Vec<u32>>,
    /// ADT バリアント名 -> payload field のソース型名
    pub(crate) adt_variant_field_type_names: HashMap<String, Vec<Option<String>>>,
    /// ADT 型名 -> 共通 payload slot の WasmGC 型
    pub(crate) adt_slot_types: HashMap<String, Vec<IrType>>,
    /// WasmGC ADT の表現上の未対応理由
    pub(crate) adt_field_errors: Vec<String>,
    /// 文字列定数データ [(label, bytes)]
    pub(crate) string_data: Vec<(String, Vec<u8>)>,
    /// 次の文字列データオフセット
    pub(crate) string_offset: u32,
    /// Computation Builder 情報（ビルダー名 -> (bind関数名, return関数名)）
    pub(crate) computation_builders: HashMap<String, (String, String)>,
    /// WasmGC String の byte array type index
    pub(crate) string_array_type_index: Option<u32>,
    /// Lambda Lifting: リフトされた関数のリスト
    pub(crate) lifted_functions: Vec<crate::Function>,
    /// Lambda Lifting: 一意な Lambda 名生成用カウンター
    pub(crate) lambda_counter: u32,
    /// Lambda Lifting: リフトされた関数名 -> 関数インデックスのマッピング
    pub(crate) lifted_func_indices: HashMap<String, u32>,
    /// Lambda Lifting: 次に割り当てる関数インデックス
    pub(crate) next_func_idx: u32,
    /// lower_program 後半で追加登録する補助関数のインデックスカーソル
    pub(crate) late_func_idx: u32,
}

/// Private 宣言を展開して内部の宣言を返す
pub(crate) fn unwrap_private(decl: &Decl) -> &Decl {
    match decl {
        Decl::Private { inner, .. } => unwrap_private(inner),
        other => other,
    }
}

impl Lower {
    pub fn new() -> Self {
        Self::with_backend(LowerBackend::Linear)
    }

    /// backend を明示して lowering コンテキストを作成する。
    pub fn with_backend(backend: LowerBackend) -> Self {
        Self {
            backend,
            func_indices: HashMap::new(),
            import_count: 0,
            type_results: HashMap::new(),
            expr_type_results: HashMap::new(),
            record_type_indices: HashMap::new(),
            record_fields: HashMap::new(),
            gc_types: Vec::new(),
            trait_method_impls: HashMap::new(),
            trait_method_names: HashMap::new(),
            adt_variant_indices: HashMap::new(),
            adt_type_info: HashMap::new(),
            adt_type_indices: HashMap::new(),
            adt_variant_field_types: HashMap::new(),
            adt_variant_field_offsets: HashMap::new(),
            adt_variant_field_type_names: HashMap::new(),
            adt_slot_types: HashMap::new(),
            adt_field_errors: Vec::new(),
            string_data: Vec::new(),
            string_offset: 512, // 文字列データの開始位置（メモリ先頭は数値変換バッファ用）
            computation_builders: HashMap::new(),
            string_array_type_index: None,
            lifted_functions: Vec::new(),
            lambda_counter: 0,
            lifted_func_indices: HashMap::new(),
            next_func_idx: 0,
            late_func_idx: 0,
        }
    }

    pub fn backend(&self) -> LowerBackend {
        self.backend
    }

    /// 一意な Lambda 関数名を生成
    pub(crate) fn fresh_lambda_name(&mut self) -> String {
        let id = self.lambda_counter;
        self.lambda_counter += 1;
        format!("__lambda_{id}")
    }

    /// L# 型を選択した値表現の IR 型へ変換する。
    pub(crate) fn ir_type_for_type(&self, ty: &Type) -> IrType {
        if self.backend == LowerBackend::WasmGc {
            if matches!(ty, Type::Fun(_, _)) {
                return IrType::FuncRef;
            }
            if matches!(ty, Type::Con(name) if name == "String")
                && let Some(gc_idx) = self.string_array_type_index
            {
                return IrType::Ref(gc_idx);
            }
            if let Type::Record(name, _) = ty
                && let Some(&gc_idx) = self.record_type_indices.get(name)
            {
                return IrType::Ref(gc_idx);
            }
            if let Some(name) = type_to_name(ty)
                && let Some(&gc_idx) = self.adt_type_indices.get(&name)
            {
                return IrType::Ref(gc_idx);
            }
        }
        type_to_ir(ty)
    }

    /// 宣言中の型式を選択した値表現の IR 型へ変換する。
    pub(crate) fn type_expr_to_ir(&self, ty: &lsharp_syntax::ast::TypeExpr) -> IrType {
        if self.backend == LowerBackend::WasmGc
            && let lsharp_syntax::ast::TypeExpr::Named(_, name) = ty
        {
            if name == "String"
                && let Some(gc_idx) = self.string_array_type_index
            {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.record_type_indices.get(name) {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.adt_type_indices.get(name) {
                return IrType::Ref(gc_idx);
            }
        }
        type_expr_to_ir(ty)
    }

    fn wasm_gc_adt_field_type(&self, ty: &lsharp_syntax::ast::TypeExpr) -> Option<IrType> {
        match ty {
            lsharp_syntax::ast::TypeExpr::Named(_, name) => match name.as_str() {
                "Int" | "Float" | "Bool" | "Unit" => Some(type_expr_to_ir(ty)),
                _ => self
                    .record_type_indices
                    .get(name)
                    .or_else(|| self.adt_type_indices.get(name))
                    .copied()
                    .map(IrType::Ref),
            },
            lsharp_syntax::ast::TypeExpr::App(_, head, _) => {
                let lsharp_syntax::ast::TypeExpr::Named(_, name) = head.as_ref() else {
                    return None;
                };
                self.record_type_indices
                    .get(name)
                    .or_else(|| self.adt_type_indices.get(name))
                    .copied()
                    .map(IrType::Ref)
            }
            _ => None,
        }
    }

    pub(crate) fn ir_type_for_type_name(&self, type_name: &str) -> IrType {
        if self.backend == LowerBackend::WasmGc {
            if type_name == "String"
                && let Some(gc_idx) = self.string_array_type_index
            {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.record_type_indices.get(type_name) {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.adt_type_indices.get(type_name) {
                return IrType::Ref(gc_idx);
            }
        }
        IrType::I64
    }

}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}

/// 組み込み二項演算子か判定
pub(crate) fn is_builtin_binop(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "+."
            | "-."
            | "*."
            | "/."
            | "=="
            | "="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
    )
}
