#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly lane_script="$root_dir/scripts/ci/test-selfhost-rooting-guards.sh"

output="$($lane_script --dry-run)"

grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_string_concat_auto_roots_arguments' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_string_concat_roots_lhs_before_lowering_rhs' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_substring_auto_roots_source' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_substring_roots_source_before_lowering_index_exprs' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_ref_new_auto_roots_wrapped_value' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_vector_push_auto_roots_realloc_inputs' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_vector_push_roots_vector_before_lowering_value_expr' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_map_insert_auto_roots_receiver_key_value' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_map_get_auto_roots_receiver_and_key' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_user_call_roots_first_arg_before_lowering_later_args' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_user_call_auto_roots_arguments_until_call' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_let_roots_heap_binding_before_lowering_later_let_init' <<<"$output"
grep -Fq 'e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_let_chain_roots_final_body_before_root_pop_drops' <<<"$output"

printf '%s\n' 'selfhost rooting guard lane contract passed'
