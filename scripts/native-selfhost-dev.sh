#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE0_DIR="${NATIVE_STAGE0_DIR:-}"
SOURCE_ROOT="${NATIVE_SOURCE_ROOT:-$ROOT/selfhost}"
STAGE_DIR="${NATIVE_STAGE_DIR:-$ROOT/.native-selfhost-dev}"
RELATIVE_ENTRY="${NATIVE_RELATIVE_ENTRY:-src/App/Cli.ls}"
DECODER="$ROOT/scripts/ci/decode-native-selfhost-transport.py"
LSP_STDIO_SHIM="$ROOT/scripts/native-selfhost-lsp-stdio.py"
INSTALL_HELPER="$ROOT/scripts/native-selfhost-install.py"
REPL_HELPER="$ROOT/scripts/native-selfhost-repl.py"
DOC_HELPER="$ROOT/scripts/native-selfhost-doc.py"
COMPONENT_HELPER="$ROOT/scripts/native-selfhost-component.py"
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

default_component_output_path() {
  python3 - "$1" <<'PY'
import pathlib
import sys

try:
    print(pathlib.Path(sys.argv[1]).with_suffix(".component.wasm"))
except ValueError as error:
    raise SystemExit(f"error: cannot derive component output path: {error}")
PY
}

component_output_path() {
  local args=("$@")
  local index=2
  local output_path=""
  local target=""

  [[ "${args[0]:-}" == "compile" || "${args[0]:-}" == "build" ]] || return 1
  [[ ${#args[@]} -ge 2 ]] || return 1
  [[ -f "${args[1]}" ]] || return 1

  while (( index < ${#args[@]} )); do
    case "${args[index]}" in
      -o|--output)
        index=$((index + 1))
        (( index < ${#args[@]} )) || return 1
        output_path="${args[index]}"
        ;;
      --target)
        index=$((index + 1))
        (( index < ${#args[@]} )) || return 1
        case "${args[index]}" in
          wasi-preview1)
            target="wasi-preview1"
            ;;
          wasi-component|wasm)
            target="wasi-component"
            ;;
          *)
            return 1
            ;;
        esac
        ;;
      *)
        return 1
        ;;
    esac
    index=$((index + 1))
  done

  if [[ -z "$target" ]]; then
    if [[ -z "$output_path" || "$(basename "$output_path")" == *.component.wasm ]]; then
      target="wasi-component"
    else
      target="wasi-preview1"
    fi
  fi

  [[ "$target" == "wasi-component" ]] || return 1
  if [[ -z "$output_path" ]]; then
    output_path="$(default_component_output_path "${args[1]}")" || return 1
  fi
  printf '%s\n' "$output_path"
}

unsupported_compile_target() {
  local args=("$@")
  local index=2

  [[ "${args[0]:-}" == "compile" || "${args[0]:-}" == "build" ]] || return 1

  while (( index < ${#args[@]} )); do
    if [[ "${args[index]}" == "--target" ]]; then
      index=$((index + 1))
      (( index < ${#args[@]} )) || return 1
      case "${args[index]}" in
        web-wasm|native)
          printf '%s\n' "${args[index]}"
          return 0
          ;;
      esac
    fi
    index=$((index + 1))
  done

  return 1
}

unsupported_compile_option() {
  local args=("$@")

  [[ "${args[0]:-}" == "compile" || "${args[0]:-}" == "build" ]] || return 1
  for arg in "${args[@]:2}"; do
    if [[ "$arg" == "--emit-ir" ]]; then
      printf '%s\n' "$arg"
      return 0
    fi
  done

  return 1
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

if [[ "${1:-}" == "install" ]]; then
  [[ $# -eq 1 ]] || die "native install does not accept command arguments"
  [[ -f "$INSTALL_HELPER" ]] || die "native install helper not found: $INSTALL_HELPER"
  exec python3 "$INSTALL_HELPER" --project-dir "$PWD"
fi

if [[ "${1:-}" == "repl" ]]; then
  [[ -f "$REPL_HELPER" ]] || die "native REPL helper not found: $REPL_HELPER"
  exec python3 "$REPL_HELPER" --program "$STAGE_DIR/program.native" "${@:2}"
fi

if [[ "${1:-}" == "doc" ]]; then
  [[ -f "$DOC_HELPER" ]] || die "native documentation helper not found: $DOC_HELPER"
  exec python3 "$DOC_HELPER" --program "$STAGE_DIR/program.native" "${@:2}"
fi

if [[ "${1:-}" == "mcp-server" ]]; then
  die "native selfhost runner does not provide mcp-server; use the Rust host integration"
fi

if [[ "${1:-}" == "lsp" ]]; then
  if [[ "${2:-}" != "--stdio" ]]; then
    die "native selfhost runner supports lsp only with --stdio"
  fi
  [[ -f "$LSP_STDIO_SHIM" ]] || die "native LSP stdio shim not found: $LSP_STDIO_SHIM"
  exec python3 "$LSP_STDIO_SHIM" --program "$STAGE_DIR/program.native" -- "${@:3}"
fi

if UNSUPPORTED_TARGET="$(unsupported_compile_target "$@")"; then
  die "native selfhost runner does not support --target $UNSUPPORTED_TARGET; use wasi-preview1 or wasi-component"
fi

if UNSUPPORTED_OPTION="$(unsupported_compile_option "$@")"; then
  die "native selfhost runner does not support $UNSUPPORTED_OPTION; use the Rust host integration"
fi

if COMPONENT_OUTPUT="$(component_output_path "$@")"; then
  [[ -f "$COMPONENT_HELPER" ]] || die "native component helper not found: $COMPONENT_HELPER"
  exec python3 "$COMPONENT_HELPER" \
    --program "$STAGE_DIR/program.native" \
    --command "$1" \
    --source "$2" \
    --output "$COMPONENT_OUTPUT"
fi

exec "$STAGE_DIR/program.native" "$@"
