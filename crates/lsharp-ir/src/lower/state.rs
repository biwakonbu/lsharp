use lsharp_syntax::ast::{Decl, Program};

use crate::{GcField, GcTypeDef, GcTypeKind, IrType};

use super::{Lower, LowerBackend, unwrap_private};

impl Lower {
    fn reset_state(&mut self) {
        self.func_indices.clear();
        self.import_count = 0;
        self.type_results.clear();
        self.expr_type_results.clear();
        self.record_type_indices.clear();
        self.record_fields.clear();
        self.gc_types.clear();
        self.trait_method_impls.clear();
        self.trait_method_names.clear();
        self.adt_variant_indices.clear();
        self.adt_type_info.clear();
        self.adt_type_indices.clear();
        self.adt_variant_field_types.clear();
        self.adt_variant_field_offsets.clear();
        self.adt_variant_field_type_names.clear();
        self.adt_slot_types.clear();
        self.adt_field_errors.clear();
        self.string_data.clear();
        self.string_offset = 512;
        self.computation_builders.clear();
        self.string_array_type_index = None;
        self.lifted_functions.clear();
        self.lambda_counter = 0;
        self.lifted_func_indices.clear();
        self.next_func_idx = 0;
        self.late_func_idx = 0;
    }

    pub(crate) fn prepare_program_state(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) {
        self.reset_state();

        // 型推論結果を保存
        for (name, scheme) in type_results {
            self.type_results.insert(name.clone(), scheme.ty.clone());
        }

        if self.backend == LowerBackend::WasmGc {
            let gc_type_count = program
                .decls
                .iter()
                .filter(|decl| {
                    matches!(
                        unwrap_private(decl),
                        Decl::RecordDef { .. } | Decl::TypeDef { .. }
                    )
                })
                .count() as u32;
            self.string_array_type_index = Some(gc_type_count);
        }

        // レコード型定義を GC 型として登録
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                let gc_idx = self.gc_types.len() as u32;
                self.record_type_indices.insert(name.clone(), gc_idx);

                let gc_fields: Vec<GcField> = fields
                    .iter()
                    .map(|(fname, ftype)| GcField {
                        name: fname.clone(),
                        ty: self.type_expr_to_ir(ftype),
                        mutable: false,
                    })
                    .collect();

                // フィールド名リストを記録
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                self.record_fields.insert(name.clone(), field_names);

                self.gc_types.push(GcTypeDef {
                    name: name.clone(),
                    kind: GcTypeKind::Struct(gc_fields),
                });
            }
        }

        // WasmGC の ADT struct index を先に予約する。payload が別の ADT を参照していても、
        // 宣言順に依存せず concrete reference type を解決できるようにする。
        if self.backend == LowerBackend::WasmGc {
            for decl in &program.decls {
                if let Decl::TypeDef { name, .. } = unwrap_private(decl) {
                    let gc_idx = self.gc_types.len() as u32;
                    self.adt_type_indices.insert(name.clone(), gc_idx);
                    self.gc_types.push(GcTypeDef {
                        name: name.clone(),
                        kind: GcTypeKind::Struct(Vec::new()),
                    });
                }
            }
        }

        // ADT 型定義を WasmGC struct として登録する。
        // field 0 は variant tag、残りは variant 間で共有する typed payload slot とする。
        for decl in &program.decls {
            if let Decl::TypeDef { name, variants, .. } = unwrap_private(decl) {
                if self.backend != LowerBackend::WasmGc {
                    let variant_infos = variants
                        .iter()
                        .enumerate()
                        .map(|(tag, variant)| {
                            (variant.name.clone(), 0, tag as i32, variant.fields.len())
                        })
                        .collect();
                    for (tag, variant) in variants.iter().enumerate() {
                        self.adt_variant_indices
                            .insert(variant.name.clone(), (0, tag as i32));
                    }
                    self.adt_type_info.insert(name.clone(), variant_infos);
                    continue;
                }
                let gc_idx = self.adt_type_indices.get(name).copied().unwrap_or(0);
                let mut slot_types = Vec::new();
                for variant in variants {
                    let mut field_types = Vec::with_capacity(variant.fields.len());
                    let mut field_type_names = Vec::with_capacity(variant.fields.len());
                    for field in &variant.fields {
                        if let lsharp_syntax::ast::TypeExpr::App(_, head, _) = field
                            && let lsharp_syntax::ast::TypeExpr::Named(_, field_type_name) =
                                head.as_ref()
                            && field_type_name == name
                        {
                            self.adt_field_errors.push(format!(
                                "WasmGC ADT の自己参照 payload は現在未対応です: {name}::{}",
                                variant.name
                            ));
                        }
                        let Some(field_type) = self.wasm_gc_adt_field_type(field) else {
                            self.adt_field_errors.push(format!(
                                "WasmGC ADT payload の型を解決できません: {}::{}",
                                name, variant.name
                            ));
                            continue;
                        };
                        field_types.push(field_type);
                        field_type_names.push(match field {
                            lsharp_syntax::ast::TypeExpr::Named(_, name) => Some(name.clone()),
                            lsharp_syntax::ast::TypeExpr::App(_, head, _) => {
                                if let lsharp_syntax::ast::TypeExpr::Named(_, name) = head.as_ref()
                                {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        });
                    }
                    let field_offsets = field_types
                        .iter()
                        .map(|field_type| {
                            let offset = slot_types.len() as u32;
                            slot_types.push(*field_type);
                            offset
                        })
                        .collect::<Vec<_>>();
                    self.adt_variant_field_types
                        .insert(variant.name.clone(), field_types.clone());
                    self.adt_variant_field_offsets
                        .insert(variant.name.clone(), field_offsets);
                    self.adt_variant_field_type_names
                        .insert(variant.name.clone(), field_type_names);
                }

                self.adt_slot_types.insert(name.clone(), slot_types.clone());

                if self.backend == LowerBackend::WasmGc {
                    let mut gc_fields = Vec::with_capacity(slot_types.len() + 1);
                    gc_fields.push(GcField {
                        name: "tag".to_string(),
                        ty: IrType::I64,
                        mutable: false,
                    });
                    gc_fields.extend(slot_types.iter().enumerate().map(
                        |(field_idx, field_type)| GcField {
                            name: format!("field_{field_idx}"),
                            ty: *field_type,
                            mutable: false,
                        },
                    ));
                    self.gc_types[gc_idx as usize] = GcTypeDef {
                        name: name.clone(),
                        kind: GcTypeKind::Struct(gc_fields),
                    };
                }

                let mut variant_infos = Vec::new();
                for (tag, variant) in variants.iter().enumerate() {
                    let tag_val = tag as i32;
                    self.adt_variant_indices
                        .insert(variant.name.clone(), (gc_idx, tag_val));
                    variant_infos.push((
                        variant.name.clone(),
                        gc_idx,
                        tag_val,
                        variant.fields.len(),
                    ));
                }
                self.adt_type_info.insert(name.clone(), variant_infos);
            }
        }

        if let Some(type_index) = self.string_array_type_index {
            debug_assert_eq!(type_index as usize, self.gc_types.len());
            self.gc_types.push(GcTypeDef {
                name: "StringBytes".to_string(),
                kind: GcTypeKind::PackedByteArray,
            });
        }

        // import/内部ヘルパー関数を登録
        self.func_indices.insert("print".to_string(), 0);
        self.func_indices.insert("__alloc".to_string(), 1);
        self.func_indices.insert("__string_concat".to_string(), 2);
        self.func_indices.insert("__string_eq".to_string(), 3);
        self.func_indices.insert("print-string".to_string(), 4);
        self.func_indices.insert("proc-exit".to_string(), 5);
        self.func_indices.insert("__int_to_string".to_string(), 6);
        self.func_indices.insert("read-file".to_string(), 7);
        self.func_indices.insert("write-file".to_string(), 8);
        self.func_indices.insert("file-exists?".to_string(), 9);
        self.func_indices
            .insert("command-line-args".to_string(), 10);
        self.func_indices.insert("command-line-arg".to_string(), 11);
        self.func_indices.insert("read-stdin".to_string(), 12);
        self.func_indices.insert("__fnv1a_hash".to_string(), 13);
        self.func_indices.insert("root_push".to_string(), 14);
        self.func_indices.insert("root_pop".to_string(), 15);
        self.func_indices.insert("root_set".to_string(), 16);
        self.import_count = 17;

        // ユーザー定義関数のインデックスを事前登録
        let mut func_idx = self.import_count;
        for decl in &program.decls {
            if let Decl::Defn { name, .. } = unwrap_private(decl) {
                self.func_indices.insert(name.clone(), func_idx);
                func_idx += 1;
            }
        }

        // フィールドアクセサ関数のインデックスを登録
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                for (fname, _) in fields {
                    let accessor_name = format!("{name}.{fname}");
                    self.func_indices.insert(accessor_name, func_idx);
                    func_idx += 1;
                }
            }
        }

        // ADT コンストラクタ関数のインデックスを登録
        for decl in &program.decls {
            if let Decl::TypeDef { variants, .. } = unwrap_private(decl) {
                for variant in variants {
                    self.func_indices.insert(variant.name.clone(), func_idx);
                    func_idx += 1;
                }
            }
        }

        // トレイト定義からメソッド名の逆引きテーブルを構築（P5-6: 静的ディスパッチ）
        for decl in &program.decls {
            if let Decl::TraitDef {
                name: trait_name,
                methods,
                ..
            } = unwrap_private(decl)
            {
                for method in methods {
                    self.trait_method_names
                        .entry(method.name.clone())
                        .or_default()
                        .push(trait_name.clone());
                }
            }
        }

        // トレイト実装メソッドのインデックスを登録 (P5-6: 辞書パスイング)
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
                        name: method_name, ..
                    } = unwrap_private(method_decl)
                    {
                        // マングル名: TraitName_TypeName_methodName
                        let mangled = format!("{trait_name}_{type_name}_{method_name}");
                        self.func_indices.insert(mangled.clone(), func_idx);
                        self.trait_method_impls.insert(
                            (trait_name.clone(), type_name.clone(), method_name.clone()),
                            mangled,
                        );
                        func_idx += 1;
                    }
                }
            }
        }

        // Computation Builder の登録
        for decl in &program.decls {
            if let Decl::ComputationBuilder {
                name,
                bind_fn,
                return_fn,
                ..
            } = unwrap_private(decl)
            {
                self.computation_builders
                    .insert(name.clone(), (bind_fn.clone(), return_fn.clone()));
            }
        }

        // Lambda Lifting 用の次の関数インデックスを設定
        self.next_func_idx = func_idx;
        self.late_func_idx = func_idx;
    }
}
