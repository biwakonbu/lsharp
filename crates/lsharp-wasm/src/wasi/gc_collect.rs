use super::*;

#[path = "gc_collect_core.rs"]
mod core;

pub(super) fn emit_gc_collect_func(codes: &mut CodeSection, globals: CollectorGlobals) {
    core::emit_gc_collect_func(codes, globals);
}
