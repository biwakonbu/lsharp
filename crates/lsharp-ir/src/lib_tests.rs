#[cfg(test)]
#[path = "lib_tests/linker.rs"]
mod linker_tests;

#[test]
fn test_link_modules_public_seam_preserves_import_rebase_contract() {
    let module = Module {
        functions: vec![Function {
            name: "write".to_string(),
            params: vec![],
            result: IrType::I32,
            locals: vec![],
            body: vec![Instruction::Call(0), Instruction::Return],
            is_export: false,
        }],
        gc_types: vec![],
        imports: vec![ImportFunc {
            module: "env".to_string(),
            name: "write".to_string(),
            params: vec![IrType::I32],
            result: IrType::I32,
        }],
        globals: vec![],
        string_data: vec![],
    };

    let linked = link_modules(&[module]);

    assert_eq!(linked.imports.len(), 1);
    assert_eq!(linked.imports[0].name, "write");
    assert_eq!(linked.functions.len(), 1);
    assert!(matches!(linked.functions[0].body[0], Instruction::Call(0)));
}

#[test]
fn test_module_dump_preserves_public_model_display_contract() {
    let module = Module {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: vec![],
            body: vec![Instruction::I64Const(7), Instruction::Return],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "Pair".to_string(),
            kind: GcTypeKind::Struct(vec![GcField {
                name: "value".to_string(),
                ty: IrType::I64,
                mutable: false,
            }]),
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    assert_eq!(
        module.dump(),
        "gc_type Pair = struct {value: i64}\n\nfn main(i64) -> i64:\n  i64.const 7\n  return\n\n"
    );
}

#[cfg(test)]
mod import_dedup_tests {
    use super::*;

    #[test]
    fn test_import_deduplication() {
        // 両方のモジュールが同じ import (wasi fd_write) を持つ場合、1つに統合される
        let mod_a = Module {
            functions: vec![Function {
                name: "write_a".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::Call(0)], // import index 0
                is_export: false,
            }],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
                params: vec![IrType::I32, IrType::I32, IrType::I32, IrType::I32],
                result: IrType::I32,
            }],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![Function {
                name: "write_b".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::Call(0)], // import index 0 (同じ fd_write)
                is_export: false,
            }],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "wasi_snapshot_preview1".to_string(),
                name: "fd_write".to_string(),
                params: vec![IrType::I32, IrType::I32, IrType::I32, IrType::I32],
                result: IrType::I32,
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        // import は1つに重複除去される
        assert_eq!(linked.imports.len(), 1);
        assert_eq!(linked.imports[0].name, "fd_write");
        // 両方の関数が同じ import index 0 を参照
        if let Instruction::Call(idx) = &linked.functions[0].body[0] {
            assert_eq!(*idx, 0);
        }
        if let Instruction::Call(idx) = &linked.functions[1].body[0] {
            assert_eq!(*idx, 0);
        }
    }

    #[test]
    fn test_different_imports_not_deduplicated() {
        let mod_a = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "print".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };
        let mod_b = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![ImportFunc {
                module: "env".to_string(),
                name: "read".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
            }],
            globals: vec![],
            string_data: vec![],
        };

        let linked = link_modules(&[mod_a, mod_b]);
        assert_eq!(linked.imports.len(), 2);
        assert_eq!(linked.imports[0].name, "print");
        assert_eq!(linked.imports[1].name, "read");
    }

    #[test]
    fn test_empty_imports() {
        let module = Module {
            functions: vec![],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };
        let linked = link_modules(&[module]);
        assert!(linked.imports.is_empty());
    }
}

#[cfg(test)]
#[path = "lib_tests/multifile_compile.rs"]
mod multifile_compile_tests;

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    #[test]
    fn test_source_fingerprint_identical_content() {
        let left = SourceFingerprint::from_source("(defn answer [] 42)\n");
        let right = SourceFingerprint::from_source("(defn answer [] 42)\n");

        assert_eq!(left, right, "同一ソースは同一 fingerprint になるべき");
    }

    #[test]
    fn test_source_fingerprint_one_char_change() {
        let left = SourceFingerprint::from_source("(defn answer [] 42)\n");
        let right = SourceFingerprint::from_source("(defn answer [] 43)\n");

        assert_ne!(
            left, right,
            "1 文字でも変更されたソースは別 fingerprint になるべき"
        );
    }

    #[test]
    fn test_source_fingerprint_empty_source() {
        let empty = SourceFingerprint::from_source("");
        let also_empty = SourceFingerprint::from_source("");
        let whitespace = SourceFingerprint::from_source(" ");

        assert_eq!(
            empty, also_empty,
            "空ソースは決定的に fingerprint できるべき"
        );
        assert_ne!(
            empty, whitespace,
            "空ソースと空白 1 文字は別 fingerprint になるべき"
        );
    }
}

#[cfg(test)]
#[path = "lib_tests/incremental_compile.rs"]
mod incremental_compile_tests;

#[cfg(test)]
#[path = "lib_tests/incremental_analysis.rs"]
mod incremental_analysis_tests;
#[cfg(test)]
mod memory_instruction_tests {
    use super::*;

    #[test]
    fn test_instruction_display_preserves_call_import_format() {
        assert_eq!(Instruction::CallImport(7).to_string(), "call_import 7");
    }

    #[test]
    fn test_memory_load_store_instructions() {
        let instructions = [
            Instruction::I32Const(100),
            Instruction::I32Load { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I32Const(42),
            Instruction::I32Store { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i64_memory_instructions() {
        let instructions = [
            Instruction::I32Const(100),
            Instruction::I64Load { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I64Const(12345),
            Instruction::I64Store { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_byte_memory_instructions() {
        let instructions = [
            Instruction::I32Const(100),
            Instruction::I32Load8U { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I32Const(65),
            Instruction::I32Store8 { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_arithmetic_instructions() {
        let instructions = [
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32Add,
            Instruction::I32Sub,
            Instruction::I32Mul,
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_comparison_instructions() {
        let instructions = [
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32GtU,
            Instruction::I32GeU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_i32_bitwise_instructions() {
        let instructions = [
            Instruction::I32Const(0xFF),
            Instruction::I32Const(4),
            Instruction::I32Shl,
            Instruction::I32ShrU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_memory_management_instructions() {
        let instructions = [
            Instruction::MemorySize,
            Instruction::I32Const(1),
            Instruction::MemoryGrow,
        ];
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_i64_extend_i32_unsigned() {
        let instructions = [Instruction::I32Const(42), Instruction::I64ExtendI32U];
        assert_eq!(instructions.len(), 2);
    }

    #[test]
    fn test_instruction_display_memory_ops() {
        assert_eq!(
            format!("{}", Instruction::I32Load { offset: 0 }),
            "i32.load offset=0"
        );
        assert_eq!(
            format!("{}", Instruction::I32Store { offset: 4 }),
            "i32.store offset=4"
        );
        assert_eq!(format!("{}", Instruction::MemoryGrow), "memory.grow");
        assert_eq!(format!("{}", Instruction::MemorySize), "memory.size");
        assert_eq!(format!("{}", Instruction::I32Add), "i32.add");
    }
}

#[cfg(test)]
mod selfhost_collision_tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    fn selfhost_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(rel)
    }

    fn parse_defn_names(rel: &str) -> HashSet<String> {
        let path = selfhost_path(rel);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{} を読めませんでした: {err}", path.display()));
        let program = lsharp_syntax::parse(&source)
            .unwrap_or_else(|err| panic!("{} を parse できませんでした: {err}", path.display()));
        program
            .decls
            .into_iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::Defn { name, .. } => Some(name),
                lsharp_syntax::ast::Decl::Private { inner, .. } => match *inner {
                    lsharp_syntax::ast::Decl::Defn { name, .. } => Some(name),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_selfhost_ir_control_flow_builders_do_not_reuse_ast_make_if_name() {
        let ast_names = parse_defn_names("selfhost/src/Syntax/AST.ls");
        let ir_names = parse_defn_names("selfhost/src/IR/IR.ls");

        assert!(
            ast_names.contains("make-if"),
            "Syntax.AST 側の make-if 前提が崩れている"
        );
        assert!(
            !ir_names.contains("make-if"),
            "IR.IR に make-if があると multi-file lowering で Syntax.AST.make-if と衝突する"
        );
    }
}
