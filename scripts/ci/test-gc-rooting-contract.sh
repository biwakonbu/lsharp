#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly rooting_script="$root_dir/scripts/ci/test-gc-rooting.sh"

output="$("$rooting_script" --dry-run)"

grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_direct_rooted_string_across_trigger' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_ref_cell' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_map_value' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_string_reachable_through_rooted_closure_capture' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_non_self_recursive_heap_param' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_let_heap_local_across_alloc' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_opaque_nested_call_result_across_forced_gc' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_preserves_pattern_bound_heap_field_across_alloc' <<<"$output"
grep -Fq 'e2e::runtime_allocator_closures::test_e2e_runtime_collector_ignores_legacy_zero_root_slot_sentinel' <<<"$output"

printf '%s\n' 'GC rooting lane contract passed'
