use super::compile_surface::{
    ImportVisibilitySpec, ModuleTypeSurface, collect_import_modules, collect_import_visibility,
    dependency_surface_key, push_defn_origins_infer_order,
};
use super::{
    CompilationCache, Function, GcTypeDef, Module, ModuleCacheEntry, ModuleIrSegments,
    SourceFingerprint, lower, module_graph,
};
use lsharp_types::infer::ExprTypeKey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(test)]
use super::{
    INCREMENTAL_LINK_CACHE_HIT_COUNT, INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED,
    INCREMENTAL_LINK_FULL_COUNT, INCREMENTAL_LINK_FULL_TRACKING_ENABLED, INCREMENTAL_LOWER_COUNT,
    INCREMENTAL_LOWER_TRACKING_ENABLED, INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT,
    INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED, INCREMENTAL_PARSE_COUNT,
    INCREMENTAL_PARSE_TRACKING_ENABLED, INCREMENTAL_SCC_INFER_COUNT,
    INCREMENTAL_SCC_INFER_TRACKING_ENABLED, INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT,
    INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED, INCREMENTAL_TYPE_INFER_COUNT,
    INCREMENTAL_TYPE_INFER_TRACKING_ENABLED,
};

include!("compile_support.rs");
include!("compile_pipeline.rs");
include!("compile_entrypoints.rs");
include!("compile_incremental.rs");
