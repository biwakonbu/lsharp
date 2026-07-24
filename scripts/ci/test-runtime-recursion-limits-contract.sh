#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly runtime_script="$root_dir/scripts/ci/test-runtime-recursion-limits.sh"

output="$("$runtime_script" --dry-run)"
grep -Fq 'e2e::runtime_recursion_limits::test_e2e_runtime_recursion_stack_limit_reports_trap' <<<"$output"

printf '%s\n' 'runtime recursion limit lane contract passed'
