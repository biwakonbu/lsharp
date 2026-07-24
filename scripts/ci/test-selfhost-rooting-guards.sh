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

run_rooting_guard() {
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

run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_string_concat_auto_roots_arguments
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_string_concat_roots_lhs_before_lowering_rhs
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_substring_auto_roots_source_string
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_substring_roots_source_before_lowering_index_exprs
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_ref_new_auto_roots_wrapped_value
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_vector_push_auto_roots_realloc_inputs
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_vector_push_roots_vector_before_lowering_value_expr
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_map_insert_auto_roots_receiver_key_value
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_map_get_auto_roots_receiver_and_key
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_user_call_roots_first_arg_before_lowering_later_args
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_user_call_auto_roots_arguments_until_call
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_let_roots_heap_binding_before_lowering_later_let_init
run_rooting_guard e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_let_chain_roots_final_body_before_root_pop_drops
