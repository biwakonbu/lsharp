#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly lane_script="$root_dir/scripts/ci/test-gc-soak-telemetry.sh"

verified_output="$($lane_script --profile verified --dry-run)"
soak_output="$($lane_script --profile soak --dry-run)"
all_output="$($lane_script --profile all --dry-run)"

grep -Fq 'e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_reclaims_unrooted_allocations' <<<"$verified_output"
grep -Fq 'e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_preserves_rooted_allocations_and_reuses_freed_block' <<<"$verified_output"
grep -Fq 'e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_ignores_legacy_zero_root_slot_sentinel' <<<"$verified_output"
if grep -Fq -- '--ignored' <<<"$verified_output"; then
    printf '%s\n' 'verified profile must not run ignored tests' >&2
    exit 1
fi

grep -Fq 'e2e::selfhost_gc_stateful_soak::test_e2e_gc_repl_stateful_long_session_in_session_collector_telemetry' <<<"$soak_output"
grep -Fq 'e2e::selfhost_gc_stateful_soak::test_e2e_gc_lsp_actual_stdio_repeated_sequence_in_session_collector_telemetry' <<<"$soak_output"
grep -Fq -- '--ignored' <<<"$soak_output"

verified_count=$(grep -Fc 'cargo test -p lsharp-wasm --test e2e' <<<"$verified_output")
soak_count=$(grep -Fc 'cargo test -p lsharp-wasm --test e2e' <<<"$soak_output")
all_count=$(grep -Fc 'cargo test -p lsharp-wasm --test e2e' <<<"$all_output")
[[ "$verified_count" -eq 3 ]]
[[ "$soak_count" -eq 2 ]]
[[ "$all_count" -eq 5 ]]

printf '%s\n' 'GC soak telemetry lane contract passed'
