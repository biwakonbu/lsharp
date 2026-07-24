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

run_runtime_limit_test() {
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

run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_alloc_memory_grow_failure_does_not_return_out_of_bounds_address
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_alloc_memory_grow_failure_reports_ls4002
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_object_table_grows_past_initial_capacity
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_free_list_grows_past_initial_capacity
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_free_list_growth_reuses_moved_entries
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_grows_past_initial_capacity
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_growth_preserves_root_api
run_runtime_limit_test e2e::runtime_allocator_closures::test_e2e_runtime_collector_reuses_unrooted_allocations_across_repeated_start_series
