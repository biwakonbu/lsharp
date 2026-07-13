#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACKAGE="$ROOT/scripts/ci/package-native-stage0.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$1', got '$2'"
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

expect_reject() {
  local label="$1"
  shift

  local output
  local status
  set +e
  output="$("$@" 2>&1)"
  status=$?
  set -e
  [[ "$status" -ne 0 ]] || fail "$label unexpectedly succeeded"
  [[ -n "$output" ]] || fail "$label did not report an error"
}

[[ -x "$PACKAGE" ]] || fail "native stage0 package builder is missing or not executable: $PACKAGE"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-stage0-package.XXXXXX")"
TMP_ROOT="$(cd "$TMP_ROOT" && pwd)"
trap 'rm -rf "$TMP_ROOT"' EXIT

INPUT_DIR="$TMP_ROOT/input"
OUTPUT_DIR="$TMP_ROOT/package"
HOST_BIN="$TMP_ROOT/host-bin"
HOST_TOOL_LOG="$TMP_ROOT/host-tools.log"
BUNDLE_LOG="$TMP_ROOT/bundle.log"
COMPILER="$INPUT_DIR/compiler.native"
TRANSPORT_DRIVER="$INPUT_DIR/transport-driver"
MATERIALIZER="$INPUT_DIR/materializer.py"
NON_EXECUTABLE_COMPILER="$INPUT_DIR/non-executable-compiler"
EMPTY_MATERIALIZER="$INPUT_DIR/empty-materializer.py"
TARGET="x86_64-unknown-linux-gnu"

mkdir -p "$INPUT_DIR" "$HOST_BIN"
: >"$HOST_TOOL_LOG"
: >"$BUNDLE_LOG"

cat >"$COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compiler|%s\n' "$*" >>"${NATIVE_STAGE0_PACKAGE_TEST_LOG:?}"
SH
chmod +x "$COMPILER"

cat >"$TRANSPORT_DRIVER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'transport-driver|%s\n' "$*" >>"${NATIVE_STAGE0_PACKAGE_TEST_LOG:?}"
SH
chmod +x "$TRANSPORT_DRIVER"

cat >"$MATERIALIZER" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_STAGE0_PACKAGE_TEST_LOG"])
with log_path.open("a", encoding="utf-8") as log:
    log.write(f"materializer|bundled-script|{' '.join(sys.argv[1:])}\n")
PY
chmod 0644 "$MATERIALIZER"

cat >"$NON_EXECUTABLE_COMPILER" <<'SH'
#!/usr/bin/env bash
exit 0
SH
chmod 0644 "$NON_EXECUTABLE_COMPILER"
: >"$EMPTY_MATERIALIZER"

for host_tool in cargo lsharp curl wget; do
  cat >"$HOST_BIN/$host_tool" <<'SH'
#!/usr/bin/env bash
printf 'host-tool|%s\n' "$(basename "$0")" >>"${NATIVE_STAGE0_PACKAGE_TEST_LOG:?}"
exit 99
SH
  chmod +x "$HOST_BIN/$host_tool"
done

run_package() {
  PATH="$HOST_BIN:$PATH" "$PACKAGE" "$@"
}

expect_reject "missing target" run_package \
  --compiler "$COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$MATERIALIZER" \
  --output-dir "$TMP_ROOT/missing-target"

expect_reject "unsupported target" run_package \
  --target "x86_64-apple-darwin" \
  --compiler "$COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$MATERIALIZER" \
  --output-dir "$TMP_ROOT/unsupported-target"

expect_reject "non-executable compiler" run_package \
  --target "$TARGET" \
  --compiler "$NON_EXECUTABLE_COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$MATERIALIZER" \
  --output-dir "$TMP_ROOT/non-executable-compiler"

expect_reject "missing transport driver" run_package \
  --target "$TARGET" \
  --compiler "$COMPILER" \
  --transport-driver "$INPUT_DIR/missing-transport-driver" \
  --materializer "$MATERIALIZER" \
  --output-dir "$TMP_ROOT/missing-transport-driver"

expect_reject "empty materializer" run_package \
  --target "$TARGET" \
  --compiler "$COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$EMPTY_MATERIALIZER" \
  --output-dir "$TMP_ROOT/empty-materializer"

export NATIVE_STAGE0_PACKAGE_TEST_LOG="$HOST_TOOL_LOG"
run_package \
  --target "$TARGET" \
  --compiler "$COMPILER" \
  --transport-driver "$TRANSPORT_DRIVER" \
  --materializer "$MATERIALIZER" \
  --output-dir "$OUTPUT_DIR"

assert_eq "" "$(cat "$HOST_TOOL_LOG")"

python3 - "$OUTPUT_DIR/manifest.json" "$TARGET" <<'PY'
import json
import os
import sys

manifest_path = sys.argv[1]
target = sys.argv[2]
with open(manifest_path, encoding="utf-8") as source:
    manifest = json.load(source)

expected = {
    "kind": "lsharp-native-selfhost-stage0",
    "target": target,
    "compiler": "bin/compiler",
    "transport_driver": "bin/transport-driver",
    "materializer": "bin/materializer",
}
if manifest != expected:
    raise SystemExit(f"unexpected manifest: {manifest!r}")

for field in ("compiler", "transport_driver", "materializer"):
    value = manifest[field]
    parts = value.split("/")
    if os.path.isabs(value) or any(part in ("", ".", "..") for part in parts):
        raise SystemExit(f"unsafe manifest path for {field}: {value}")
PY

for executable in \
  "$OUTPUT_DIR/bin/compiler" \
  "$OUTPUT_DIR/bin/transport-driver" \
  "$OUTPUT_DIR/bin/materializer"; do
  [[ -x "$executable" ]] || fail "packaged executable is missing or not executable: $executable"
done
[[ -s "$OUTPUT_DIR/bin/materializer.py" ]] || fail "packaged materializer script is missing"
[[ ! -x "$OUTPUT_DIR/bin/materializer.py" ]] || fail "packaged materializer script unexpectedly became executable"
cmp -s "$MATERIALIZER" "$OUTPUT_DIR/bin/materializer.py" \
  || fail "packaged materializer script does not match the input"
assert_file_contains "$OUTPUT_DIR/bin/materializer" "materializer.py"
! grep -F -- "$MATERIALIZER" "$OUTPUT_DIR/bin/materializer" >/dev/null \
  || fail "materializer wrapper refers to the source script"

assert_eq "bin/compiler
bin/materializer
bin/materializer.py
bin/transport-driver
manifest.json" "$(cd "$OUTPUT_DIR" && find . -type f -print | sed 's#^./##' | LC_ALL=C sort)"
[[ ! -e "$OUTPUT_DIR/lsharp" ]] || fail "package unexpectedly contains a Rust host launcher"
[[ ! -e "$OUTPUT_DIR/bin/lsharp" ]] || fail "package unexpectedly contains a Rust host launcher"
[[ ! -e "$OUTPUT_DIR/bin/cargo" ]] || fail "package unexpectedly contains Cargo"

NATIVE_STAGE0_PACKAGE_TEST_LOG="$BUNDLE_LOG" "$OUTPUT_DIR/bin/compiler" compiler-probe
NATIVE_STAGE0_PACKAGE_TEST_LOG="$BUNDLE_LOG" "$OUTPUT_DIR/bin/transport-driver" transport-probe
rm "$MATERIALIZER"
NATIVE_STAGE0_PACKAGE_TEST_LOG="$BUNDLE_LOG" "$OUTPUT_DIR/bin/materializer" materializer-probe

assert_file_contains "$BUNDLE_LOG" "compiler|compiler-probe"
assert_file_contains "$BUNDLE_LOG" "transport-driver|transport-probe"
assert_file_contains "$BUNDLE_LOG" "materializer|bundled-script|materializer-probe"

echo "native stage0 package tests: OK"
