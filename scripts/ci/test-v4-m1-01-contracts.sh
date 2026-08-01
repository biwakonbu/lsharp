#!/usr/bin/env bash

# V4-M1-01 の fast contract gate。個別コマンドは失敗箇所の診断用に残し、通常はこの入口を使う。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

run_contract() {
  local name="$1"
  shift
  printf '\n== %s ==\n' "$name"
  "$@"
}

run_contract "fixture matrix" \
  python3 scripts/ci/test-semantic-fixture-matrix.py
run_contract "fixture diff" \
  python3 scripts/ci/test-semantic-fixture-diff.py
run_contract "Rust oracle producer" \
  python3 scripts/ci/test-semantic-fixture-rust-report.py
run_contract "native stage0 producer" \
  python3 scripts/ci/test-semantic-fixture-native-report.py
run_contract "evidence schema" \
  python3 scripts/ci/test-semantic-fixture-evidence-schema.py
run_contract "evidence audit" \
  python3 scripts/ci/test-semantic-fixture-evidence-audit.py
run_contract "aggregate schema" \
  python3 scripts/ci/test-semantic-fixture-evidence-aggregate-schema.py
run_contract "aggregate audit" \
  python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py
run_contract "producer command docs" \
  python3 scripts/ci/test-semantic-fixture-producer-docs.py
run_contract "repository docs audit" \
  bash scripts/audit_docs.sh
run_contract "whitespace audit" \
  git diff --check

printf '\nV4-M1-01 contract gate: PASS\n'
