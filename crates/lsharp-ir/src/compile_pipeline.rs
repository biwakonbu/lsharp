fn lower_multi_file_modular_with_segments(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    reusable_segments: &[Option<ModuleIrSegments>],
    segment_reuse_candidates: &[bool],
) -> Result<ModularLoweringResult, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.prepare_program_state(&merged_program, all_type_results);
    lower_ctx.expr_type_results = all_expr_type_results.clone();

    let mut segments = vec![ModuleIrSegments::empty(); module_programs.len()];
    let cached_precomputed_shapes: Vec<Option<ModulePrecomputedShape>> = reusable_segments
        .iter()
        .map(|segments| {
            segments
                .as_ref()
                .map(module_precomputed_shape_from_segments)
        })
        .collect();
    let cached_defn_shapes: Vec<Option<ModuleDefnStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_defn_state_shape))
        .collect();
    let cached_trait_shapes: Vec<Option<ModuleTraitImplStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_trait_impl_state_shape))
        .collect();
    let precomputed_shape_matches: Vec<bool> = module_programs
        .iter()
        .zip(cached_precomputed_shapes.iter())
        .map(|(program, cached)| {
            cached
                .map(|cached_shape| module_precomputed_shape_from_program(program) == cached_shape)
                .unwrap_or(false)
        })
        .collect();
    let mut defn_shape_matches = vec![false; module_programs.len()];
    let mut fresh_defn_lower_count = 0usize;
    let mut precomputed_prefix_stable = true;
    let mut defn_prefix_stable = true;

    for (idx, program) in module_programs.iter().enumerate() {
        let current_defn_needs_prefix_state =
            cached_defn_shapes[idx].is_some_and(defn_state_depends_on_prefix);
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && (!current_defn_needs_prefix_state || defn_prefix_stable)
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.defns().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.defn_lifted());
            segments[idx].set_defns(cached.defns().clone());
            segments[idx].set_defn_lifted(cached.defn_lifted().clone());
            defn_shape_matches[idx] = true;
        } else {
            fresh_defn_lower_count += 1;
            let gc_types = lower_ctx.gc_types_for_program(program);
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_defn_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_defns(build_segment_module(functions, gc_types, string_data));
            segments[idx].set_defn_lifted(build_segment_module(lifted, Vec::new(), Vec::new()));
            defn_shape_matches[idx] = cached_defn_shapes[idx].is_some_and(|cached_shape| {
                module_defn_state_shape(&segments[idx]) == cached_shape
            });
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        defn_prefix_stable &= defn_shape_matches[idx];
    }

    let defn_global_stable = defn_shape_matches.iter().all(|stable| *stable);

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_accessors(cached.accessors().clone());
        } else {
            segments[idx].set_accessors(build_segment_module(
                lower_ctx.lower_field_accessors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    precomputed_prefix_stable = true;
    let mut trait_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && defn_global_stable
            && precomputed_prefix_stable
            && trait_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.trait_impls().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.trait_impl_lifted());
            segments[idx].set_trait_impls(cached.trait_impls().clone());
            segments[idx].set_trait_impl_lifted(cached.trait_impl_lifted().clone());
        } else {
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_trait_impl_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_trait_impls(build_segment_module(functions, Vec::new(), string_data));
            segments[idx].set_trait_impl_lifted(build_segment_module(
                lifted,
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        trait_prefix_stable &= cached_trait_shapes[idx].is_some_and(|cached_shape| {
            module_trait_impl_state_shape(&segments[idx]) == cached_shape
        });
    }

    precomputed_prefix_stable = true;
    let mut constraint_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && constraint_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            lower_ctx.late_func_idx += cached.constraints().functions.len() as u32;
            segments[idx].set_constraints(cached.constraints().clone());
        } else {
            segments[idx].set_constraints(build_segment_module(
                lower_ctx.lower_constraint_functions(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        constraint_prefix_stable &= cached_precomputed_shapes[idx].is_some_and(|cached_shape| {
            module_precomputed_shape_from_segments(&segments[idx]).constraint_count
                == cached_shape.constraint_count
        });
    }

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_ctors(cached.ctors().clone());
        } else {
            segments[idx].set_ctors(build_segment_module(
                lower_ctx.lower_adt_constructors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    Ok(ModularLoweringResult {
        segments,
        fresh_defn_lower_count,
    })
}

fn lower_multi_file_modular(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let reusable_segments = vec![None; module_programs.len()];
    let lowering = lower_multi_file_modular_with_segments(
        module_programs,
        all_decls,
        all_type_results,
        all_expr_type_results,
        &reusable_segments,
        &vec![false; module_programs.len()],
    )?;
    Ok(link_module_ir_segments(&lowering.segments))
}

fn collect_private_surface_names(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    out: &mut HashSet<String>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        match decl {
            Decl::Private { inner, .. } => match inner.as_ref() {
                Decl::Defn { name, .. }
                | Decl::TypeDef { name, .. }
                | Decl::RecordDef { name, .. }
                | Decl::TypeAlias { name, .. }
                | Decl::TypeConstrained { name, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    out.insert(qualified);
                }
                Decl::ModuleDecl { name, body, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    collect_private_surface_names(body, Some(&qualified), out);
                }
                _ => {}
            },
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let qualified = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_private_surface_names(body, Some(&qualified), out);
            }
            _ => {}
        }
    }
}

fn register_expr_scope_owner(
    owners: &mut HashMap<String, Option<String>>,
    scope: String,
    module_name: &str,
) {
    if let Some(existing) = owners.get_mut(&scope) {
        if existing.as_deref() != Some(module_name) {
            *existing = None;
        }
    } else {
        owners.insert(scope, Some(module_name.to_string()));
    }
}

fn collect_expr_scope_owners(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    module_name: &str,
    owners: &mut HashMap<String, Option<String>>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        match actual_decl {
            Decl::Defn { name, .. } => {
                let scope = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                register_expr_scope_owner(owners, scope, module_name);
            }
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let prefix = module_prefix
                    .map(|outer| format!("{outer}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_expr_scope_owners(body, Some(&prefix), module_name, owners);
            }
            Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } => {
                for method in methods {
                    let method = match method {
                        Decl::Private { inner, .. } => inner.as_ref(),
                        other => other,
                    };
                    if let Decl::Defn { name, .. } = method {
                        register_expr_scope_owner(
                            owners,
                            format!("{}::{}{}{}", trait_name, name, '$', type_name),
                            module_name,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn try_build_unrestricted_merged_scc_surfaces(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    inferred_private_names: &[String],
    merged_expr_types: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    results_by_module: &mut HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>>,
) -> Option<HashMap<String, ModuleTypeSurface>> {
    if !inferred_private_names.is_empty()
        || group.iter().any(|module_name| {
            direct_imports
                .get(module_name)
                .into_iter()
                .flat_map(HashMap::values)
                .any(|import| import.only.is_some())
        })
    {
        return None;
    }

    let mut owners = HashMap::new();
    for module_name in group {
        let program = parsed_modules.get(module_name)?;
        collect_expr_scope_owners(&program.decls, None, module_name, &mut owners);
        results_by_module.get(module_name)?;
    }

    let mut expr_types_by_module: HashMap<String, HashMap<ExprTypeKey, lsharp_types::types::Type>> =
        HashMap::new();
    for (key, ty) in merged_expr_types {
        let Some(Some(module_name)) = owners.get(&key.scope) else {
            return None;
        };
        expr_types_by_module
            .entry(module_name.clone())
            .or_default()
            .insert(key.clone(), ty.clone());
    }

    let mut surfaces = HashMap::new();
    for module_name in group {
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results: results_by_module.remove(module_name)?,
                hidden: HashSet::new(),
                expr_types: expr_types_by_module.remove(module_name).unwrap_or_default(),
            },
        );
    }
    Some(surfaces)
}

/// SCC の merged inference 用に宣言を連結する。
///
/// 同じ import 宣言が SCC 内の複数 module に現れても、型環境への注入は一度で足りる。
/// ただし `:only`、alias、`open` が異なる import は意味が異なるため、完全一致する宣言
/// だけを重複除去する。宣言の順序と defn の所属 module は維持する。
type SccImportKey = (String, Option<String>, Option<Vec<String>>, bool);

pub(super) fn merge_scc_declarations(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
) -> Result<(Vec<lsharp_syntax::ast::Decl>, Vec<String>), String> {
    use lsharp_syntax::ast::Decl;

    let mut merged_decls = Vec::new();
    let mut defn_origins = Vec::new();
    let mut seen_imports: HashSet<SccImportKey> = HashSet::new();

    for module_name in group {
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        for decl in &program.decls {
            match decl {
                Decl::ModuleDecl { body, .. } if body.is_empty() => {}
                Decl::ImportDecl {
                    module,
                    alias,
                    only,
                    open,
                    ..
                } => {
                    let key = (module.clone(), alias.clone(), only.clone(), *open);
                    if seen_imports.insert(key) {
                        merged_decls.push(decl.clone());
                    }
                }
                _ => {
                    push_defn_origins_infer_order(
                        std::slice::from_ref(decl),
                        module_name,
                        None,
                        &mut defn_origins,
                    );
                    merged_decls.push(decl.clone());
                }
            }
        }
    }

    Ok((merged_decls, defn_origins))
}

fn infer_scc_type_surfaces(
    group: &[String],
    graph: &module_graph::ModuleGraph,
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    module_paths: &HashMap<String, std::path::PathBuf>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    known_surfaces: &HashMap<String, ModuleTypeSurface>,
) -> Result<HashMap<String, ModuleTypeSurface>, String> {
    use lsharp_syntax::ast::Program;

    let group_set: HashSet<&str> = group.iter().map(String::as_str).collect();
    if group.len() == 1 {
        let module_name = &group[0];
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        let mut infer = lsharp_types::infer::Infer::new();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        note_incremental_type_infer();
        let results = infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: [{}] {error}", error.code())
        })?;
        return Ok(HashMap::from([(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            },
        )]));
    }

    let (merged_decls, defn_origins) = merge_scc_declarations(group, parsed_modules)?;

    let mut infer = lsharp_types::infer::Infer::new();
    for module_name in group {
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
    }

    let merged = Program {
        decls: merged_decls,
    };
    let type_results = infer.infer_program(&merged).map_err(|error| {
        let path = group
            .first()
            .and_then(|module| module_paths.get(module))
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| group.join(", "));
        format!("{path}: [{}] {error}", error.code())
    })?;
    if type_results.len() != defn_origins.len() {
        return Err(format!(
            "SCC の型結果数が宣言数と一致しません: modules={}, results={}, origins={}",
            group.join(", "),
            type_results.len(),
            defn_origins.len()
        ));
    }

    let mut results_by_module: HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>> =
        HashMap::new();
    for ((name, scheme), origin) in type_results.into_iter().zip(defn_origins) {
        results_by_module
            .entry(origin)
            .or_default()
            .push((name, scheme));
    }

    let inferred_private_names = infer.module_env.privates.clone();
    let merged_expr_types = infer.expr_type_results_snapshot();
    if let Some(surfaces) = try_build_unrestricted_merged_scc_surfaces(
        group,
        parsed_modules,
        direct_imports,
        &inferred_private_names,
        &merged_expr_types,
        &mut results_by_module,
    ) {
        note_incremental_scc_merged_fast_path();
        return Ok(surfaces);
    }

    let mut provisional_surfaces = HashMap::new();
    for module_name in group {
        let results = results_by_module.remove(module_name).unwrap_or_default();
        let mut private_names = HashSet::new();
        if let Some(program) = parsed_modules.get(module_name) {
            collect_private_surface_names(&program.decls, None, &mut private_names);
        }
        let hidden = inferred_private_names
            .iter()
            .filter(|name| private_names.contains(*name))
            .cloned()
            .collect();
        provisional_surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden,
                expr_types: HashMap::new(),
            },
        );
    }

    // merged prepass で相互再帰の型を確定した後、各 module を元の import visibility
    // で再検証する。これにより SCC 内でも `:only` / private の境界を失わない。
    let mut surfaces = HashMap::new();
    for module_name in group {
        let mut module_infer = lsharp_types::infer::Infer::new();
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = provisional_surfaces
                    .get(&dependency)
                    .or_else(|| known_surfaces.get(&dependency))
            {
                module_infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        let results = module_infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: [{}] {error}", error.code())
        })?;
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: module_infer.module_env.privates.iter().cloned().collect(),
                expr_types: module_infer.expr_type_results_snapshot(),
            },
        );
    }

    Ok(surfaces)
}
