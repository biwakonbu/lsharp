#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
dry_run=false

if [[ "${1:-}" == "--dry-run" ]]; then
    dry_run=true
elif [[ "${1:-}" == "--help" ]]; then
    printf 'usage: %s [--dry-run]\n' "${BASH_SOURCE[0]}"
    exit 0
elif [[ -n "${1:-}" ]]; then
    printf 'usage: %s [--dry-run]\n' "${BASH_SOURCE[0]}" >&2
    exit 2
fi

run_gc_root_test() {
    local filter="$1"
    local command=(
        cargo test -p lsharp-wasm --test e2e "$filter"
        -- --exact --nocapture --test-threads=1
    )

    printf '%s\n' "cargo test -p lsharp-wasm --test e2e $filter -- --exact --nocapture --test-threads=1"
    if [[ "$dry_run" == true ]]; then
        return
    fi

    (cd -- "$root_dir" && "${command[@]}")
}

run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_direct_rooted_string_across_trigger
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_ref_cell
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_map_value
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_closure_capture
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_non_self_recursive_heap_param
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_let_heap_local_across_alloc
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_opaque_nested_call_result_across_forced_gc
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_pattern_bound_heap_field_across_alloc
run_gc_root_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_ignores_legacy_zero_root_slot_sentinel
