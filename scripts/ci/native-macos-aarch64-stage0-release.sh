#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="aarch64-apple-darwin"
STAGE0_DIR="${LSHARP_NATIVE_MACOS_AARCH64_STAGE0_DIR:-${ROOT_DIR}/ci-artifacts/native-stage0/${TARGET}/current}"
SOURCE_COMMIT="$(git rev-parse --verify HEAD 2>/dev/null || true)"
WORK_DIR=""
KEEP_WORK_DIR="${LSHARP_NATIVE_MACOS_AARCH64_KEEP_STAGE0_WORK_DIR:-0}"
TMPDIR_ROOT="${TMPDIR:-/tmp}"
TMPDIR_ROOT="${TMPDIR_ROOT%/}"

die() {
  echo "ERROR: $*" >&2
  exit 1
}

[[ -n "$TMPDIR_ROOT" ]] || die "TMPDIR must not be root"

require_file() {
  local path="$1"
  local description="$2"
  [[ -f "$path" && -s "$path" ]] || die "$description is required: $path"
}

require_safe_output_path() {
  case "$1" in
    "${ROOT_DIR}/ci-artifacts/native-stage0/${TARGET}/"*|/tmp/lsharp-*) ;;
    *) die "stage0 output must be under the worktree native-stage0 artifact directory or /tmp/lsharp-*: $1" ;;
  esac
}

cleanup() {
  if [[ -n "$WORK_DIR" && "$KEEP_WORK_DIR" != "1" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

[[ "$(uname -s)" == "Darwin" && "$(uname -m)" == "arm64" ]] \
  || die "current-source Mac stage0 producer requires macOS arm64"
[[ "$SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]] \
  || die "current checkout source commit is unavailable: $SOURCE_COMMIT"

require_safe_output_path "$STAGE0_DIR"
[[ ! -e "$STAGE0_DIR" && ! -L "$STAGE0_DIR" ]] \
  || die "stage0 output directory already exists: $STAGE0_DIR"

WORK_DIR="$(mktemp -d "${TMPDIR_ROOT}/lsharp-native-macos-aarch64-stage0.XXXXXX")"
APP_ARTIFACT_DIR="$WORK_DIR/app-cli"
STAGE0_COMPILER_ARTIFACT_DIR="$WORK_DIR/stage0-compiler"
CARGO_TARGET_DIR="$WORK_DIR/cargo-target"
mkdir -p "$APP_ARTIFACT_DIR" "$(dirname "$STAGE0_DIR")"

LSHARP_NATIVE_MACOS_AARCH64_RELEASE_ARTIFACT_DIR="$APP_ARTIFACT_DIR" \
LSHARP_NATIVE_MACOS_AARCH64_STAGE0_COMPILER_ARTIFACT_DIR="$STAGE0_COMPILER_ARTIFACT_DIR" \
LSHARP_NATIVE_MACOS_AARCH64_CARGO_TARGET_DIR="$CARGO_TARGET_DIR" \
  bash "$ROOT_DIR/scripts/ci/native-macos-aarch64-selfhost-release.sh"

PROGRAM_PATH="$APP_ARTIFACT_DIR/program.native"
MANIFEST_PATH="$APP_ARTIFACT_DIR/manifest.json"
require_file "$PROGRAM_PATH" "current-source Mac App.Cli program"
require_file "$MANIFEST_PATH" "current-source Mac App.Cli manifest"
STAGE0_COMPILER_PATH="$STAGE0_COMPILER_ARTIFACT_DIR/compiler.native"
STAGE0_COMPILER_MANIFEST_PATH="$STAGE0_COMPILER_ARTIFACT_DIR/manifest.json"
require_file "$STAGE0_COMPILER_PATH" "current-source Mac stage0 compiler"
require_file "$STAGE0_COMPILER_MANIFEST_PATH" "current-source Mac stage0 compiler manifest"

python3 - "$MANIFEST_PATH" "$PROGRAM_PATH" "$SOURCE_COMMIT" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest_path, program_path, expected_source_commit = map(pathlib.Path, sys.argv[1:])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
expected = {
    "status": "pass",
    "artifact_kind": "native App.Cli release program",
    "target": "aarch64-apple-darwin",
    "entry_module": "App.Cli",
    "source": "src/App/Cli.ls",
    "source_commit": str(expected_source_commit),
    "selfhost_fixed_point": True,
    "program_sha256": hashlib.sha256(program_path.read_bytes()).hexdigest(),
}
for key, value in expected.items():
    if manifest.get(key) != value:
        raise SystemExit(f"current-source Mac App.Cli manifest mismatch: {key}")
PY

python3 - "$STAGE0_COMPILER_MANIFEST_PATH" "$STAGE0_COMPILER_PATH" "$SOURCE_COMMIT" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest_path, compiler_path, expected_source_commit = map(pathlib.Path, sys.argv[1:])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
expected = {
    "status": "pass",
    "artifact_kind": "native stage0 compiler",
    "target": "aarch64-apple-darwin",
    "entry_module": "App.Cli",
    "source": "src/App/Cli.ls",
    "source_commit": str(expected_source_commit),
    "compiler_sha256": hashlib.sha256(compiler_path.read_bytes()).hexdigest(),
}
for key, value in expected.items():
    if manifest.get(key) != value:
        raise SystemExit(f"current-source Mac stage0 compiler manifest mismatch: {key}")
PY

bash "$ROOT_DIR/scripts/ci/package-native-stage0.sh" \
  --target "$TARGET" \
  --source-commit "$SOURCE_COMMIT" \
  --compiler "$STAGE0_COMPILER_PATH" \
  --transport-driver "$ROOT_DIR/scripts/ci/native-stage0-transport-macos-aarch64.sh" \
  --materializer "$ROOT_DIR/scripts/ci/materialize-native-macos-aarch64-bundle.py" \
  --output-dir "$STAGE0_DIR"

python3 - "$STAGE0_DIR/manifest.json" "$STAGE0_DIR" "$SOURCE_COMMIT" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
package_dir = pathlib.Path(sys.argv[2])
expected_source_commit = sys.argv[3]
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("Mac stage0 manifest kind mismatch")
if manifest.get("target") != "aarch64-apple-darwin":
    raise SystemExit("Mac stage0 manifest target mismatch")
if manifest.get("source_commit") != expected_source_commit:
    raise SystemExit("Mac stage0 manifest source commit mismatch")
for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"Mac stage0 manifest {field} is invalid")
    path = package_dir / value
    if not path.is_file() or not path.stat().st_size:
        raise SystemExit(f"Mac stage0 payload is unavailable: {path}")
    if not (path.stat().st_mode & 0o111):
        raise SystemExit(f"Mac stage0 payload is not executable: {path}")
PY

echo "Mac current-source native stage0: $STAGE0_DIR ($SOURCE_COMMIT)"
