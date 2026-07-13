#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"

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

assert_file_not_contains() {
  local path="$1"
  local unexpected="$2"
  ! grep -F -- "$unexpected" "$path" >/dev/null || fail "$path unexpectedly contains: $unexpected"
}

[[ -x "$RUNNER" ]] || fail "native selfhost dev runner is missing or not executable: $RUNNER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-dev.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

TEST_ROOT="$TMP_ROOT/repo"
STAGE0_DIR="$TMP_ROOT/stage0"
SOURCE_ROOT="$TMP_ROOT/source"
STAGE_DIR="$TMP_ROOT/stage"
LOG_FILE="$TMP_ROOT/invocations.log"
HOST_BIN="$TMP_ROOT/host-bin"

mkdir -p "$TEST_ROOT/scripts/ci" "$STAGE0_DIR/bin" "$SOURCE_ROOT/src/App" "$HOST_BIN"
PROJECT_DIR="$(cd "$TEST_ROOT" && pwd)"
cp "$RUNNER" "$TEST_ROOT/scripts/native-selfhost-dev.sh"
chmod +x "$TEST_ROOT/scripts/native-selfhost-dev.sh"
cp "$ROOT/scripts/ci/decode-native-selfhost-transport.py" \
  "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"
chmod +x "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"

cat >"$TEST_ROOT/scripts/native-selfhost-lsp-stdio.py" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_TEST_LOG"])
args = sys.argv[1:]
if len(args) < 3 or args[0] != "--program" or args[2] != "--":
    raise SystemExit(102)
if not pathlib.Path(args[1]).is_file():
    raise SystemExit(103)
with log_path.open("a", encoding="ascii") as log:
    log.write("lsp-shim|" + " ".join(args[3:]) + "\\n")
PY

cat >"$TEST_ROOT/scripts/native-selfhost-install.py" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_TEST_LOG"])
args = sys.argv[1:]
expected = ["--project-dir", os.path.normpath(os.environ["NATIVE_TEST_PROJECT_DIR"])]
if args != expected:
    raise SystemExit(f"unexpected install arguments: {args!r}; expected {expected!r}")
with log_path.open("a", encoding="ascii") as log:
    log.write("install-helper|" + " ".join(args) + "\n")
PY

cat >"$TEST_ROOT/scripts/native-selfhost-repl.py" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_TEST_LOG"])
args = sys.argv[1:]
if len(args) < 2 or args[0] != "--program":
    raise SystemExit(105)
if not pathlib.Path(args[1]).is_file():
    raise SystemExit(106)
with log_path.open("a", encoding="ascii") as log:
    log.write("repl-helper|" + " ".join(args[2:]) + "\n")
PY

cat >"$TEST_ROOT/scripts/native-selfhost-doc.py" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_TEST_LOG"])
args = sys.argv[1:]
if len(args) != 4 or args[0] != "--program":
    raise SystemExit(107)
if not pathlib.Path(args[1]).is_file():
    raise SystemExit(108)
if args[2] != os.environ["NATIVE_TEST_DOC_SOURCE"] or args[3] != "--json":
    raise SystemExit(109)
with log_path.open("a", encoding="ascii") as log:
    log.write("doc-helper|" + " ".join(args[2:]) + "\n")
PY

cat >"$TEST_ROOT/scripts/native-selfhost-component.py" <<'PY'
import os
import pathlib
import sys

log_path = pathlib.Path(os.environ["NATIVE_TEST_LOG"])
args = sys.argv[1:]
if len(args) != 8 or args[0] != "--program" or args[2] != "--command":
    raise SystemExit(110)
if not pathlib.Path(args[1]).is_file():
    raise SystemExit(111)
if args[3] not in ("compile", "build") or args[4] != "--source" or args[6] != "--output":
    raise SystemExit(112)
if args[5] != os.environ["NATIVE_TEST_DOC_SOURCE"]:
    raise SystemExit(113)
pathlib.Path(args[7]).write_bytes(b"component")
with log_path.open("a", encoding="ascii") as log:
    log.write("component-helper|" + " ".join(args[3:]) + "\n")
PY

cat >"$STAGE0_DIR/manifest.json" <<'JSON'
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "x86_64-unknown-linux-gnu",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

cat >"$STAGE0_DIR/bin/compiler" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compiler|%s\n' "$*" >>"$NATIVE_TEST_LOG"
SH
chmod +x "$STAGE0_DIR/bin/compiler"

cat >"$STAGE0_DIR/bin/transport-driver" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ $# -eq 4 ]] || exit 91
compiler="$1"
source_root="$2"
entry="$3"
transport_output="$4"
[[ "$source_root" == */source ]] || exit 92
[[ -f "$source_root/$entry" ]] || exit 93
"$compiler" "$source_root" "$entry"
printf 'transport|%s|%s\n' "$source_root" "$entry" >>"$NATIVE_TEST_LOG"
cat >"$transport_output" <<'TRANSPORT'
9000000005
0
10
0
9000000006
9000000001
1
9000000002
0
9000000003
0
9000000004
TRANSPORT
SH
chmod +x "$STAGE0_DIR/bin/transport-driver"

cat >"$STAGE0_DIR/bin/materializer" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ $# -eq 3 ]] || exit 94
stage_dir="$1"
[[ -s "$2" ]] || exit 95
[[ -f "$3" ]] || exit 96
case "${NATIVE_TEST_EXPECTED_TARGET:-x86_64-unknown-linux-gnu}" in
  x86_64-unknown-linux-gnu)
    [[ "${LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0:-}" == "1" ]] || exit 99
    ;;
  aarch64-apple-darwin)
    [[ "${LSHARP_NATIVE_MACOS_AARCH64_SKIP_ARGV0:-}" == "1" ]] || exit 100
    ;;
  *)
    exit 101
    ;;
esac
printf 'materializer|%s\n' "$stage_dir" >>"$NATIVE_TEST_LOG"
cat >"$stage_dir/program.native" <<'PROGRAM'
#!/usr/bin/env bash
set -euo pipefail
[[ -z "${LSHARP_PATH+x}" ]] || exit 97
[[ -z "${LSHARP_DISABLE_EMBEDDED_COMPONENT+x}" ]] || exit 98
printf 'program|%s\n' "$*" >>"$NATIVE_TEST_LOG"
PROGRAM
chmod +x "$stage_dir/program.native"
SH
chmod +x "$STAGE0_DIR/bin/materializer"

cat >"$SOURCE_ROOT/src/App/Cli.ls" <<'LS'
(module App.Cli)
LS

DOC_INPUT="$TEST_ROOT/doc-input.ls"
DEFAULT_COMPONENT_OUTPUT="$PROJECT_DIR/doc-input.component.wasm"
COMPONENT_OUTPUT="$TEST_ROOT/compile.component.wasm"
BUILD_COMPONENT_OUTPUT="$TEST_ROOT/build.component.wasm"
PREVIEW1_OUTPUT="$TEST_ROOT/compile-preview1.wasm"
FORCED_PREVIEW1_OUTPUT="$TEST_ROOT/compile-forced-preview1.component.wasm"
printf '%s\n' '(defn main [] 42)' >"$DOC_INPUT"

cat >"$HOST_BIN/cargo" <<'SH'
#!/usr/bin/env bash
printf 'host-cargo\n' >>"$NATIVE_TEST_LOG"
exit 99
SH
chmod +x "$HOST_BIN/cargo"

cat >"$HOST_BIN/lsharp" <<'SH'
#!/usr/bin/env bash
printf 'host-lsharp\n' >>"$NATIVE_TEST_LOG"
exit 99
SH
chmod +x "$HOST_BIN/lsharp"

run_runner() {
  (
    cd "$TEST_ROOT"
    NATIVE_TEST_LOG="$LOG_FILE" \
      NATIVE_TEST_PROJECT_DIR="$PROJECT_DIR" \
      NATIVE_TEST_DOC_SOURCE="$DOC_INPUT" \
      LSHARP_PATH="$HOST_BIN/lsharp" \
      LSHARP_DISABLE_EMBEDDED_COMPONENT=1 \
      PATH="$HOST_BIN:$PATH" \
      "$TEST_ROOT/scripts/native-selfhost-dev.sh" \
        --stage0-dir "$STAGE0_DIR" \
        --source-root "$SOURCE_ROOT" \
        --stage-dir "$STAGE_DIR" \
        "$@"
  )
}

run_runner alpha beta
assert_eq "1" "$(grep -c '^transport|' "$LOG_FILE")"
assert_eq "1" "$(grep -c '^materializer|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|alpha beta"
assert_file_not_contains "$LOG_FILE" "host-cargo"
assert_file_not_contains "$LOG_FILE" "host-lsharp"

run_runner reuse
assert_eq "1" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|reuse"

printf '\n;; source refresh\n' >>"$SOURCE_ROOT/src/App/Cli.ls"
run_runner changed
assert_eq "2" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|changed"

run_runner --bootstrap bootstrap
assert_eq "3" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|bootstrap"

python3 - "$STAGE0_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
manifest = json.loads(path.read_text())
manifest["target"] = "aarch64-apple-darwin"
path.write_text(json.dumps(manifest) + "\n")
PY
rm -rf "$STAGE_DIR"
NATIVE_TEST_EXPECTED_TARGET=aarch64-apple-darwin run_runner mac
assert_file_contains "$STAGE_DIR/manifest.json" '"target": "aarch64-apple-darwin"'

run_runner lsp --stdio relay
assert_file_contains "$LOG_FILE" "lsp-shim|relay"
assert_file_not_contains "$LOG_FILE" "program|lsp --stdio relay"

if run_runner lsp >"$TMP_ROOT/lsp.stdout" 2>"$TMP_ROOT/lsp.stderr"; then
  fail "native runner accepted bare lsp without stdio transport"
fi
assert_file_contains "$TMP_ROOT/lsp.stderr" "error: native selfhost runner supports lsp only with --stdio"
assert_file_not_contains "$LOG_FILE" "program|lsp"

if run_runner mcp-server >"$TMP_ROOT/mcp-server.stdout" 2>"$TMP_ROOT/mcp-server.stderr"; then
  fail "native runner accepted Rust-only mcp-server"
fi
assert_file_contains "$TMP_ROOT/mcp-server.stderr" "error: native selfhost runner does not provide mcp-server"
assert_file_not_contains "$LOG_FILE" "program|mcp-server"

run_runner install
assert_file_contains "$LOG_FILE" "install-helper|--project-dir $PROJECT_DIR"
assert_file_not_contains "$LOG_FILE" "program|install"

run_runner repl --stdin
assert_file_contains "$LOG_FILE" "repl-helper|--stdin"
assert_file_not_contains "$LOG_FILE" "program|repl --stdin"

run_runner doc "$DOC_INPUT" --json
assert_file_contains "$LOG_FILE" "doc-helper|$DOC_INPUT --json"
assert_file_not_contains "$LOG_FILE" "program|doc $DOC_INPUT --json"

run_runner parse "$DOC_INPUT"
assert_file_contains "$LOG_FILE" "program|parse $DOC_INPUT"
assert_file_not_contains "$LOG_FILE" "component-helper|parse --source $DOC_INPUT"

run_runner compile "$DOC_INPUT"
assert_file_contains "$LOG_FILE" "component-helper|compile --source $DOC_INPUT --output $DEFAULT_COMPONENT_OUTPUT"
assert_eq "component" "$(<"$DEFAULT_COMPONENT_OUTPUT")"
assert_file_not_contains "$LOG_FILE" "program|compile $DOC_INPUT"

run_runner compile "$DOC_INPUT" -o "$PREVIEW1_OUTPUT"
assert_file_contains "$LOG_FILE" "program|compile $DOC_INPUT -o $PREVIEW1_OUTPUT"
assert_file_not_contains "$LOG_FILE" "component-helper|compile --source $DOC_INPUT --output $PREVIEW1_OUTPUT"

run_runner compile "$DOC_INPUT" -o "$FORCED_PREVIEW1_OUTPUT" --target wasi-preview1
assert_file_contains "$LOG_FILE" "program|compile $DOC_INPUT -o $FORCED_PREVIEW1_OUTPUT --target wasi-preview1"
assert_file_not_contains "$LOG_FILE" "component-helper|compile --source $DOC_INPUT --output $FORCED_PREVIEW1_OUTPUT"

run_runner compile "$DOC_INPUT" -o "$COMPONENT_OUTPUT" --target wasi-component
assert_file_contains "$LOG_FILE" "component-helper|compile --source $DOC_INPUT --output $COMPONENT_OUTPUT"
assert_eq "component" "$(<"$COMPONENT_OUTPUT")"
assert_file_not_contains "$LOG_FILE" "program|compile $DOC_INPUT -o $COMPONENT_OUTPUT --target wasi-component"

run_runner build "$DOC_INPUT" --output "$BUILD_COMPONENT_OUTPUT" --target wasm
assert_file_contains "$LOG_FILE" "component-helper|build --source $DOC_INPUT --output $BUILD_COMPONENT_OUTPUT"
assert_eq "component" "$(<"$BUILD_COMPONENT_OUTPUT")"
assert_file_not_contains "$LOG_FILE" "program|build $DOC_INPUT --output $BUILD_COMPONENT_OUTPUT --target wasm"

if run_runner compile "$DOC_INPUT" --target web-wasm >"$TMP_ROOT/web-wasm.stdout" 2>"$TMP_ROOT/web-wasm.stderr"; then
  fail "native runner accepted unsupported web-wasm target"
fi
assert_file_contains "$TMP_ROOT/web-wasm.stderr" "error: native selfhost runner does not support --target web-wasm"
assert_file_not_contains "$LOG_FILE" "program|compile $DOC_INPUT --target web-wasm"

if run_runner build "$DOC_INPUT" --target native >"$TMP_ROOT/native.stdout" 2>"$TMP_ROOT/native.stderr"; then
  fail "native runner accepted unsupported native target"
fi
assert_file_contains "$TMP_ROOT/native.stderr" "error: native selfhost runner does not support --target native"
assert_file_not_contains "$LOG_FILE" "program|build $DOC_INPUT --target native"

if run_runner compile "$DOC_INPUT" --emit-ir >"$TMP_ROOT/emit-ir.stdout" 2>"$TMP_ROOT/emit-ir.stderr"; then
  fail "native runner accepted Rust-only --emit-ir"
fi
assert_file_contains "$TMP_ROOT/emit-ir.stderr" "error: native selfhost runner does not support --emit-ir"
assert_file_not_contains "$LOG_FILE" "program|compile $DOC_INPUT --emit-ir"

assert_file_not_contains "$RUNNER" 'cargo '
assert_file_not_contains "$RUNNER" 'command -v lsharp'
assert_file_not_contains "$RUNNER" 'which lsharp'
assert_file_not_contains "$RUNNER" '"$ROOT/scripts/selfhost-dev.sh"'
assert_file_not_contains "$RUNNER" '"$ROOT/scripts/bootstrap.sh"'

echo "native selfhost dev runner tests: OK"
