#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
dry_run=false
profile=verified

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            dry_run=true
            ;;
        --profile)
            shift
            if [[ $# -eq 0 ]]; then
                printf '%s\n' '--profile requires verified, soak, or all' >&2
                exit 2
            fi
            profile="$1"
            ;;
        --help)
            printf 'usage: %s [--dry-run] [--profile verified|soak|all]\n' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            printf 'usage: %s [--dry-run] [--profile verified|soak|all]\n' "${BASH_SOURCE[0]}" >&2
            exit 2
            ;;
    esac
    shift
done

run_gc_soak_test() {
    local filter="$1"
    local profile="${2:-normal}"
    local ignored_suffix=""
    if [[ "$profile" == "ignored" ]]; then
        ignored_suffix=" --ignored"
    elif [[ "$profile" != "normal" ]]; then
        printf 'unknown test profile: %s\n' "$profile" >&2
        exit 2
    fi

    local command=(
        cargo test -p lsharp-wasm --test e2e "$filter"
        -- --exact --nocapture --test-threads=1
    )
    if [[ "$profile" == "ignored" ]]; then
        command+=(--ignored)
    fi

    printf '%s\n' "cargo test -p lsharp-wasm --test e2e $filter -- --exact --nocapture --test-threads=1${ignored_suffix}"
    if [[ "$dry_run" == true ]]; then
        return
    fi

    (cd -- "$root_dir" && "${command[@]}")
}

run_verified_profile() {
    run_gc_soak_test e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_reclaims_unrooted_allocations
    run_gc_soak_test e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_preserves_rooted_allocations_and_reuses_freed_block
    run_gc_soak_test e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_gc_collect_ignores_legacy_zero_root_slot_sentinel
}

run_soak_profile() {
    run_gc_soak_test e2e::selfhost_gc_stateful_soak::test_e2e_gc_repl_stateful_long_session_in_session_collector_telemetry ignored
    run_gc_soak_test e2e::selfhost_gc_stateful_soak::test_e2e_gc_lsp_actual_stdio_repeated_sequence_in_session_collector_telemetry ignored
}

case "$profile" in
    verified)
        run_verified_profile
        ;;
    soak)
        run_soak_profile
        ;;
    all)
        run_verified_profile
        run_soak_profile
        ;;
    *)
        printf 'unknown profile: %s (expected verified, soak, or all)\n' "$profile" >&2
        exit 2
        ;;
esac
