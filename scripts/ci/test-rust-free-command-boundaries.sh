#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-rust-free-boundaries.XXXXXX")"

FAIL_MARKER="$TMP_ROOT/cargo-invoked"
export FAIL_MARKER
mkdir -p "$TMP_ROOT/bin"
cat >"$TMP_ROOT/bin/cargo" <<'SH'
#!/usr/bin/env bash
printf '%s\n' invoked >"$FAIL_MARKER"
exit 99
SH
chmod 755 "$TMP_ROOT/bin/cargo"

cleanup() {
  rm -rf "$TMP_ROOT" \
    "$ROOT/target/ci/test-rust-free-boundaries" \
    "$ROOT/target/ci/test-rust-free-boundaries-phase11"
}
trap cleanup EXIT

run_without_binary() {
  local label="$1"
  shift
  local output="$TMP_ROOT/$label.output"
  local status=0

  set +e
  PATH="$TMP_ROOT/bin:$PATH" "$@" >"$output" 2>&1
  status=$?
  set -e

  [[ "$status" -ne 0 ]] || {
    echo "FAIL: $label unexpectedly succeeded without LSHARP_BIN" >&2
    cat "$output" >&2
    exit 1
  }
  [[ ! -f "$FAIL_MARKER" ]] || {
    echo "FAIL: $label invoked cargo while LSHARP_BIN was missing" >&2
    cat "$output" >&2
    exit 1
  }
  grep -Eq 'LSHARP_BIN|lsharp binary' "$output" || {
    echo "FAIL: $label did not explain how to provide a binary" >&2
    cat "$output" >&2
    exit 1
  }
}

run_without_binary \
  default-path-smoke \
  env LSHARP_BIN="$TMP_ROOT/missing" OUT_DIR="$ROOT/target/ci/test-rust-free-boundaries" \
  bash "$ROOT/scripts/ci/default-path-smoke.sh"

run_without_binary \
  compile-phase11-inputs \
  env LSHARP_BIN="$TMP_ROOT/missing" OUT_DIR="$ROOT/target/ci/test-rust-free-boundaries-phase11" \
  bash "$ROOT/scripts/ci/compile-phase11-inputs.sh"

run_without_binary \
  readme-smoke \
  env LSHARP_BIN="$TMP_ROOT/missing" SMOKE_DIR="$TMP_ROOT/readme" \
  bash "$ROOT/scripts/smoke_test_readme.sh"

echo "rust-free command boundaries: OK"
