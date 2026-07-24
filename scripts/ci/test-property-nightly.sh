#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
property_cases="${PROPTEST_CASES:-4096}"
property_seed="${PROPTEST_RNG_SEED:-20260725}"
dry_run=false

if [[ "${1:-}" == "--dry-run" ]]; then
    dry_run=true
elif [[ -n "${1:-}" ]]; then
    printf 'usage: %s [--dry-run]\n' "${BASH_SOURCE[0]}" >&2
    exit 2
fi

run_property_test() {
    local package="$1"
    local filter="$2"
    local command=(
        env
        "PROPTEST_CASES=$property_cases"
        "PROPTEST_RNG_SEED=$property_seed"
        cargo test -p "$package" --lib "$filter" -- --test-threads=1
    )

    printf '%s\n' "PROPTEST_CASES=$property_cases PROPTEST_RNG_SEED=$property_seed cargo test -p $package --lib $filter -- --test-threads=1"
    if [[ "$dry_run" == true ]]; then
        return
    fi

    (cd -- "$root_dir" && "${command[@]}")
}

run_property_test lsharp-syntax parser_never_panics_for_bounded_arbitrary_bytes
run_property_test lsharp-syntax pretty_printed_ast_reparses_to_the_same_source
run_property_test lsharp-types unify_success_is_symmetric
run_property_test lsharp-types bounded_expression_inference_never_panics
