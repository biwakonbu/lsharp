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

readonly test_filter="e2e::runtime_recursion_limits::test_e2e_runtime_recursion_stack_limit_reports_trap"
readonly command=(
    cargo test -p lsharp-wasm --test e2e "$test_filter"
    -- --exact --nocapture --test-threads=1
)

printf '%s\n' "cargo test -p lsharp-wasm --test e2e $test_filter -- --exact --nocapture --test-threads=1"
if [[ "$dry_run" == true ]]; then
    exit 0
fi

(cd -- "$root_dir" && "${command[@]}")
