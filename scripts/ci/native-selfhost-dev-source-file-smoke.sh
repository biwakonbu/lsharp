#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"
VALIDATION_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-canonical-source.ls"
EXPECTED_VALIDATION_MANIFEST="$ROOT/tests/fixtures/validation/ec-m3-canonical-manifest.json"
VALIDATION_INVALID_SOURCE="$ROOT/tests/fixtures/validation/ec-m3-duplicate-node-source.ls"
STAGE0_DIR="${NATIVE_STAGE0_DIR:-}"
SOURCE_ROOT="${NATIVE_SELFHOST_SOURCE_ROOT:-$ROOT/selfhost}"
STAGE_DIR="${NATIVE_SELFHOST_STAGE_DIR:-}"
KEEP_STAGE_DIR="${NATIVE_SELFHOST_KEEP_STAGE_DIR:-0}"
WORK_DIR=""
STAGE_DIR_CREATED=0

cleanup() {
  [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
  if [[ "$STAGE_DIR_CREATED" -eq 1 && "$KEEP_STAGE_DIR" != "1" ]]; then
    rm -rf "$STAGE_DIR"
  fi
}
trap cleanup EXIT

die() {
  echo "ERROR: $*" >&2
  exit 1
}

require_file() {
  local path="$1"
  local description="$2"
  [[ -f "$path" && -s "$path" ]] || die "$description is required: $path"
}

[[ -n "$STAGE0_DIR" ]] || die "NATIVE_STAGE0_DIR is required"
[[ -d "$STAGE0_DIR" ]] || die "native stage0 directory is missing: $STAGE0_DIR"
[[ -d "$SOURCE_ROOT" ]] || die "native selfhost source root is missing: $SOURCE_ROOT"
[[ -x "$RUNNER" ]] || die "native selfhost runner is missing: $RUNNER"
require_file "$STAGE0_DIR/manifest.json" "native stage0 manifest"
require_file "$SOURCE_ROOT/src/App/Cli.ls" "native selfhost App.Cli source"
require_file "$VALIDATION_SOURCE" "EC-M3-01 validation source fixture"
require_file "$EXPECTED_VALIDATION_MANIFEST" "EC-M3-01 canonical manifest fixture"
require_file "$VALIDATION_INVALID_SOURCE" "EC-M3-01 duplicate node source fixture"

stage0_target="$(python3 - "$STAGE0_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"invalid native stage0 manifest: {error}")

if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("native stage0 manifest kind is invalid")

target = manifest.get("target")
if target not in ("aarch64-apple-darwin", "x86_64-unknown-linux-gnu"):
    raise SystemExit(f"native stage0 target is unsupported: {target!r}")
print(target)
PY
)"

case "$(uname -s)/$(uname -m)" in
  Darwin/arm64)
    expected_target="aarch64-apple-darwin"
    ;;
  Linux/x86_64)
    expected_target="x86_64-unknown-linux-gnu"
    ;;
  *)
    die "native selfhost source-file smoke requires macOS arm64 or Linux x86_64"
    ;;
esac
[[ "$stage0_target" == "$expected_target" ]] || die "stage0 target mismatch: expected $expected_target, got $stage0_target"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-source-smoke.XXXXXX")"
if [[ -z "$STAGE_DIR" ]]; then
  STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-stage.XXXXXX")"
  STAGE_DIR_CREATED=1
fi

BLOCKED_TOOL_DIR="$WORK_DIR/blocked-tools"
mkdir -p "$BLOCKED_TOOL_DIR"
for blocked_tool in cargo rustc lsharp; do
  printf '%s\n' '#!/usr/bin/env bash' 'exit 99' >"$BLOCKED_TOOL_DIR/$blocked_tool"
  chmod 755 "$BLOCKED_TOOL_DIR/$blocked_tool"
done

INPUT="$WORK_DIR/input.ls"
METADATA="$WORK_DIR/metadata.ls"
PROPERTY="$WORK_DIR/property.ls"
VACUOUS_PROPERTY="$WORK_DIR/vacuous-property.ls"
DYNAMIC_COMPLEMENT_PROPERTY="$WORK_DIR/dynamic-complement-property.ls"
COMPILE_OUTPUT="$WORK_DIR/compile.wasm"
BUILD_OUTPUT="$WORK_DIR/build.wasm"
VALIDATION_MANIFEST="$WORK_DIR/ec-m3-canonical-manifest.json"
VALIDATION_INVALID_MANIFEST="$WORK_DIR/ec-m3-duplicate-node-manifest.json"

printf '%s\n' '(defn main [] 42)' >"$INPUT"
cat >"$METADATA" <<'LSHARP'
(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
LSHARP
cat >"$PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))]
  x)
LSHARP
cat >"$VACUOUS_PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [sample Int] :cases 1 :postcondition (or true (= sample 0)))]
  x)
LSHARP
cat >"$DYNAMIC_COMPLEMENT_PROPERTY" <<'LSHARP'
(defn identity [x]
  :property [(for-all [value Int] :cases 1 :postcondition (or (= value 0) (not (= value 0))))]
  x)
LSHARP

run_command() {
  local label="$1"
  local bootstrap="$2"
  shift 2

  local status=0
  set +e
  if [[ "$bootstrap" == "1" ]]; then
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        --bootstrap \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  else
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  fi
  set -e

  if [[ "$status" -ne 0 ]]; then
    echo "ERROR: $label failed with exit=$status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit "$status"
  fi
  if [[ -s "$WORK_DIR/$label.stderr" ]]; then
    echo "ERROR: $label emitted stderr" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  fi
}

require_line() {
  local label="$1"
  local expected="$2"
  grep -Fx "$expected" "$WORK_DIR/$label.stdout" >/dev/null || {
    echo "ERROR: $label stdout is missing $expected" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  }
}

require_exact_output() {
  local label="$1"
  local expected="$2"
  if ! printf '%s' "$expected" | cmp -s - "$WORK_DIR/$label.stdout"; then
    echo "ERROR: $label stdout does not match expected output" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  fi
}

run_expected_failure() {
  local label="$1"
  local bootstrap="$2"
  shift 2

  local status=0
  set +e
  if [[ "$bootstrap" == "1" ]]; then
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        --bootstrap \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  else
    PATH="$BLOCKED_TOOL_DIR:$PATH" \
      "$RUNNER" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
    status=$?
  fi
  set -e

  [[ "$status" -eq 2 ]] || {
    echo "ERROR: $label expected exit 2, got $status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
  [[ ! -s "$WORK_DIR/$label.stderr" ]] || {
    echo "ERROR: $label emitted stderr" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
}

run_expected_validation_error() {
  local label="$1"
  shift

  local status=0
  set +e
  PATH="$BLOCKED_TOOL_DIR:$PATH" \
    "$RUNNER" \
      --stage0-dir "$STAGE0_DIR" \
      --source-root "$SOURCE_ROOT" \
      --stage-dir "$STAGE_DIR" \
      "$@" >"$WORK_DIR/$label.stdout" 2>"$WORK_DIR/$label.stderr"
  status=$?
  set -e

  [[ "$status" -eq 1 ]] || {
    echo "ERROR: $label expected exit 1, got $status" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    cat "$WORK_DIR/$label.stderr" >&2
    exit 1
  }
  [[ ! -s "$WORK_DIR/$label.stdout" ]] || {
    echo "ERROR: $label must not emit a report on validation error" >&2
    cat "$WORK_DIR/$label.stdout" >&2
    exit 1
  }
  [[ -s "$WORK_DIR/$label.stderr" ]] || {
    echo "ERROR: $label must emit a diagnostic on validation error" >&2
    exit 1
  }
}

run_command parse 1 parse "$INPUT"
for expected in decls:1 first-decl:defn first-body:int diagnostics:0; do
  require_line parse "$expected"
done

run_command check 0 check "$INPUT"
for expected in Int diagnostics:0; do
  require_line check "$expected"
done

run_command fmt 0 fmt "$INPUT"
require_line fmt '(defn main [] 42)'

run_command test 0 test "$INPUT"
require_exact_output test $'examples:0\ninvariants:0\nfailures:0\n'

run_command metadata-test 0 test "$METADATA"
require_exact_output metadata-test $'examples:2\ninvariants:1\nfailures:0\n'

run_command property-json 0 test "$PROPERTY" --format json
python3 - "$WORK_DIR/property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "pass":
    raise SystemExit(f"property JSON status is not pass: {report!r}")
if conformance["method"] != "sampled-property":
    raise SystemExit(f"property JSON method is invalid: {report!r}")
if conformance["cases"] != 5 or conformance["coverage"]["executed"] != 5:
    raise SystemExit(f"property JSON coverage is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"property JSON intent status is invalid: {report!r}")
PY

run_expected_failure vacuous-property-json 0 test "$VACUOUS_PROPERTY" --format json
python3 - "$WORK_DIR/vacuous-property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"vacuous property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "fail":
    raise SystemExit(f"vacuous property JSON status is not fail: {report!r}")
diagnostics = conformance["diagnostics"]
if diagnostics["count"] != 1 or diagnostics["firstErrorCode"] != 2005:
    raise SystemExit(f"vacuous property JSON diagnostic is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"vacuous property JSON intent status is invalid: {report!r}")
PY

run_expected_failure dynamic-complement-property-json 0 test "$DYNAMIC_COMPLEMENT_PROPERTY" --format json
python3 - "$WORK_DIR/dynamic-complement-property-json.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"dynamic complement property JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
conformance = report["implementation_conformance"]
if conformance["status"] != "fail":
    raise SystemExit(f"dynamic complement property JSON status is not fail: {report!r}")
diagnostics = conformance["diagnostics"]
if diagnostics["count"] != 1 or diagnostics["firstErrorCode"] != 2005:
    raise SystemExit(f"dynamic complement property JSON diagnostic is invalid: {report!r}")
if report["intent_validation"]["status"] != "unknown":
    raise SystemExit(f"dynamic complement property JSON intent status is invalid: {report!r}")
PY

run_expected_failure validation-manifest-unknown 0 validate \
  --source "$VALIDATION_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_MANIFEST"
cmp -s "$EXPECTED_VALIDATION_MANIFEST" "$VALIDATION_MANIFEST" \
  || die "EC-M3-01 native manifest bytes differ from the Rust canonical fixture"
python3 - "$WORK_DIR/validation-manifest-unknown.stdout" <<'PY'
import json
import pathlib
import sys

lines = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
if len(lines) != 1:
    raise SystemExit(f"validation JSON must contain one report line: {lines!r}")
report = json.loads(lines[0])
if report.get("status") != "unknown":
    raise SystemExit(f"validation status must be unknown: {report!r}")
if report.get("open_questions") != 1 or report.get("independent_reviews") != 0:
    raise SystemExit(f"validation unknown metrics are invalid: {report!r}")
if report.get("trace_gaps") != [] or report.get("contradicting_observations") != 0:
    raise SystemExit(f"validation trace/contradiction metrics are invalid: {report!r}")
PY

run_expected_validation_error validation-duplicate-node \
  validate \
  --source "$VALIDATION_INVALID_SOURCE" \
  --format json \
  --emit-manifest "$VALIDATION_INVALID_MANIFEST"
grep -F "source validation error" "$WORK_DIR/validation-duplicate-node.stderr" >/dev/null \
  || die "duplicate validation must expose a source validation error diagnostic"
grep -F "source validation error:2" "$WORK_DIR/validation-duplicate-node.stderr" >/dev/null \
  || die "duplicate validation diagnostic must expose the canonical duplicate-node code"
[[ ! -e "$VALIDATION_INVALID_MANIFEST" ]] \
  || die "duplicate validation must produce no report or manifest"

run_command compile 0 compile "$INPUT" -o "$COMPILE_OUTPUT"
run_command build 0 build "$INPUT" -o "$BUILD_OUTPUT"
for output in "$COMPILE_OUTPUT" "$BUILD_OUTPUT"; do
  [[ -s "$output" ]] || die "native command did not write output: $output"
  [[ "$(od -An -tx1 -N4 "$output" | tr -d '[:space:]')" == "0061736d" ]] \
    || die "native command did not write core Wasm bytes: $output"
done
grep -Eq '^wasm-size:[1-9][0-9]*$' "$WORK_DIR/compile.stdout" \
  || die "compile stdout is missing a positive wasm-size"
grep -Eq '^wasm-size:[1-9][0-9]*$' "$WORK_DIR/build.stdout" \
  || die "build stdout is missing a positive wasm-size"

printf '%s native selfhost source-file smoke passed\n' "$expected_target"
