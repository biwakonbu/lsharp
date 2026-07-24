#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly runtime_script="$root_dir/scripts/ci/test-runtime-limits.sh"

output="$("$runtime_script" --dry-run)"

grep -Fq 'e2e::runtime_allocator_closures::test_e2e_alloc_memory_grow_failure_does_not_return_out_of_bounds_address' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_object_table_grows_past_initial_capacity' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_free_list_grows_past_initial_capacity' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_free_list_growth_reuses_moved_entries' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_grows_past_initial_capacity' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_growth_preserves_root_api' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_reuses_unrooted_allocations_across_repeated_start_series' <<<"$output"

printf '%s\n' 'runtime limit lane contract passed'
