#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"
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
COMPILE_OUTPUT="$WORK_DIR/compile.wasm"
BUILD_OUTPUT="$WORK_DIR/build.wasm"

printf '%s\n' '(defn main [] 42)' >"$INPUT"
cat >"$METADATA" <<'LSHARP'
(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
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
