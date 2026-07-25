//! Typed AST -> IR 変換 (Lowering)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;
use lsharp_types::infer::ExprTypeKey;
use lsharp_types::types::Type;

#[cfg(test)]
use crate::GcTypeKind;
use crate::{GcTypeDef, IrType, Module};

mod context;
mod decl;
mod expr;
mod heap_helpers;
mod pattern;
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

    pub(crate) fn lower_defn_functions(
        &mut self,
        program: &Program,
    ) -> Result<Vec<crate::Function>, LowerError> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::Defn {
                name, params, body, ..
            } = unwrap_private(decl)
            {
                let func = self.lower_function(name, params, body)?;
                functions.push(func);
            }
        }
        Ok(functions)
    }

    pub(crate) fn lower_field_accessors(&self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                for (field_idx, (fname, ftype)) in fields.iter().enumerate() {
                    let accessor =
                        self.generate_field_accessor(name, fname, field_idx as u32, ftype);
                    functions.push(accessor);
                }
            }
        }
        functions
    }

    pub(crate) fn lower_trait_impl_functions(
        &mut self,
        program: &Program,
    ) -> Result<Vec<crate::Function>, LowerError> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } = unwrap_private(decl)
            {
                for method_decl in methods {
                    if let Decl::Defn {
                        name: method_name,
                        params,
                        body,
                        ..
                    } = unwrap_private(method_decl)
                    {
                        let mangled = format!("{trait_name}_{type_name}_{method_name}");
                        let func = self.lower_function(&mangled, params, body)?;
                        functions.push(func);
                    }
                }
            }
        }
        Ok(functions)
    }

    pub(crate) fn lower_constraint_functions(&mut self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::TypeConstrained {
                name, constraints, ..
            } = unwrap_private(decl)
            {
                // Name.new: (-> BaseType BaseType) -- 制約チェック付き
                let check_func = self.generate_constraint_check(name, constraints);
                // 関数インデックスを登録
                let check_name = format!("{name}.new");
                if !self.func_indices.contains_key(&check_name) {
                    self.func_indices
                        .insert(check_name.clone(), self.late_func_idx);
                    self.late_func_idx += 1;
                }
                functions.push(check_func);

                // Name.valid?: (-> BaseType Bool) -- 検証のみ（トラップしない）
                let valid_func = self.generate_constraint_valid(name, constraints);
                let valid_name = format!("{name}.valid?");
                if !self.func_indices.contains_key(&valid_name) {
                    self.func_indices
                        .insert(valid_name.clone(), self.late_func_idx);
                    self.late_func_idx += 1;
                }
                functions.push(valid_func);
            }
        }
        functions
    }

    pub(crate) fn lower_adt_constructors(&self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::TypeDef { name, variants, .. } = unwrap_private(decl) {
                let gc_type_idx = self.adt_type_indices.get(name).copied().unwrap_or(0);
                let slot_types = self.adt_slot_types.get(name).cloned().unwrap_or_default();
                for variant in variants {
                    if let Some(&(_, tag_val)) = self.adt_variant_indices.get(&variant.name) {
                        let field_types = if self.backend == LowerBackend::WasmGc {
                            self.adt_variant_field_types
                                .get(&variant.name)
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            vec![IrType::I64; variant.fields.len()]
                        };
                        let constructor_slot_types = if self.backend == LowerBackend::WasmGc {
                            slot_types.clone()
                        } else {
                            vec![IrType::I64; variant.fields.len()]
                        };
                        let field_offsets = if self.backend == LowerBackend::WasmGc {
                            self.adt_variant_field_offsets
                                .get(&variant.name)
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        let ctor = self.generate_adt_constructor(
                            &variant.name,
                            gc_type_idx,
                            tag_val,
                            &field_types,
                            &constructor_slot_types,
                            &field_offsets,
                        );
                        functions.push(ctor);
                    }
                }
            }
        }
        functions
    }

    pub(crate) fn clone_string_data_from(&self, start: usize) -> Vec<(String, Vec<u8>)> {
        self.string_data[start..].to_vec()
    }

    pub(crate) fn gc_types_for_program(&self, program: &Program) -> Vec<GcTypeDef> {
        let mut gc_types = Vec::new();
        for decl in &program.decls {
            if let Decl::RecordDef { name, .. } = unwrap_private(decl)
                && let Some(&gc_idx) = self.record_type_indices.get(name)
            {
                gc_types.push(self.gc_types[gc_idx as usize].clone());
            }
        }
        for decl in &program.decls {
            if let Decl::TypeDef { name, .. } = unwrap_private(decl)
                && let Some(&gc_idx) = self.adt_type_indices.get(name)
            {
                gc_types.push(self.gc_types[gc_idx as usize].clone());
            }
        }
        if let Some(gc_idx) = self.string_array_type_index {
            gc_types.push(self.gc_types[gc_idx as usize].clone());
        }
        gc_types
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

    /// プログラム全体を IR に変換
    pub fn lower_program(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) -> Result<Module, LowerError> {
        let expr_type_results = HashMap::new();
        self.lower_program_with_expr_types(program, type_results, &expr_type_results)
    }

    pub fn lower_program_with_expr_types(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
        expr_type_results: &HashMap<ExprTypeKey, Type>,
    ) -> Result<Module, LowerError> {
        self.prepare_program_state(program, type_results);
        if self.backend == LowerBackend::WasmGc
            && let Some(message) = self.adt_field_errors.first()
        {
            return Err(LowerError::Unsupported {
                msg: message.clone(),
                span: None,
            });
        }
        self.expr_type_results = expr_type_results.clone();

        let mut functions = self.lower_defn_functions(program)?;
        functions.extend(self.lower_field_accessors(program));
        functions.extend(self.lower_trait_impl_functions(program)?);
        functions.extend(self.lower_constraint_functions(program));
        functions.extend(self.lower_adt_constructors(program));

        // Lambda Lifting: リフトされた関数を追加
        let lifted = self.lifted_functions.clone();
        functions.extend(lifted);

        let module = Module {
            functions,
            gc_types: self.gc_types.clone(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: self.string_data.clone(),
        };
        crate::root_lifetime::validate_module(&module)
            .map_err(|error| LowerError::RootLifetime { error })?;
        Ok(module)
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
