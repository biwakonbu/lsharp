#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"
INSTALLER="$ROOT/scripts/native-selfhost-install.py"
DECODER="$ROOT/scripts/ci/decode-native-selfhost-transport.py"

# shellcheck source=../lib/source-fingerprint.sh
source "$ROOT/scripts/lib/source-fingerprint.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-install-runner.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

TEST_ROOT="$TMP_ROOT/repo"
STAGE0_DIR="$TMP_ROOT/stage0"
SOURCE_ROOT="$TEST_ROOT/selfhost"
STAGE_DIR="$TMP_ROOT/stage"
PROJECT_DIR="$TEST_ROOT/project"
DEPENDENCY_DIR="$TEST_ROOT/geometry"
HOST_BIN="$TMP_ROOT/host-bin"
HOST_COMMAND_LOG="$TMP_ROOT/host-command.log"

mkdir -p "$TEST_ROOT/scripts/ci" "$TEST_ROOT/scripts/lib" "$STAGE0_DIR/bin" "$SOURCE_ROOT/src/App" \
  "$PROJECT_DIR" "$DEPENDENCY_DIR/src" "$HOST_BIN"
# runner は自身の位置から ROOT を解決するので、fixture 側にも共有ライブラリが要る。
cp "$ROOT/scripts/lib/source-fingerprint.sh" "$TEST_ROOT/scripts/lib/source-fingerprint.sh"
cp "$RUNNER" "$TEST_ROOT/scripts/native-selfhost-dev.sh"
cp "$INSTALLER" "$TEST_ROOT/scripts/native-selfhost-install.py"
cp "$DECODER" "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"
chmod +x "$TEST_ROOT/scripts/native-selfhost-dev.sh" \
  "$TEST_ROOT/scripts/native-selfhost-install.py" \
  "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"

cat >"$STAGE0_DIR/manifest.json" <<'JSON'
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "x86_64-unknown-linux-gnu",
  "source_commit": "0000000000000000000000000000000000000000",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

cat >"$STAGE0_DIR/bin/compiler" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
exit 0
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
[[ -f "$source_root/$entry" ]] || exit 92
"$compiler" "$source_root" "$entry"
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
[[ $# -eq 3 ]] || exit 93
stage_dir="$1"
[[ -s "$2" && -f "$3" ]] || exit 94
[[ "${LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0:-}" == "1" ]] || exit 95
cat >"$stage_dir/program.native" <<'PROGRAM'
#!/usr/bin/env bash
set -euo pipefail
exit 0
PROGRAM
chmod +x "$stage_dir/program.native"
SH
chmod +x "$STAGE0_DIR/bin/materializer"

cat >"$SOURCE_ROOT/src/App/Cli.ls" <<'LS'
(module App.Cli)
LS

cat >"$DEPENDENCY_DIR/lsharp.toml" <<'TOML'
[project]
name = "geometry"
version = "1.4.0"

[project.exports]
modules = ["Geometry"]
TOML
printf '%s\n' '(module Geometry)' >"$DEPENDENCY_DIR/src/Geometry.ls"

cat >"$PROJECT_DIR/lsharp.toml" <<'TOML'
[dependencies.geometry]
path = "../geometry"
TOML

for command_name in cargo lsharp; do
  cat >"$HOST_BIN/$command_name" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$0" >>"$HOST_COMMAND_LOG"
exit 97
SH
  chmod +x "$HOST_BIN/$command_name"
done

(
  cd "$TEST_ROOT"
  git init -q
  git add scripts selfhost project/lsharp.toml
  git -c user.name='L# native install runner test' \
    -c user.email='lsharp-native-install-runner@example.invalid' \
    commit -qm 'native install runner fixture'
)
CURRENT_SOURCE_COMMIT="$(cd "$TEST_ROOT" && git rev-parse HEAD)"
# strict lane は source_commit と selfhost source fingerprint の両方一致を要求する。
# fixture の実 source から算出して manifest に載せる。
CURRENT_SOURCE_FINGERPRINT="$(lsharp_source_fingerprint "$SOURCE_ROOT/src")"
python3 - "$STAGE0_DIR/manifest.json" "$CURRENT_SOURCE_COMMIT" "$CURRENT_SOURCE_FINGERPRINT" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
manifest = json.loads(path.read_text(encoding="utf-8"))
manifest["source_commit"] = sys.argv[2]
manifest["selfhost_src_fingerprint"] = sys.argv[3]
path.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
PY

(
  cd "$PROJECT_DIR"
  PATH="$HOST_BIN:$PATH" \
    HOST_COMMAND_LOG="$HOST_COMMAND_LOG" \
    "$TEST_ROOT/scripts/native-selfhost-dev.sh" \
      --stage0-dir "$STAGE0_DIR" \
      --source-root "$SOURCE_ROOT" \
      --stage-dir "$STAGE_DIR" \
      install >"$TMP_ROOT/stdout" 2>"$TMP_ROOT/stderr"
)

grep -F 'installed 1 dependency entries' "$TMP_ROOT/stdout" >/dev/null \
  || fail "native install runner did not report the installed dependency"
[[ ! -s "$HOST_COMMAND_LOG" ]] || fail "native install invoked cargo/lsharp host fallback"

installed="$(find "$PROJECT_DIR/.lsharp/packages" -maxdepth 1 -type l -name 'geometry-*' -print -quit)"
[[ -n "$installed" ]] || fail "native install runner did not create the path package symlink"
[[ "$(realpath "$installed")" == "$(realpath "$DEPENDENCY_DIR")" ]] \
  || fail "installed path package does not point to the dependency source"
grep -F 'name = "geometry"' "$PROJECT_DIR/.lsharp/lock.toml" >/dev/null \
  || fail "native install runner did not write lock.toml"
grep -F '.lsharp/packages/' "$PROJECT_DIR/.lsharp/module-index/Geometry.path" >/dev/null \
  || fail "native install runner did not write the exported module index"

echo "native selfhost install runner tests: OK"
