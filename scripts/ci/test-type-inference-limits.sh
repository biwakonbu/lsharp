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

run_type_limit_test() {
    local filter="$1"
    local command=(
        cargo test -p lsharp-types --test infer_limits "$filter"
        -- --exact --nocapture --test-threads=1
    )

    printf '%s\n' "cargo test -p lsharp-types --test infer_limits $filter -- --exact --nocapture --test-threads=1"
    if [[ "$dry_run" == true ]]; then
        return
    fi

    (cd -- "$root_dir" && "${command[@]}")
}

run_type_limit_test self_application_reports_infinite_type
run_type_limit_test wide_record_type_annotations_do_not_panic
run_type_limit_test deeply_nested_type_annotations_do_not_panic
