#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SMOKE="$ROOT/scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh"
SOURCE_SMOKE="$ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
SOURCE_COMMIT="$(git rev-parse --verify HEAD)"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_script_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null \
    || fail "$path does not contain: $expected"
}

expect_reject() {
  local label="$1"
  shift

  local output
  local exit_code
  set +e
  output="$($@ 2>&1)"
  exit_code=$?
  set -e
  [[ "$exit_code" -ne 0 ]] || fail "$label unexpectedly succeeded"
  [[ -n "$output" ]] || fail "$label did not report an error"
}

[[ -x "$SMOKE" ]] || fail "Linux native stage0 source-file smoke is missing: $SMOKE"
[[ -x "$SOURCE_SMOKE" ]] || fail "native selfhost source-file smoke is missing: $SOURCE_SMOKE"

for required in \
  'run_expected_failure validation-text-unknown' \
  '--source "$VALIDATION_SOURCE"' \
  '--format text' \
  'require_exact_output validation-text-unknown' \
  'status: unknown' \
  'open-questions: 1' \
  'independent-reviews: 0' \
  'contradicting-observations: 0' \
  'stale-reviews: 0' \
  'stale-evidence: 0' \
  'run_expected_validation_error validation-manifest-write-failure' \
  'VALIDATION_WRITE_FAILURE_MANIFEST' \
  'missing-parent' \
  '--emit-manifest "$VALIDATION_WRITE_FAILURE_MANIFEST"' \
  'source validation manifest write failed' \
  '[[ ! -e "$VALIDATION_WRITE_FAILURE_MANIFEST" ]]' \
  'VALIDATION_PASS_SOURCE' \
  'validation-pass-json' \
  'validation-pass-text' \
  'status: pass' \
  'independent-reviews: 1' \
  'VALIDATION_FAIL_SOURCE' \
  'validation-fail-json' \
  'validation-fail-text' \
  'run_report_failure' \
  'status: fail' \
  'contradicting-observations: 1' \
  'VALIDATION_STALE_SOURCE' \
  'run_expected_failure validation-stale-json' \
  'validation-stale-json' \
  'run_expected_failure validation-stale-text' \
  'validation-stale-text' \
  'stale-reviews: 1' \
  'stale-evidence: 1' \
  'VALIDATION_ORPHAN_SOURCE' \
  'VALIDATION_ORPHAN_MANIFEST' \
  'run_expected_validation_error validation-orphan' \
  'source validation error:5' \
  'VALIDATION_MALFORMED_SOURCE' \
  'VALIDATION_MALFORMED_MANIFEST' \
  'run_expected_validation_error validation-malformed-edge' \
  'source validation error:1' \
  'VALIDATION_INVALID_ID_SOURCE' \
  'VALIDATION_INVALID_ID_MANIFEST' \
  'run_expected_validation_error validation-invalid-id' \
  'source validation error:2' \
  'VALIDATION_KIND_MISMATCH_SOURCE' \
  'VALIDATION_KIND_MISMATCH_MANIFEST' \
  'run_expected_validation_error validation-kind-mismatch' \
  'source validation error:3'; do
  assert_script_contains "$SOURCE_SMOKE" "$required"
done

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-linux-stage0-source-smoke-test.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

STAGE0_DIR="$TMP_ROOT/stage0"
BIN_DIR="$TMP_ROOT/bin"
mkdir -p "$STAGE0_DIR/bin" "$BIN_DIR"

for executable in compiler transport-driver materializer; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$STAGE0_DIR/bin/$executable"
  chmod 0755 "$STAGE0_DIR/bin/$executable"
done

cat >"$STAGE0_DIR/manifest.json" <<JSON
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "x86_64-unknown-linux-gnu",
  "source_commit": "$SOURCE_COMMIT",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

cat >"$BIN_DIR/uname" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -s) printf 'Darwin\n' ;;
  -m) printf 'arm64\n' ;;
  *) exit 1 ;;
esac
SH
chmod 0755 "$BIN_DIR/uname"

cat >"$BIN_DIR/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  list)
    printf 'Running\n'
    ;;
  shell)
    if [[ "$*" == *"df -Pk /tmp"* ]]; then
      printf '10485760\n'
    fi
    ;;
  copy)
    ;;
  *)
    printf 'unexpected limactl invocation: %s\n' "$*" >&2
    exit 1
    ;;
esac
SH
chmod 0755 "$BIN_DIR/limactl"

run_smoke() {
  PATH="$BIN_DIR:$PATH" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_DIR="$STAGE0_DIR" \
    LSHARP_NATIVE_LINUX_X86_KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR=1 \
    "$SMOKE"
}

run_smoke

python3 - "$STAGE0_DIR/manifest.json" <<'PY'
import json
import sys

path = sys.argv[1]
manifest = json.load(open(path, encoding="utf-8"))
manifest["source_commit"] = "0" * 40
with open(path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle)
    handle.write("\n")
PY

expect_reject "stale stage0 source commit" run_smoke

echo "Linux native stage0 source-file provenance tests: OK"
