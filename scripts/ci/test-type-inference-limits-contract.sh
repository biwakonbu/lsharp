#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly lane_script="$root_dir/scripts/ci/test-type-inference-limits.sh"

output="$($lane_script --dry-run)"

grep -Fq 'self_application_reports_infinite_type' <<<"$output"
grep -Fq 'wide_record_type_annotations_do_not_panic' <<<"$output"
grep -Fq 'deeply_nested_type_annotations_do_not_panic' <<<"$output"

printf '%s\n' 'type inference limit lane contract passed'
