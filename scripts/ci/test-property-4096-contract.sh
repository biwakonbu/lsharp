#!/usr/bin/env bash
set -euo pipefail

readonly root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly property_script="$root_dir/scripts/ci/test-property-nightly.sh"

output="$(env -u PROPTEST_CASES -u PROPTEST_RNG_SEED "$property_script" --dry-run)"

grep -Fq 'PROPTEST_CASES=4096' <<<"$output"
grep -Fq 'PROPTEST_RNG_SEED=20260725' <<<"$output"
grep -Fq 'lsharp-syntax --lib parser_never_panics_for_bounded_arbitrary_bytes' <<<"$output"
grep -Fq 'lsharp-syntax --lib pretty_printed_ast_reparses_to_the_same_source' <<<"$output"
grep -Fq 'lsharp-types --lib unify_success_is_symmetric' <<<"$output"
grep -Fq 'lsharp-types --lib bounded_expression_inference_never_panics' <<<"$output"

printf '%s\n' 'property 4096-case lane contract passed'
