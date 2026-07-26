#[cfg(test)]
#[path = "lib_tests/linker.rs"]
mod linker_tests;
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
mod multifile_compile_tests {
    use super::*;

    #[test]
    fn test_merged_scc_declarations_deduplicate_identical_imports() {
        let module_a =
            lsharp_syntax::parse("(module A) (import Shared) (import Shared) (defn a [] 1)")
                .expect("module A should parse");
        let module_b = lsharp_syntax::parse("(module B) (import Shared) (defn b [] 2)")
            .expect("module B should parse");
        let parsed_modules =
            HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
        let group = vec!["A".to_string(), "B".to_string()];

        let (merged_decls, defn_origins) =
            merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
        let imports = merged_decls
            .iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::ImportDecl { module, .. } => Some(module.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(imports, vec!["Shared"]);
        assert_eq!(defn_origins, vec!["A", "B"]);
    }

    #[test]
    fn test_merged_scc_declarations_keep_distinct_import_visibility() {
        let module_a = lsharp_syntax::parse("(module A) (import Shared :only [x]) (defn a [] 1)")
            .expect("module A should parse");
        let module_b = lsharp_syntax::parse("(module B) (import Shared :only [y]) (defn b [] 2)")
            .expect("module B should parse");
        let parsed_modules =
            HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
        let group = vec!["A".to_string(), "B".to_string()];

        let (merged_decls, _) =
            merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
        let imports = merged_decls
            .iter()
            .filter_map(|decl| match decl {
                lsharp_syntax::ast::Decl::ImportDecl { only, .. } => only.clone(),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(imports, vec![vec!["x".to_string()], vec!["y".to_string()]]);
    }

    fn main_function(module: &Module) -> &Function {
        module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function should exist")
    }

    fn call_positions(body: &[Instruction], target: u32) -> Vec<usize> {
        body.iter()
            .enumerate()
            .filter_map(|(idx, instr)| match instr {
                Instruction::Call(actual) if *actual == target => Some(idx),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_compile_multi_file_injects_only_dependency_closure() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_dependency_closure");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("A.ls"), "(module A)\n(defn status [x] x)\n").unwrap();
        std::fs::write(
            dir.join("Noise.ls"),
            "(module Noise)\n(defn status [x] true)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ZConsumer.ls"),
            "(module ZConsumer)\n(import A)\n(defn check [x] (= (status x) 1))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(import Noise)\n(import ZConsumer)\n(defn main [] (if (check 1) 1 0))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "unrelated sibling module types should not pollute dependency inference: {result:?}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_import_only_blocks_non_selected_symbol() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_import_only_blocks");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(defn helper [] 1)\n(defn secret [] 2)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils :only [helper])\n(defn main [] (secret))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            ":only で除外されたシンボルは compile でも参照できないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_infers_mutual_recursive_scc() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_mutual_recursive_scc_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "相互再帰 SCC はモジュール単位の循環エラーではなく一括推論へ進めるべき: {result:?}"
        );
        let module = result.unwrap();
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "a-step")
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "b-step")
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "main")
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_unrestricted_scc_uses_merged_surface_fast_path() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_unrestricted_scc_fast_path_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let tracker = IncrementalSccMergedFastPathTracker::new();
        tracker.reset();
        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_ok(),
            "公開 import の SCC は compile できるべき: {result:?}"
        );
        assert_eq!(
            tracker.count(),
            1,
            "可視性制約のない A↔B SCC は merged inference の surface を再検証なしで利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_infers_mutual_recursive_scc() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_mutual_recursive_scc_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        let type_tracker = IncrementalTypeInferTracker::new();
        type_tracker.reset();
        let first = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);
        assert!(
            first.is_ok(),
            "incremental compile も相互再帰 SCC を受理するべき: {first:?}"
        );
        assert_eq!(
            type_tracker.count(),
            1,
            "singleton SCC は module-local inference を 1 回だけ実行するべき"
        );
        drop(type_tracker);
        let first = first.unwrap();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        let second = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(
            tracker.count(),
            0,
            "SCC の clean rebuild は module inference を再実行しないべき"
        );
        assert_eq!(
            first.dump(),
            second.dump(),
            "SCC の clean rebuild は同じ linked IR を返すべき"
        );

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            tracker.count() > 0,
            "SCC の dirty rebuild は型推論を再実行するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_scc_reuses_clean_ir_segments_after_dirty_module() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_scc_segments_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            !cache
                .get("Base")
                .expect("Base module should be cached")
                .ir_segments()
                .is_empty(),
            "SCC compile 後も独立した module の IR segment を cache するべき"
        );

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let link_tracker = IncrementalLinkTracker::new();
        link_tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "SCC 内の A だけが dirty なら clean module の segment を再利用し、fresh lower は A のみにするべき"
        );
        assert_eq!(
            link_tracker.cache_hit_count(),
            1,
            "SCC の segment 長が不変なら cached final module を range patch するべき"
        );
        assert_eq!(
            link_tracker.full_count(),
            0,
            "SCC の segment 長が不変なら full relink を再実行しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "SCC の dirty segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "SCC の dirty segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_scc_reuses_clean_type_surfaces_after_impl_change() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_incremental_scc_type_cache_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
        )
        .unwrap();

        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "A の実装だけが dirty な場合は Base/Main の clean SCC を再推論せず、A↔B SCC だけを再推論するべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "SCC type surface reuse 後も final linked IR は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_scc_preserves_import_only_visibility() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_scc_import_only_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B :only [b-step])\n(defn a-step [] (secret))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [] (a-step))\n(defn secret [] 2)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            "SCC 内でも import :only の境界を越えてはならない"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("secret"),
            "診断に拒否された symbol を含めるべき: {error}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_private_import_blocks_symbol() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_private_blocks");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(private (defn secret [] 2))\n(defn helper [] 1)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils)\n(defn main [] (secret))\n",
        )
        .unwrap();

        let result = compile_multi_file(&dir.join("Main.ls"));
        assert!(
            result.is_err(),
            "private なシンボルは compile でも参照できないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_modular_lowering_matches_merged_reference_with_strings() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_modular_matches_merged");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn helper [] \"lib\")\n(defn helper2 [] \"++\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Suffix.ls"),
            "(module Suffix)\n(defn bang [] \"!\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(import Suffix)\n(defn main [] (string-concat (string-concat (helper) (helper2)) (bang)))\n",
        )
        .unwrap();

        let merged =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged)
                .unwrap();
        let modular =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular)
                .unwrap();

        assert_eq!(
            merged.dump(),
            modular.dump(),
            "module-local lowering は merged lowering と同じ関数順序・命令列を維持するべき"
        );
        assert_eq!(
            merged.string_data, modular.string_data,
            "module-local lowering は merged lowering と同じ string_data 配列を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_closure_call_roots_local_generic_result_argument() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_closure_generic_result_rooting");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn make-show [] (fn [s] (string-length s)))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (let [id (fn [x] x) f (make-show)] (f (id \"hello\"))))\n",
        )
        .unwrap();

        let merged =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged)
                .unwrap();
        let modular =
            compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular)
                .unwrap();

        assert_eq!(
            call_positions(&main_function(&merged).body, 14).len(),
            4,
            "multi-file merged lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
            main_function(&merged).body
        );
        assert_eq!(
            call_positions(&main_function(&modular).body, 14).len(),
            4,
            "multi-file modular lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
            main_function(&modular).body
        );
        assert_eq!(
            merged.dump(),
            modular.dump(),
            "expr-type table を通した modular lowering も merged lowering と同一 IR を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

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
mod incremental_compile_tests {
    use super::*;

    fn main_function(module: &Module) -> &Function {
        module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function should exist")
    }

    fn call_positions(body: &[Instruction], target: u32) -> Vec<usize> {
        body.iter()
            .enumerate()
            .filter_map(|(idx, instr)| match instr {
                Instruction::Call(actual) if *actual == target => Some(idx),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_compile_multi_file_incremental_empty_cache_matches_full_compile() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_empty_cache");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let main_entry = cache.get("Main").expect("Main module should be cached");

        assert_eq!(
            full.dump(),
            incremental.dump(),
            "空キャッシュ初回コンパイルは既存のフルコンパイルと同一結果になるべき"
        );
        assert_eq!(
            cache.len(),
            2,
            "初回 incremental compile は通過したモジュールを cache に記録するべき"
        );
        assert!(
            main_entry.type_result_len() > 0,
            "cache entry は型サーフェスも保持するべき"
        );
        assert_eq!(
            main_entry.fingerprint(),
            SourceFingerprint::from_source(main_source),
            "cache entry は読み込んだソースの fingerprint を保持するべき"
        );
        assert_eq!(
            main_entry.imports(),
            ["Lib"],
            "cache entry は direct import module 名を保持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_matches_fresh_and_warm_compile() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_api_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let fresh = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let tracker = IncrementalTypeInferTracker::new();
        let cold = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(fresh.dump(), cold.dump());
        assert_eq!(cache.len(), 2);

        tracker.reset();
        let warm = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(cold.dump(), warm.dump());
        assert_eq!(
            tracker.count(),
            0,
            "warm cache compile は再型推論しないべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_isolated_by_entry_root() {
        let base = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_scope_{}",
            std::process::id()
        ));
        if base.exists() {
            std::fs::remove_dir_all(&base).unwrap();
        }
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();

        std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            first.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();
        std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_with_cache(&first.join("Main.ls"), &mut cache).unwrap();
        assert_eq!(
            cache.len(),
            2,
            "first project は Main と Lib を cache するべき"
        );

        let second_module = compile_multi_file_with_cache(&second.join("Main.ls"), &mut cache)
            .expect("entry root が変わっても compile できるべき");
        assert!(matches!(
            main_function(&second_module).body.as_slice(),
            [Instruction::I64Const(42)]
        ));
        assert_eq!(
            cache.len(),
            1,
            "別 project の stale module は cache に残さないべき"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_compile_multi_file_with_cache_tracks_dependency_surface_key() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_compile_multi_file_with_cache_dependency_key_{}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        let lib_path = dir.join("Lib.ls");
        let main_path = dir.join("Main.ls");
        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            &main_path,
            "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let initial_key = cache.get("Main").unwrap().deps_key();

        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 8)\n").unwrap();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let implementation_only_key = cache.get("Main").unwrap().deps_key();
        assert_eq!(
            initial_key, implementation_only_key,
            "依存 module の実装だけが変わった場合、公開型 key は維持するべき"
        );

        std::fs::write(&lib_path, "(module Lib)\n(defn helper [] true)\n").unwrap();
        compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
        let surface_changed_key = cache.get("Main").unwrap().deps_key();
        assert_ne!(
            implementation_only_key, surface_changed_key,
            "依存 module の公開型が変わった場合、依存 key も変わるべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_roots_local_generic_closure_result_argument() {
        let dir = std::env::temp_dir()
            .join("lsharp_compile_multi_file_incremental_closure_generic_result_rooting");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Lib.ls"),
            "(module Lib)\n(defn make-show [] (fn [s] (string-length s)))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (let [id (fn [x] x) f (make-show)] (f (id \"hello\"))))\n",
        )
        .unwrap();

        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
        let mut cache = CompilationCache::new();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            call_positions(&main_function(&full).body, 14).len(),
            4,
            "full multi-file compile は local generic closure result を使う closure call で outer arg 用まで root_push するべき: {:?}",
            main_function(&full).body
        );
        assert_eq!(
            call_positions(&main_function(&incremental).body, 14).len(),
            4,
            "incremental multi-file compile も expr-type cache を通して outer arg 用まで root_push するべき: {:?}",
            main_function(&incremental).body
        );
        assert_eq!(
            full.dump(),
            incremental.dump(),
            "incremental multi-file compile も expr-type table を含めて full compile と同一 IR を維持するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_parse_on_cache_hit() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let lib_source = "(module Lib)\n(defn helper [] 7)\n";
        let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let mut cache = CompilationCache::new();
        let tracker = IncrementalParseTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        tracker.reset();
        cached_program_or_parse(
            "Lib",
            lib_source,
            SourceFingerprint::from_source(lib_source),
            &cache,
        )
        .unwrap();
        cached_program_or_parse(
            "Main",
            main_source,
            SourceFingerprint::from_source(main_source),
            &cache,
        )
        .unwrap();
        assert_eq!(
            tracker.count(),
            0,
            "事前確認として cache helper 単体では両モジュールとも hit するべき"
        );

        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "fingerprint が不変な再コンパイルでは AST cache hit により parse をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reparses_only_changed_module() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_single_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 2))\n",
        )
        .unwrap();

        let tracker = IncrementalParseTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "1 モジュールだけ fingerprint が変わった場合はその AST だけ再パースするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_cached_ast_arc_on_cache_hit() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ast_arc_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        let main_source = "(module Main)\n(defn main [] 1)\n";
        std::fs::write(dir.join("Main.ls"), main_source).unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let cached = cache
            .get("Main")
            .expect("Main module should be cached")
            .ast_arc();
        let reused = cached_program_or_parse(
            "Main",
            main_source,
            SourceFingerprint::from_source(main_source),
            &cache,
        )
        .unwrap();

        assert!(
            std::sync::Arc::ptr_eq(&cached, &reused),
            "AST cache hit では同じ Arc<Program> を再利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_type_inference_on_clean_cache_hit() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "dirty set が空なら cached ModuleTypeSurface を再利用して型推論をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_ir_generation_on_clean_cache_hit() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ir_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        let tracker = IncrementalLowerTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            0,
            "dirty set が空なら cached IR を再利用して lowering をスキップするべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_analyze_single_file_incremental_skips_parse_and_infer_on_clean_cache_hit() {
        let mut cache = CompilationCache::new();
        let source = "(module Main)\n(defn main [] 1)\n";

        analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

        let parse_tracker = IncrementalParseTracker::new();
        let infer_tracker = IncrementalTypeInferTracker::new();
        parse_tracker.reset();
        infer_tracker.reset();

        analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

        assert_eq!(
            parse_tracker.count(),
            0,
            "single-file incremental analysis は clean hit で parse を再実行しないべき"
        );
        assert_eq!(
            infer_tracker.count(),
            0,
            "single-file incremental analysis は clean hit で type infer を再実行しないべき"
        );
    }

    #[test]
    fn test_analyze_single_file_incremental_reparses_and_reinfers_on_source_change() {
        let mut cache = CompilationCache::new();
        analyze_single_file_incremental(
            "lsp://Main",
            "(module Main)\n(defn main [] 1)\n",
            &mut cache,
        )
        .unwrap();

        let parse_tracker = IncrementalParseTracker::new();
        let infer_tracker = IncrementalTypeInferTracker::new();
        parse_tracker.reset();
        infer_tracker.reset();

        analyze_single_file_incremental(
            "lsp://Main",
            "(module Main)\n(defn main [] 2)\n",
            &mut cache,
        )
        .unwrap();

        assert_eq!(
            parse_tracker.count(),
            1,
            "single-file incremental analysis は fingerprint が変わった source を再パースするべき"
        );
        assert_eq!(
            infer_tracker.count(),
            1,
            "single-file incremental analysis は fingerprint が変わった source を再推論するべき"
        );
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_reports_unsaved_missing_import() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir()
            .join("lsharp_analyze_multi_file_incremental_overlay_missing_import");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] 1)\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("Main.ls"),
            "(module Main)\n(import Missing)\n(defn main [] 1)\n".to_string(),
        );
        let mut cache = CompilationCache::new();

        let result = analyze_multi_file_incremental_with_overrides(
            &dir.join("Main.ls"),
            &overrides,
            &mut cache,
        );

        let _ = std::fs::remove_dir_all(&dir);

        let error = result.expect_err("unsaved import override は missing module error を返すべき");
        assert!(
            error.contains("Missing"),
            "error は unsaved source の import 先 Missing を含むべき: {error}"
        );
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_isolated_by_entry_root() {
        use std::collections::HashMap;

        let base = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_scope_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            first.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
        )
        .unwrap();
        std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

        let overrides = HashMap::new();
        let mut cache = CompilationCache::new();
        analyze_multi_file_incremental_with_overrides(
            &first.join("Main.ls"),
            &overrides,
            &mut cache,
        )
        .unwrap();
        assert_eq!(
            cache.len(),
            2,
            "first workspace は Main と Lib を cache するべき"
        );

        analyze_multi_file_incremental_with_overrides(
            &second.join("Main.ls"),
            &overrides,
            &mut cache,
        )
        .unwrap();
        assert_eq!(
            cache.len(),
            1,
            "別 workspace の override analysis は stale module を残さないべき"
        );

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_infers_mutual_recursive_scc() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_scc_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n"
                .to_string(),
        );
        let mut cache = CompilationCache::new();
        let result = analyze_multi_file_incremental_with_overrides(
            &dir.join("Main.ls"),
            &overrides,
            &mut cache,
        );

        assert!(
            result.is_ok(),
            "source override analysis も相互再帰 SCC を受理するべき: {result:?}"
        );
        assert_eq!(cache.len(), 3, "SCC 内外の 3 module を cache するべき");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_analyze_multi_file_incremental_with_overrides_reuses_clean_scc_type_surfaces() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_analyze_multi_file_incremental_overlay_type_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("B.ls"),
            "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
        )
        .unwrap();

        let mut overrides = HashMap::new();
        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n"
                .to_string(),
        );
        let mut cache = CompilationCache::new();
        let tracker = IncrementalSccInferTracker::new();
        tracker.reset();
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
            .unwrap();
        assert_eq!(
            tracker.count(),
            3,
            "override の初回分析は Base / A↔B / Main の3 SCCを型推論するべき"
        );

        overrides.insert(
            dir.join("A.ls"),
            "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n"
                .to_string(),
        );
        tracker.reset();
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
            .unwrap();
        assert_eq!(
            tracker.count(),
            1,
            "override で A の実装だけが dirty な場合は A↔B SCC だけを再推論するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_prefix_module_ir_segments_before_first_dirty_module()
     {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_module_ir_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        assert!(
            !cache
                .get("Base")
                .expect("Base module should be cached")
                .ir_segments()
                .is_empty(),
            "warm cache 後は prefix module の IR segment が保存されるべき"
        );

        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 2))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "tail module だけ dirty な場合は clean prefix module の IR segment を再利用し、fresh lower は dirty suffix のみで済むべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "prefix IR segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "prefix IR segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_clean_suffix_module_when_dirty_middle_layout_is_stable()
     {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_ir_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "dirty middle module が layout 不変なら clean suffix module の IR segment も再利用し、fresh defn lower は dirty module のみで済むべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "clean suffix IR segment reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "clean suffix IR segment reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reuses_clean_suffix_when_dirty_middle_only_changes_string_state()
     {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_string_state");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"a\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"alphabet\")\n",
        )
        .unwrap();

        let tracker = IncrementalModuleSegmentLowerTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "dirty middle module の defn string state だけ変わる場合は clean suffix module の defn を再 lower しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "suffix defn reuse 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "suffix defn reuse 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_patches_cached_final_link_when_segment_lengths_match() {
        let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_hit");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Base.ls"),
            "(module Base)\n(defn base-val [] 10)\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
        )
        .unwrap();

        let tracker = IncrementalLinkTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.cache_hit_count(),
            1,
            "module order と segment 長が不変なら cached final module を range patch して full relink を避けるべき"
        );
        assert_eq!(
            tracker.full_count(),
            0,
            "range patch が成立する変更では full relink を再実行しないべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "link cache hit 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "link cache hit 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_invalidates_link_cache_when_segment_lengths_change() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_miss");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(defn mid-val [] \"a\")\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Mid)\n(defn main [] (mid-val))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        std::fs::write(
            dir.join("Mid.ls"),
            "(module Mid)\n(defn mid-val [] (string-concat \"a\" \"b\"))\n",
        )
        .unwrap();

        let tracker = IncrementalLinkTracker::new();
        tracker.reset();
        let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

        assert_eq!(
            tracker.cache_hit_count(),
            0,
            "string_data segment 長が変わる変更では cached final module patch は使わないべき"
        );
        assert_eq!(
            tracker.full_count(),
            1,
            "segment 長が変わる変更では full relink にフォールバックするべき"
        );
        assert_eq!(
            incremental.dump(),
            full.dump(),
            "link cache miss 後も final linked IR は full compile と一致するべき"
        );
        assert_eq!(
            incremental.string_data, full.string_data,
            "link cache miss 後も string_data は full compile と一致するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_skips_dependent_reinfer_when_surface_unchanged() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_impl_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 8)\n").unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

        assert_eq!(
            tracker.count(),
            1,
            "依存先の実装変更で型サーフェスが不変なら dependency のみ再型推論し、dependent は再利用するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_reinfers_on_dependency_signature_change() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_sig_change");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();

        let mut cache = CompilationCache::new();
        compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
        std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] true)\n").unwrap();

        let tracker = IncrementalTypeInferTracker::new();
        tracker.reset();
        let result = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);

        assert!(
            result.is_err(),
            "依存先シグネチャ変更で不整合になれば compile は失敗するべき"
        );
        assert_eq!(
            tracker.count(),
            2,
            "依存先シグネチャ変更では dependency + dependent を再型推論するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds() {
        let cli_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../selfhost/src/App/Cli.ls");
        let mut cache = CompilationCache::new();

        compile_multi_file_incremental(&cli_path, &mut cache)
            .expect("first incremental compile of selfhost Cli.ls should succeed");
        let second = compile_multi_file_incremental(&cli_path, &mut cache);

        assert!(
            second.is_ok(),
            "clean rebuild with formatter trio cache should not fail: {second:?}"
        );
    }

    #[test]
    fn test_formatter_modules_declare_cross_module_dispatch_imports() {
        let source_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../selfhost/src/Tools/Text");
        let expr = std::fs::read_to_string(source_root.join("FormatterExpr.ls")).unwrap();
        let decl = std::fs::read_to_string(source_root.join("FormatterDecl.ls")).unwrap();

        assert!(
            expr.lines()
                .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
            "FormatterExpr は dispatch 関数の提供元を明示 import するべき"
        );
        assert!(
            decl.lines()
                .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
            "FormatterDecl は dispatch 関数の提供元を明示 import するべき"
        );
    }
}

#[cfg(test)]
mod memory_instruction_tests {
    use super::*;

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
