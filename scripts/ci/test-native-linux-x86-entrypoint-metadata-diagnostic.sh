#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="${ROOT_DIR}/scripts/ci/fixtures/native-linux-x86-entrypoint-metadata.txt"
DIAGNOSTIC="${ROOT_DIR}/scripts/ci/diagnose-native-linux-x86-entrypoint-metadata.py"

output="$(python3 "${DIAGNOSTIC}" "${FIXTURE}" --function-index 3418)"
DIAGNOSTIC_OUTPUT="${output}" python3 - <<'PY'
import json
import os

report = json.loads(os.environ["DIAGNOSTIC_OUTPUT"])
assert report["function_index"] == 3418
assert report["call_count"] == 1
row = report["calls"][0]
assert row["instr_idx"] == 0
assert row["opcode"] == 40
assert row["operand"] == 3416
assert row["offset"] == 11
assert row["rel32"] == -24867
assert row["target_offset"] == -24851
assert row["expected_target_offset"] == -24851
assert row["expected_rel32"] == -24867
assert row["emitted_bytes"] == "e8dd9effff"
assert row["expected_bytes"] == "e8dd9effff"
assert row["bytes_match"] is True
assert row["rel32_match"] is True
assert row["target_match"] is True
PY

mismatch="$(mktemp -t lsharp-native-linux-x86-entrypoint-metadata.XXXXXX)"
trap 'rm -f "${mismatch}"' EXIT
awk 'BEGIN { changed = 0 } !changed && $0 == "221" { print "222"; changed = 1; next } { print }' \
  "${FIXTURE}" >"${mismatch}"
if python3 "${DIAGNOSTIC}" "${mismatch}" --function-index 3418 >/dev/null 2>&1; then
  echo "diagnostic unexpectedly accepted mismatched rel32 bytes" >&2
  exit 1
fi

echo "native Linux x86 entrypoint metadata diagnostic passed"
