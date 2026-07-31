#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PRODUCER="$ROOT/scripts/ci/native-macos-aarch64-stage0-release.sh"
CHAIN_SOURCE="$ROOT/crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-macos-stage0-release.XXXXXX")"
FAKE_ROOT="$TMP_ROOT/project"
PATH_PREFIX="$TMP_ROOT/bin"
LOG_PATH="$TMP_ROOT/invocations.log"
SOURCE_COMMIT="0123456789abcdef0123456789abcdef01234567"
OUTPUT_PARENT="$(mktemp -d /tmp/lsharp-native-macos-stage0-release-output.XXXXXX)"
OUTPUT_DIR="$OUTPUT_PARENT/stage0"
trap 'rm -rf "$TMP_ROOT" "$OUTPUT_PARENT"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

mkdir -p "$FAKE_ROOT/scripts/ci" "$PATH_PREFIX"
cp "$PRODUCER" "$FAKE_ROOT/scripts/ci/"

python3 - "$CHAIN_SOURCE" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
marker = "write_actual_macos_aarch64_stage0_compiler_artifact("
start = source.rindex(marker)
call_end = source.index(")?;", start) + 3
call = source[start:call_end]
if "&stage3_input" not in call:
    raise SystemExit("stage0 compiler artifact must use the fixed-point stage3 compiler input")
if "&app_cli_input" in call:
    raise SystemExit("stage0 compiler artifact must not use the App.Cli launcher input")
PY

grep -F 'LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR' "$PRODUCER" >/dev/null \
  || fail "Mac producer must request a dedicated stage0 compiler artifact"
grep -F 'compiler.native' "$PRODUCER" >/dev/null \
  || fail "Mac producer must package compiler.native rather than the App.Cli launcher"

cat >"$PATH_PREFIX/uname" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -s) printf '%s\n' 'Darwin' ;;
  -m) printf '%s\n' 'arm64' ;;
  *) exit 1 ;;
esac
SH
chmod +x "$PATH_PREFIX/uname"

cat >"$PATH_PREFIX/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" == 'rev-parse --verify HEAD' || "$*" == 'rev-parse HEAD' ]]; then
  printf '%s\n' '0123456789abcdef0123456789abcdef01234567'
  exit 0
fi
echo "unexpected git invocation: $*" >&2
exit 1
SH
chmod +x "$PATH_PREFIX/git"

cat >"$FAKE_ROOT/scripts/ci/native-macos-aarch64-selfhost-release.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "app-cli artifact=${LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR:-} stage0-compiler=${LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR:-} target=${LSHARP_NATIVE_MACOS_AARCH64_CARGO_TARGET_DIR:-}" >>"$FAKE_LOG"
mkdir -p "$LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR"
printf '%s\n' 'fake compiler' >"$LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR/program.native"
chmod +x "$LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR/program.native"
program_sha256="$(shasum -a 256 "$LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR/program.native" | awk '{print $1}')"
printf '%s\n' "{\"status\":\"pass\",\"artifact_kind\":\"native App.Cli release program\",\"target\":\"aarch64-apple-darwin\",\"entry_module\":\"App.Cli\",\"source\":\"src/App/Cli.ls\",\"source_commit\":\"0123456789abcdef0123456789abcdef01234567\",\"selfhost_fixed_point\":true,\"program_sha256\":\"$program_sha256\"}" >"$LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR/manifest.json"
mkdir -p "$LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR"
printf '%s\n' 'fake stage0 compiler' >"$LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR/compiler.native"
chmod +x "$LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR/compiler.native"
compiler_sha256="$(shasum -a 256 "$LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR/compiler.native" | awk '{print $1}')"
printf '%s\n' "{\"status\":\"pass\",\"artifact_kind\":\"native stage0 compiler\",\"target\":\"aarch64-apple-darwin\",\"entry_module\":\"App.Cli\",\"source\":\"src/App/Cli.ls\",\"source_commit\":\"0123456789abcdef0123456789abcdef01234567\",\"compiler_sha256\":\"$compiler_sha256\"}" >"$LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR/manifest.json"
SH
chmod +x "$FAKE_ROOT/scripts/ci/native-macos-aarch64-selfhost-release.sh"

cat >"$FAKE_ROOT/scripts/ci/package-native-stage0.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
target=''
source_commit=''
compiler=''
transport_driver=''
materializer=''
output_dir=''
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) target="$2"; shift 2 ;;
    --source-commit) source_commit="$2"; shift 2 ;;
    --compiler) compiler="$2"; shift 2 ;;
    --transport-driver) transport_driver="$2"; shift 2 ;;
    --materializer) materializer="$2"; shift 2 ;;
    --output-dir) output_dir="$2"; shift 2 ;;
    *) echo "unexpected package argument: $1" >&2; exit 1 ;;
  esac
done
printf '%s\n' "package target=$target source=$source_commit compiler=$compiler transport=$transport_driver materializer=$materializer output=$output_dir" >>"$FAKE_LOG"
mkdir -p "$output_dir/bin"
printf '%s\n' '{"kind":"lsharp-native-selfhost-stage0","target":"aarch64-apple-darwin","source_commit":"0123456789abcdef0123456789abcdef01234567","compiler":"bin/compiler","transport_driver":"bin/transport-driver","materializer":"bin/materializer"}' >"$output_dir/manifest.json"
for file in compiler transport-driver materializer; do
  printf '%s\n' "fake $file" >"$output_dir/bin/$file"
  chmod +x "$output_dir/bin/$file"
done
SH
chmod +x "$FAKE_ROOT/scripts/ci/package-native-stage0.sh"

FAKE_LOG="$LOG_PATH" \
PATH="$PATH_PREFIX:$PATH" \
LSHARP_NATIVE_MACOS_AARCH64_STAGE0_DIR="$OUTPUT_DIR" \
  bash "$FAKE_ROOT/scripts/ci/native-macos-aarch64-stage0-release.sh"

grep -F "app-cli artifact=" "$LOG_PATH" >/dev/null || fail "App.Cli producer was not invoked"
grep -E 'stage0-compiler=.*/stage0-compiler' "$LOG_PATH" >/dev/null \
  || fail "dedicated stage0 compiler artifact directory was not forwarded"
grep -F "package target=aarch64-apple-darwin source=$SOURCE_COMMIT" "$LOG_PATH" >/dev/null \
  || fail "stage0 package provenance was not forwarded"
grep -E 'compiler=.*/stage0-compiler/compiler\.native' "$LOG_PATH" >/dev/null \
  || fail "stage0 package did not receive the dedicated compiler input"
grep -E 'transport=.*/project/scripts/ci/native-stage0-transport-macos-aarch64\.sh' "$LOG_PATH" >/dev/null \
  || fail "Mac transport driver was not selected"
grep -E 'materializer=.*/project/scripts/ci/materialize-native-macos-aarch64-bundle\.py' "$LOG_PATH" >/dev/null \
  || fail "Mac materializer was not selected"
app_artifact_dir="$(sed -n 's/^app-cli artifact=\([^ ]*\) stage0-compiler=.*/\1/p' "$LOG_PATH")"
[[ -n "$app_artifact_dir" && ! -e "$app_artifact_dir" ]] \
  || fail "producer temporary App.Cli artifact directory was not cleaned up"
stage0_compiler_artifact_dir="$(sed -n 's/^app-cli artifact=[^ ]* stage0-compiler=\([^ ]*\) target=.*/\1/p' "$LOG_PATH")"
[[ -n "$stage0_compiler_artifact_dir" && ! -e "$stage0_compiler_artifact_dir" ]] \
  || fail "producer temporary stage0 compiler artifact directory was not cleaned up"

python3 - "$OUTPUT_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["kind"] == "lsharp-native-selfhost-stage0"
assert manifest["target"] == "aarch64-apple-darwin"
assert manifest["source_commit"] == "0123456789abcdef0123456789abcdef01234567"
PY

set +e
stale_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  LSHARP_NATIVE_MACOS_AARCH64_STAGE0_DIR="$OUTPUT_DIR" \
    bash "$FAKE_ROOT/scripts/ci/native-macos-aarch64-stage0-release.sh" 2>&1
)"
stale_status=$?
set -e
[[ "$stale_status" -ne 0 ]] || fail "existing stage0 output was overwritten"
grep -F 'output directory already exists' <<<"$stale_output" >/dev/null \
  || fail "existing stage0 output rejection was not explicit"

set +e
unsafe_output="$(
  PATH="$PATH_PREFIX:$PATH" \
  LSHARP_NATIVE_MACOS_AARCH64_STAGE0_DIR="$TMP_ROOT/unsafe-stage0" \
    bash "$FAKE_ROOT/scripts/ci/native-macos-aarch64-stage0-release.sh" 2>&1
)"
unsafe_status=$?
set -e
[[ "$unsafe_status" -ne 0 ]] || fail "unsafe stage0 output path was accepted"
grep -F 'stage0 output must be under' <<<"$unsafe_output" >/dev/null \
  || fail "unsafe stage0 output rejection was not explicit"

echo 'native Mac aarch64 stage0 release tests: OK'
