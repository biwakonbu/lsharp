pub(super) fn parse_program_for_incremental(
    source: &str,
) -> Result<lsharp_syntax::ast::Program, lsharp_syntax::ParseAllError> {
    #[cfg(test)]
    {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_PARSE_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
    lsharp_syntax::parse(source)
}

/// incremental 経路の parse choke point。
///
/// cache hit / fresh parse のどちらでも、返す前に block 形式 module body を弾く
/// (`I-39`)。ここを唯一の入口にしておかないと、呼び出し側 6 箇所のどれかで
/// 検査を落としても誰も気付けない。
///
/// error は既に整形済みの `String` で返す。呼び出し側はすべて直後に
/// `format!` で String へ落としており、`ParseAllError` を保持していなかった。
pub(super) fn cached_program_or_parse(
    mod_name: &str,
    source: &str,
    fingerprint: SourceFingerprint,
    cache: &CompilationCache,
) -> Result<Arc<lsharp_syntax::ast::Program>, String> {
    let program = if let Some(entry) = cache.get(mod_name)
        && entry.fingerprint() == fingerprint
    {
        entry.ast_arc()
    } else {
        Arc::new(
            parse_program_for_incremental(source)
                .map_err(|e| format!("[{}] {e}", e.code()))?,
        )
    };
    crate::module_body_form::reject_block_form_module_body(program.as_ref())?;
    Ok(program)
}

fn note_incremental_type_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_merged_fast_path() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_lower() {
    #[cfg(test)]
    {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LOWER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_module_segment_lower_by(_count: usize) {
    #[cfg(test)]
    {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|slot| {
                    slot.set(slot.get() + _count);
                });
            }
        });
    }
}

fn note_incremental_link_full() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_link_cache_hit() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn build_module_cache_entry(
    fingerprint: SourceFingerprint,
    deps_key: u64,
    program: &Arc<lsharp_syntax::ast::Program>,
    type_surface: ModuleTypeSurface,
) -> ModuleCacheEntry {
    ModuleCacheEntry::new(
        fingerprint,
        deps_key,
        Arc::clone(program),
        type_surface,
        Module {
            functions: Vec::new(),
            gc_types: Vec::new(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: Vec::new(),
        },
        ModuleIrSegments::empty(),
        collect_import_modules(program.as_ref()),
    )
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(super) enum MultiFileLoweringMode {
    Merged,
    Modular,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SegmentRange {
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleLinkRanges {
    defns_functions: SegmentRange,
    accessors_functions: SegmentRange,
    trait_impls_functions: SegmentRange,
    constraints_functions: SegmentRange,
    ctors_functions: SegmentRange,
    defn_lifted_functions: SegmentRange,
    trait_impl_lifted_functions: SegmentRange,
    defns_gc_types: SegmentRange,
    defns_string_data: SegmentRange,
    trait_impls_string_data: SegmentRange,
}

fn module_has_content(module: &Module) -> bool {
    !module.functions.is_empty()
        || !module.gc_types.is_empty()
        || !module.imports.is_empty()
        || !module.globals.is_empty()
        || !module.string_data.is_empty()
}

fn build_segment_module(
    functions: Vec<Function>,
    gc_types: Vec<GcTypeDef>,
    string_data: Vec<(String, Vec<u8>)>,
) -> Module {
    Module {
        functions,
        gc_types,
        imports: Vec::new(),
        globals: Vec::new(),
        string_data,
    }
}

fn link_modules_preserving_indices(modules: &[Module]) -> Module {
    let mut linked = Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    };

    for module in modules {
        linked.functions.extend(module.functions.clone());
        linked.gc_types.extend(module.gc_types.clone());
        linked.imports.extend(module.imports.clone());
        linked.globals.extend(module.globals.clone());
        linked.string_data.extend(module.string_data.clone());
    }

    linked
}

fn lower_multi_file_merged(
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.lower_program_with_expr_types(
        &merged_program,
        all_type_results,
        all_expr_type_results,
    )
}

fn prime_cached_string_data(lower_ctx: &mut lower::Lower, string_data: &[(String, Vec<u8>)]) {
    for (label, bytes) in string_data {
        lower_ctx.string_data.push((label.clone(), bytes.clone()));
        lower_ctx.string_offset += bytes.len() as u32;
    }
}

fn prime_cached_lifted(lower_ctx: &mut lower::Lower, module: &Module) {
    lower_ctx.lambda_counter += module.functions.len() as u32;
    lower_ctx.lifted_functions.extend(module.functions.clone());
}

fn link_module_ir_segments(segments: &[ModuleIrSegments]) -> Module {
    let mut modules = Vec::new();

    for segment in segments {
        if module_has_content(segment.defns()) {
            modules.push(segment.defns().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.accessors()) {
            modules.push(segment.accessors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impls()) {
            modules.push(segment.trait_impls().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.constraints()) {
            modules.push(segment.constraints().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.ctors()) {
            modules.push(segment.ctors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.defn_lifted()) {
            modules.push(segment.defn_lifted().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impl_lifted()) {
            modules.push(segment.trait_impl_lifted().clone());
        }
    }

    link_modules_preserving_indices(&modules)
}

fn next_segment_range(cursor: &mut usize, len: usize) -> SegmentRange {
    let range = SegmentRange {
        start: *cursor,
        len,
    };
    *cursor += len;
    range
}

fn compute_module_link_ranges(segments: &[ModuleIrSegments]) -> Vec<ModuleLinkRanges> {
    let mut ranges = vec![ModuleLinkRanges::default(); segments.len()];

    let mut function_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_functions =
            next_segment_range(&mut function_cursor, segment.defns().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].accessors_functions =
            next_segment_range(&mut function_cursor, segment.accessors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_functions =
            next_segment_range(&mut function_cursor, segment.trait_impls().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].constraints_functions =
            next_segment_range(&mut function_cursor, segment.constraints().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].ctors_functions =
            next_segment_range(&mut function_cursor, segment.ctors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defn_lifted_functions =
            next_segment_range(&mut function_cursor, segment.defn_lifted().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impl_lifted_functions = next_segment_range(
            &mut function_cursor,
            segment.trait_impl_lifted().functions.len(),
        );
    }

    let mut gc_type_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_gc_types =
            next_segment_range(&mut gc_type_cursor, segment.defns().gc_types.len());
    }

    let mut string_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_string_data =
            next_segment_range(&mut string_cursor, segment.defns().string_data.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_string_data =
            next_segment_range(&mut string_cursor, segment.trait_impls().string_data.len());
    }

    ranges
}

fn segment_layout_matches(old: &ModuleIrSegments, new: &ModuleIrSegments) -> bool {
    old.defns().functions.len() == new.defns().functions.len()
        && old.accessors().functions.len() == new.accessors().functions.len()
        && old.trait_impls().functions.len() == new.trait_impls().functions.len()
        && old.constraints().functions.len() == new.constraints().functions.len()
        && old.ctors().functions.len() == new.ctors().functions.len()
        && old.defn_lifted().functions.len() == new.defn_lifted().functions.len()
        && old.trait_impl_lifted().functions.len() == new.trait_impl_lifted().functions.len()
        && old.defns().gc_types.len() == new.defns().gc_types.len()
        && old.defns().string_data.len() == new.defns().string_data.len()
        && old.trait_impls().string_data.len() == new.trait_impls().string_data.len()
}

fn can_patch_linked_module(
    cache: &CompilationCache,
    module_order: &[String],
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> bool {
    cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
        && old_segments.len() == new_segments.len()
        && old_segments
            .iter()
            .zip(new_segments.iter())
            .all(|(old, new)| segment_layout_matches(old, new))
}

fn overwrite_range<T: Clone>(target: &mut [T], range: SegmentRange, replacement: &[T]) {
    debug_assert_eq!(range.len, replacement.len());
    target[range.start..range.start + range.len].clone_from_slice(replacement);
}

fn patch_linked_module(
    base: &Module,
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> Module {
    let ranges = compute_module_link_ranges(old_segments);
    let mut patched = base.clone();

    for (range, segment) in ranges.iter().zip(new_segments.iter()) {
        overwrite_range(
            &mut patched.functions,
            range.defns_functions,
            &segment.defns().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.accessors_functions,
            &segment.accessors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impls_functions,
            &segment.trait_impls().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.constraints_functions,
            &segment.constraints().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.ctors_functions,
            &segment.ctors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.defn_lifted_functions,
            &segment.defn_lifted().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impl_lifted_functions,
            &segment.trait_impl_lifted().functions,
        );
        overwrite_range(
            &mut patched.gc_types,
            range.defns_gc_types,
            &segment.defns().gc_types,
        );
        overwrite_range(
            &mut patched.string_data,
            range.defns_string_data,
            &segment.defns().string_data,
        );
        overwrite_range(
            &mut patched.string_data,
            range.trait_impls_string_data,
            &segment.trait_impls().string_data,
        );
    }

    patched
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModulePrecomputedShape {
    defn_count: usize,
    accessor_count: usize,
    trait_impl_count: usize,
    constraint_count: usize,
    ctor_count: usize,
    gc_type_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleDefnStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleTraitImplStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

fn module_precomputed_shape_from_program(
    program: &lsharp_syntax::ast::Program,
) -> ModulePrecomputedShape {
    use lsharp_syntax::ast::Decl;

    let mut shape = ModulePrecomputedShape::default();
    for decl in &program.decls {
        match decl {
            Decl::Defn { .. } => shape.defn_count += 1,
            Decl::RecordDef { fields, .. } => {
                shape.accessor_count += fields.len();
                shape.gc_type_count += 1;
            }
            Decl::ImplDef { methods, .. } => {
                shape.trait_impl_count += methods.len();
            }
            Decl::TypeConstrained { .. } => {
                shape.constraint_count += 2;
            }
            Decl::TypeDef { variants, .. } => {
                shape.ctor_count += variants.len();
            }
            Decl::ModuleDecl { .. } | Decl::ImportDecl { .. } => {}
            _ => {}
        }
    }
    shape
}

fn module_precomputed_shape_from_segments(segments: &ModuleIrSegments) -> ModulePrecomputedShape {
    ModulePrecomputedShape {
        defn_count: segments.defns().functions.len(),
        accessor_count: segments.accessors().functions.len(),
        trait_impl_count: segments.trait_impls().functions.len(),
        constraint_count: segments.constraints().functions.len(),
        ctor_count: segments.ctors().functions.len(),
        gc_type_count: segments.defns().gc_types.len(),
    }
}

fn module_defn_state_shape(module: &ModuleIrSegments) -> ModuleDefnStateShape {
    ModuleDefnStateShape {
        string_bytes: module
            .defns()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.defn_lifted().functions.len(),
    }
}

fn module_trait_impl_state_shape(module: &ModuleIrSegments) -> ModuleTraitImplStateShape {
    ModuleTraitImplStateShape {
        string_bytes: module
            .trait_impls()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.trait_impl_lifted().functions.len(),
    }
}

fn defn_state_depends_on_prefix(shape: ModuleDefnStateShape) -> bool {
    shape.string_bytes > 0 || shape.lifted_count > 0
}

struct ModularLoweringResult {
    segments: Vec<ModuleIrSegments>,
    fresh_defn_lower_count: usize,
}
