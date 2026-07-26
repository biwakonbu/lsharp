/// 循環を含む multi-file compile の incremental cache 更新を行う。
///
/// SCC は module 単位のトポロジカル推論では扱えないため、SCC ごとに一括推論してから
/// modular lowering を行う。型推論は SCC 単位で行うが、lowering は clean module の
/// segment を再利用し、dirty module の segment だけを生成し直す。
fn compile_multi_file_incremental_scc(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    let module_order = sorted_files
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect::<Vec<_>>();
    let mut current_fingerprints = HashMap::new();
    let mut all_clean = true;
    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        all_clean &= cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        current_fingerprints.insert(mod_name.clone(), fingerprint);
    }
    if all_clean
        && let Some(linked) = cache
            .linked_module()
            .filter(|linked| linked.module_order() == module_order)
    {
        return Ok(linked.final_module().clone());
    }

    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = current_fingerprints
            .get(mod_name)
            .copied()
            .unwrap_or_else(|| SourceFingerprint::from_source(&source));
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = current_fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    let mut all_decls = Vec::new();
    let mut all_type_results = Vec::new();
    let mut all_expr_type_results = HashMap::new();
    let mut module_programs = Vec::new();
    let mut cache_entries = Vec::new();
    let mut surface_changed_modules = HashSet::new();
    let mut segment_reuse_candidates = Vec::new();
    for (mod_name, _) in sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
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

        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);
        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;
        let surface_changed = cache
            .get(mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name.clone(), entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
}

/// source override を含む循環 module の incremental analysis を行う。
///
/// LSP の未保存 source でも compile と同じ SCC 推論境界を使い、解析結果だけを cache に保存する。
/// lowering はこの入口の責務ではないため、IR は空のまま保持する。
fn analyze_multi_file_incremental_scc_with_overrides(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in sorted_files {
        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache.insert_module(mod_name.clone(), entry);
    }

    Ok(())
}

fn read_source_with_overrides(
    path: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
) -> Result<String, String> {
    if let Some(source) = source_overrides.get(path) {
        return Ok(source.clone());
    }

    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn analyze_single_file_incremental(
    module_name: &str,
    source: &str,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let fingerprint = SourceFingerprint::from_source(source);
    let clean_hit = cache
        .get(module_name)
        .is_some_and(|entry| entry.fingerprint() == fingerprint);
    if clean_hit {
        return Ok(());
    }

    let program = cached_program_or_parse(module_name, source, fingerprint, cache)
        .map_err(|e| format!("[{}] {e}", e.code()))?;
    let mut infer = lsharp_types::infer::Infer::new();
    note_incremental_type_infer();
    let type_results = infer
        .infer_program(program.as_ref())
        .map_err(|e| format!("[{}] {e}", e.code()))?;
    let type_surface = ModuleTypeSurface {
        results: type_results,
        hidden: infer.module_env.privates.iter().cloned().collect(),
        expr_types: infer.expr_type_results_snapshot(),
    };
    let entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
    cache.insert_module(module_name.to_string(), entry);
    Ok(())
}

pub fn analyze_multi_file_incremental_with_overrides(
    entry_file: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    use module_graph::ModuleGraph;

    cache.prepare_for_entry(entry_file);
    let (graph, sorted_files) =
        ModuleGraph::build_from_entry_with_overrides_scc(entry_file, source_overrides)
            .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        return analyze_single_file_incremental(mod_name, &source, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return analyze_multi_file_incremental_scc_with_overrides(
            &graph,
            &sorted_files,
            source_overrides,
            cache,
        );
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }

    if changed_modules.is_empty() {
        return Ok(());
    }

    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name, type_surface);
    }

    for (mod_name, entry) in cache_entries {
        cache.insert_module(mod_name, entry);
    }

    Ok(())
}

pub fn compile_multi_file_incremental(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if clean_hit {
            return Ok(cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .ir()
                .clone());
        }
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let type_surface = if clean_hit {
            cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let mut lower_ctx = lower::Lower::new();
        note_incremental_lower();
        let module = lower_ctx
            .lower_program_with_expr_types(
                program.as_ref(),
                &type_surface.results,
                &type_surface.expr_types,
            )
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
        entry.set_ir(module.clone());
        cache.insert_module(mod_name.clone(), entry);
        return Ok(module);
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return compile_multi_file_incremental_scc(&graph, &sorted_files, cache);
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }
    if changed_modules.is_empty() {
        let first_clean_entry = module_inputs
            .first()
            .and_then(|(mod_name, _, _, _)| cache.get(mod_name).map(|entry| entry.ir().clone()))
            .expect("all clean hits should have cache entries");
        return Ok(first_clean_entry);
    }
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut segment_reuse_candidates: Vec<bool> = Vec::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        all_type_results.extend(type_surface.results.clone());
        all_expr_type_results.extend(type_surface.expr_types.clone());
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

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name.clone(), type_surface);
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let module_order: Vec<String> = cache_entries
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect();
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name, entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
}
