#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE0_DIR="${NATIVE_STAGE0_DIR:-}"
SOURCE_ROOT="${NATIVE_SOURCE_ROOT:-$ROOT/selfhost}"
STAGE_DIR="${NATIVE_STAGE_DIR:-$ROOT/.native-selfhost-dev}"
RELATIVE_ENTRY="${NATIVE_RELATIVE_ENTRY:-src/App/Cli.ls}"
DECODER="$ROOT/scripts/ci/decode-native-selfhost-transport.py"
FORCE_BOOTSTRAP=0

usage() {
  cat <<'EOF'
usage: scripts/native-selfhost-dev.sh [options] [--] [program args...]

options:
  --stage0-dir DIR  required stage0 package directory (or NATIVE_STAGE0_DIR)
  --source-root DIR source tree to compile (default: ./selfhost)
  --stage-dir DIR   generated native stage directory
  --entry PATH      entry path relative to source root
  --bootstrap       regenerate the native program even when sources are unchanged
  --help            show this help
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_option_value() {
  if [[ $# -lt 2 || -z "$2" ]]; then
    echo "error: $1 requires a value" >&2
    usage >&2
    exit 2
  fi
}

hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    die "sha256sum or shasum is required for source freshness"
  fi
}

hash_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    die "sha256sum or shasum is required for source freshness"
  fi
}

source_fingerprint() {
  (
    cd "$SOURCE_ROOT"
    while IFS= read -r source_path; do
      printf '%s  %s\n' "$(hash_file "$source_path")" "$source_path"
    done < <(find . -type f -print | LC_ALL=C sort)
  ) | hash_stream
}

parse_stage0_manifest() {
  local manifest_path="$STAGE0_DIR/manifest.json"
  [[ -f "$manifest_path" ]] || die "stage0 manifest is required: $manifest_path"

  python3 - "$manifest_path" <<'PY'
import json
import os
import sys

manifest_path = sys.argv[1]
try:
    with open(manifest_path, encoding="utf-8") as source:
        manifest = json.load(source)
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"error: invalid stage0 manifest {manifest_path}: {error}")

if manifest.get("kind") != "lsharp-native-selfhost-stage0":
    raise SystemExit("error: stage0 manifest kind must be lsharp-native-selfhost-stage0")

target = manifest.get("target")
if target not in ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin"):
    raise SystemExit("error: stage0 manifest target must be a supported native target")
print(target)

for field in ("compiler", "transport_driver", "materializer"):
    value = manifest.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"error: stage0 manifest {field} must be a non-empty relative path")
    normalized = os.path.normpath(value)
    if os.path.isabs(value) or normalized == ".." or normalized.startswith(f"..{os.sep}"):
        raise SystemExit(f"error: stage0 manifest {field} must be a relative path")
    print(value)
PY
}

copy_source_tree() {
  local copied_source="$STAGE_DIR/source"
  rm -rf "$copied_source"
  mkdir -p "$copied_source"
  cp -R "$SOURCE_ROOT/." "$copied_source/"
}

stage_is_ready() {
  local expected_fingerprint="$1"
  local stamp_path="$STAGE_DIR/.source-fingerprint.sha256"
  [[ -x "$STAGE_DIR/program.native" ]] || return 1
  [[ -f "$stamp_path" ]] || return 1
  [[ "$(<"$stamp_path")" == "$expected_fingerprint" ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stage0-dir)
      require_option_value "$@"
      STAGE0_DIR="$2"
      shift 2
      ;;
    --source-root)
      require_option_value "$@"
      SOURCE_ROOT="$2"
      shift 2
      ;;
    --stage-dir)
      require_option_value "$@"
      STAGE_DIR="$2"
      shift 2
      ;;
    --entry)
      require_option_value "$@"
      RELATIVE_ENTRY="$2"
      shift 2
      ;;
    --bootstrap)
      FORCE_BOOTSTRAP=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

[[ -n "$STAGE0_DIR" ]] || die "NATIVE_STAGE0_DIR or --stage0-dir is required"
[[ -d "$STAGE0_DIR" ]] || die "stage0 directory not found: $STAGE0_DIR"
[[ -d "$SOURCE_ROOT" ]] || die "source root not found: $SOURCE_ROOT"
[[ ! "$RELATIVE_ENTRY" = /* ]] || die "entry path must be relative to source root: $RELATIVE_ENTRY"
[[ "$RELATIVE_ENTRY" != ".." && "$RELATIVE_ENTRY" != ../* ]] || die "entry path must stay within source root: $RELATIVE_ENTRY"
[[ -f "$SOURCE_ROOT/$RELATIVE_ENTRY" ]] || die "entry file not found: $SOURCE_ROOT/$RELATIVE_ENTRY"
[[ -f "$DECODER" ]] || die "native selfhost transport decoder not found: $DECODER"

unset LSHARP_PATH
unset LSHARP_DISABLE_EMBEDDED_COMPONENT

manifest_paths=()
while IFS= read -r path; do
  manifest_paths+=("$path")
done < <(parse_stage0_manifest)
[[ ${#manifest_paths[@]} -eq 4 ]] || die "stage0 manifest did not provide target and three executables"

TARGET="${manifest_paths[0]}"
COMPILER="$STAGE0_DIR/${manifest_paths[1]}"
TRANSPORT_DRIVER="$STAGE0_DIR/${manifest_paths[2]}"
MATERIALIZER="$STAGE0_DIR/${manifest_paths[3]}"
for executable in "$COMPILER" "$TRANSPORT_DRIVER" "$MATERIALIZER"; do
  [[ -x "$executable" ]] || die "stage0 manifest executable is unavailable: $executable"
done

mkdir -p "$STAGE_DIR"
FINGERPRINT="$(source_fingerprint)"

if [[ "$FORCE_BOOTSTRAP" == "1" ]] || ! stage_is_ready "$FINGERPRINT"; then
  copy_source_tree
  COPIED_SOURCE="$STAGE_DIR/source"
  TRANSPORT_OUTPUT="$STAGE_DIR/transport-output.txt"
  "$TRANSPORT_DRIVER" "$COMPILER" "$COPIED_SOURCE" "$RELATIVE_ENTRY" "$TRANSPORT_OUTPUT"
  python3 "$DECODER" --target "$TARGET" "$TRANSPORT_OUTPUT" "$STAGE_DIR"
  case "$TARGET" in
    x86_64-unknown-linux-gnu)
      LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0=1 \
        "$MATERIALIZER" "$STAGE_DIR" "$STAGE_DIR/stage-code.bin" "$STAGE_DIR/entrypoint-offset.txt"
      ;;
    aarch64-apple-darwin)
      LSHARP_NATIVE_MACOS_AARCH64_SKIP_ARGV0=1 \
        "$MATERIALIZER" "$STAGE_DIR" "$STAGE_DIR/stage-code.bin" "$STAGE_DIR/entrypoint-offset.txt"
      ;;
    *)
      die "unsupported native stage0 target: $TARGET"
      ;;
  esac
  [[ -x "$STAGE_DIR/program.native" ]] || die "materializer did not produce an executable program.native"
  printf '%s\n' "$FINGERPRINT" >"$STAGE_DIR/.source-fingerprint.sha256"
fi

exec "$STAGE_DIR/program.native" "$@"
