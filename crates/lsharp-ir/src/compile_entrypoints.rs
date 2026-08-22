pub(super) fn compile_multi_file_with_mode(
    entry_file: &std::path::Path,
    lowering_mode: MultiFileLoweringMode,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    // 1. モジュールグラフの構築とファイル探索
    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    // 単一ファイルの場合は通常のパイプライン
    if sorted_files.len() == 1 {
        let (_, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let program = lsharp_syntax::parse(&source)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        // block 形式 module body は infer より前に弾く (I-39)
        crate::module_body_form::reject_block_form_module_body(&program)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut infer = lsharp_types::infer::Infer::new();
        let type_results = infer
            .infer_program(&program)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let expr_type_results = infer.expr_type_results_snapshot();
        let mut lower_ctx = lower::Lower::new();
        return lower_ctx
            .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    // 2. 全モジュールを依存順にパースし、SCC ごとに型チェックする。
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();

    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;

        let program = lsharp_syntax::parse(&source)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        // block 形式 module body は infer より前に弾く (I-39)
        crate::module_body_form::reject_block_form_module_body(&program)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        direct_imports.insert(mod_name.clone(), collect_import_visibility(&program));
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program);
    }

    for group in graph.scc_groups() {
        let surfaces = infer_scc_type_surfaces(
            &group,
            &graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in &sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        // 宣言を収集（module 宣言と import 宣言は除外）
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });
    }

    let lowered = match lowering_mode {
        MultiFileLoweringMode::Merged => {
            lower_multi_file_merged(&all_decls, &all_type_results, &all_expr_type_results)
        }
        MultiFileLoweringMode::Modular => lower_multi_file_modular(
            &module_programs,
            &all_decls,
            &all_type_results,
            &all_expr_type_results,
        ),
    };

    lowered.map_err(|e| format!("IR 変換エラー: {e}"))
}

pub fn compile_multi_file(entry_file: &std::path::Path) -> Result<Module, String> {
    compile_multi_file_with_mode(entry_file, MultiFileLoweringMode::Modular)
}

/// CLI compile で再利用できる解析/IR cache 付きの multi-file compile 入口。
///
/// 既存の `compile_multi_file_incremental` は互換 API として残し、公開 surface には
/// cache の意図が名前に現れるこちらを推奨する。
pub fn compile_multi_file_with_cache(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    cache.prepare_for_entry(entry_file);
    compile_multi_file_incremental(entry_file, cache)
}
