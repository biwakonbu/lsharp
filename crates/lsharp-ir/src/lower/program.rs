//! Program-level lowering orchestration

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::infer::ExprTypeKey;
use lsharp_types::types::Type;

use crate::{GcTypeDef, Module};

use super::{Lower, LowerBackend, LowerError, unwrap_private};

impl Lower {
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
                            vec![crate::IrType::I64; variant.fields.len()]
                        };
                        let constructor_slot_types = if self.backend == LowerBackend::WasmGc {
                            slot_types.clone()
                        } else {
                            vec![crate::IrType::I64; variant.fields.len()]
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
        let exemptions = root_lifetime_exemptions(program);
        crate::root_lifetime::validate_module(&module, &exemptions)
            .map_err(|error| LowerError::RootLifetime { error })?;
        Ok(module)
    }
}

/// `:roots-unbalanced "<理由>"` を宣言した `defn` の名前を集める。
///
/// 免除は IR には載せず、ここで AST から組み立てて `validate_module` へ渡す
/// (判断は `docs/adr/decisions-root-lifetime-intentional-imbalance-annotation.md`)。
/// 現状の対象は `defn` だけで、trait impl の method は含めない。
fn root_lifetime_exemptions(program: &Program) -> crate::root_lifetime::RootLifetimeExemptions {
    let names = program
        .decls
        .iter()
        .filter_map(|decl| match unwrap_private(decl) {
            Decl::Defn { name, metadata, .. } => metadata
                .as_ref()
                .and_then(|m| m.roots_unbalanced.as_ref())
                .map(|_| name.clone()),
            _ => None,
        });
    crate::root_lifetime::RootLifetimeExemptions::from_names(names)
}
