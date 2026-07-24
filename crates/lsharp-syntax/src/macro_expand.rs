//! P10-2/P10-3: マクロ展開エンジン
//!
//! defmacro で定義されたマクロを展開する。
//! パイプライン: parse → **macro_expand** → type inference → lowering → codegen
//!
//! ## 展開ルール
//! - `(macro-name arg1 arg2)` → マクロ本体の Quote 式を展開
//! - Quote 内の `~param` → 対応する引数に置換
//! - Quote 内の `~@param` → リストを展開 (splice)
//! - 再帰展開は深度制限 128 まで
//!
//! ## P10-3: 型付きマクロ
//! - `:type` シグネチャをパース・保存 (検証は型推論フェーズで実施)
//! - 展開トレースバック: マクロ展開の履歴を保持し、エラー時に展開元を表示

use std::collections::HashMap;

use crate::ast::{Decl, Expr, Program, TypeExpr};
use crate::span::Span;

mod builtins;
mod error;
mod expand;

pub use error::{MacroExpandError, MacroExpansionStep};

/// マクロ定義
#[derive(Debug, Clone)]
struct MacroDef {
    params: Vec<String>,
    body: Expr,
    /// P10-3: オプションの型シグネチャ
    type_sig: Option<TypeExpr>,
}

/// マクロ展開器
pub struct MacroExpander {
    /// 登録済みマクロ
    macros: HashMap<String, MacroDef>,
    /// 再帰展開の深度制限
    max_depth: usize,
    /// gensym カウンター (簡易衛生性)
    gensym_counter: u64,
    /// P10-3: 展開トレースの記録
    expansion_trace: Vec<MacroExpansionStep>,
    /// P10-5: Computation Builder 登録情報 (ビルダー名 -> (bind関数名, return関数名))
    computation_builders: HashMap<String, (String, String)>,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            max_depth: 128,
            gensym_counter: 0,
            expansion_trace: Vec::new(),
            computation_builders: HashMap::new(),
        }
    }

    /// 組み込みマクロを登録済みの展開器を作成
    pub fn with_builtins() -> Self {
        let mut expander = Self::new();
        expander.register_builtins();
        expander
    }

    /// gensym: ユニークなシンボル名を生成 (簡易衛生性)
    pub fn gensym(&mut self, prefix: &str) -> String {
        self.gensym_counter += 1;
        format!("__gensym_{}_{}", prefix, self.gensym_counter)
    }

    /// P10-3: 展開トレースを取得
    pub fn expansion_trace(&self) -> &[MacroExpansionStep] {
        &self.expansion_trace
    }

    /// P10-3: マクロの型シグネチャを取得
    pub fn macro_type_sig(&self, name: &str) -> Option<&TypeExpr> {
        self.macros.get(name).and_then(|m| m.type_sig.as_ref())
    }

    /// プログラム全体のマクロ展開
    /// - DefMacro 宣言を収集してマクロテーブルに登録
    /// - 残りの宣言内の式を展開
    /// - DefMacro 宣言自体は出力から除去
    pub fn expand_program(&mut self, program: Program) -> Result<Program, MacroExpandError> {
        self.expansion_trace.clear();
        let mut expanded_decls = Vec::new();

        for decl in program.decls {
            match decl {
                Decl::ComputationBuilder {
                    name,
                    bind_fn,
                    return_fn,
                    ..
                } => {
                    // P10-5: Computation Builder を登録 (マクロ展開時に脱糖するため)
                    self.computation_builders
                        .insert(name.clone(), (bind_fn.clone(), return_fn.clone()));
                    // ComputationBuilder 宣言は出力に残す (型推論でも参照するため)
                    expanded_decls.push(Decl::ComputationBuilder {
                        span: Span::new(0, 0),
                        name,
                        bind_fn,
                        return_fn,
                    });
                }
                Decl::DefMacro {
                    name,
                    params,
                    macro_type,
                    body,
                    ..
                } => {
                    // マクロ定義を登録 (P10-3: 型シグネチャも保存)
                    let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                    self.macros.insert(
                        name,
                        MacroDef {
                            params: param_names,
                            body,
                            type_sig: macro_type,
                        },
                    );
                }
                other => {
                    let expanded = self.expand_decl(other).map_err(|e| {
                        // P10-3: エラーにトレースバックを付与
                        e.with_trace(self.expansion_trace.clone())
                    })?;
                    expanded_decls.push(expanded);
                }
            }
        }

        Ok(Program {
            decls: expanded_decls,
        })
    }

    /// 宣言内の式を展開
    fn expand_decl(&mut self, decl: Decl) -> Result<Decl, MacroExpandError> {
        match decl {
            Decl::Defn {
                span,
                name,
                params,
                return_ty,
                body,
                where_clauses,
                metadata,
            } => {
                let expanded_body = self.expand_expr(body, 0)?;
                Ok(Decl::Defn {
                    span,
                    name,
                    params,
                    return_ty,
                    body: expanded_body,
                    where_clauses,
                    metadata,
                })
            }
            // 他の宣言は再帰的にボディ内を展開する必要はない（現時点では）
            other => Ok(other),
        }
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_p10_5;

#[cfg(test)]
mod tests_computation_macro;
