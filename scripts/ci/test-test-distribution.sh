#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$ROOT/scripts/test-distribution.py"
FIRST="$(mktemp "${TMPDIR:-/tmp}/lsharp-test-distribution.XXXXXX")"
SECOND="$(mktemp "${TMPDIR:-/tmp}/lsharp-test-distribution.XXXXXX")"
TEXT="$(mktemp "${TMPDIR:-/tmp}/lsharp-test-distribution.XXXXXX")"
trap 'rm -f "$FIRST" "$SECOND" "$TEXT"' EXIT

python3 "$SCRIPT" --root "$ROOT" --json >"$FIRST"
python3 "$SCRIPT" --root "$ROOT" --json >"$SECOND"
cmp -s "$FIRST" "$SECOND"
python3 "$SCRIPT" --root "$ROOT" >"$TEXT"
python3 - "$TEXT" <<'PY'
from pathlib import Path
import sys

lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
assert lines[0].split("\t") == [
    "crate",
    "rust_files",
    "test_attributes",
    "test_functions",
    "proptest_macros",
    "ignored_tests",
]
assert lines[-1].startswith("TOTAL\t")
assert all(value.isdigit() for value in lines[-1].split("\t")[1:])
PY

python3 - "$FIRST" <<'PY'
import json
import sys

payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
assert payload["schema_version"] == 1
expected = {
    "lsharp-docs",
    "lsharp-driver",
    "lsharp-ir",
    "lsharp-lsp",
    "lsharp-syntax",
    "lsharp-tooling",
    "lsharp-types",
    "lsharp-wasm",
}
crates = {entry["name"]: entry for entry in payload["crates"]}
assert set(crates) == expected, (set(crates), expected)
for entry in crates.values():
    assert entry["rust_files"] >= 0
    assert entry["test_attributes"] >= 0
    assert entry["test_functions"] >= 0
    assert entry["proptest_macros"] >= 0
    assert entry["ignored_tests"] >= 0
assert crates["lsharp-syntax"]["test_functions"] > 0
assert crates["lsharp-types"]["test_functions"] > 0
assert payload["totals"]["test_functions"] == sum(
    entry["test_functions"] for entry in crates.values()
)
PY

echo "test distribution contract: OK"
