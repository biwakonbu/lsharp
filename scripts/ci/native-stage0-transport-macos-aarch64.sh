#!/usr/bin/env bash
set -euo pipefail

INITIAL_DIRECTORY="$(pwd -P)"
COMPILER=""
SOURCE_ROOT=""
ENTRY=""
TRANSPORT_OUTPUT=""
TEMP_OUTPUT=""
TEMP_STDERR=""
ENTRY_PATH=""

usage() {
  cat <<'EOF'
usage: scripts/ci/native-stage0-transport-macos-aarch64.sh COMPILER SOURCE_ROOT RELATIVE_ENTRY TRANSPORT_OUTPUT
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s\n' "$path"
  else
    printf '%s/%s\n' "$INITIAL_DIRECTORY" "$path"
  fi
}

require_darwin_arm64() {
  # テスト時だけホスト判定を迂回し、コンパイラ呼び出しは常に実行する。
  if [[ "${LSHARP_NATIVE_STAGE0_TRANSPORT_TEST_ALLOW_UNSUPPORTED_HOST:-}" == "1" ]]; then
    return
  fi

  local system_name
  local machine_name
  system_name="$(uname -s)"
  machine_name="$(uname -m)"
  if [[ "$system_name" != "Darwin" || "$machine_name" != "arm64" ]]; then
    die "native stage0 transport requires Darwin arm64 (detected: $system_name $machine_name)"
  fi
}

validate_relative_entry() {
  local entry="$1"
  case "$entry" in
    ""|/*|*//*|.|..|./*|../*|*/./*|*/../*|*/.|*/..)
      die "entry path must be relative and stay within source root: $entry"
      ;;
  esac
}

resolve_path() {
  local path="$1"
  local parent
  local name
  local target
  local link_count=0

  while [[ -L "$path" ]]; do
    (( link_count += 1 ))
    (( link_count <= 64 )) || return 1
    parent="$(cd -P "$(dirname "$path")" && pwd -P)" || return 1
    target="$(readlink "$path")" || return 1
    if [[ "$target" == /* ]]; then
      path="$target"
    else
      path="$parent/$target"
    fi
  done

  parent="$(cd -P "$(dirname "$path")" && pwd -P)" || return 1
  name="$(basename "$path")"
  printf '%s/%s\n' "$parent" "$name"
}

cleanup() {
  [[ -z "$TEMP_OUTPUT" ]] || rm -f "$TEMP_OUTPUT"
  [[ -z "$TEMP_STDERR" ]] || rm -f "$TEMP_STDERR"
}

report_compiler_stderr() {
  if [[ -s "$TEMP_STDERR" ]]; then
    cat "$TEMP_STDERR" >&2
  fi
}

[[ $# -eq 4 ]] || {
  usage >&2
  exit 2
}

require_darwin_arm64

COMPILER="$(absolute_path "$1")"
SOURCE_ROOT="$(absolute_path "$2")"
ENTRY="$3"
TRANSPORT_OUTPUT="$(absolute_path "$4")"

[[ -f "$COMPILER" ]] || die "compiler is not a regular file: $COMPILER"
[[ -x "$COMPILER" ]] || die "compiler is not executable: $COMPILER"
[[ -d "$SOURCE_ROOT" ]] || die "source root not found: $SOURCE_ROOT"
validate_relative_entry "$ENTRY"
SOURCE_ROOT="$(cd "$SOURCE_ROOT" && pwd -P)"
ENTRY_PATH="$(resolve_path "$SOURCE_ROOT/$ENTRY")" || die "could not resolve entry path: $ENTRY"
if [[ "$SOURCE_ROOT" != "/" && "$ENTRY_PATH" != "$SOURCE_ROOT/"* ]]; then
  die "entry path must stay within source root: $ENTRY"
fi
[[ -f "$ENTRY_PATH" ]] || die "entry file not found: $SOURCE_ROOT/$ENTRY"

OUTPUT_PARENT="$(dirname "$TRANSPORT_OUTPUT")"
OUTPUT_NAME="$(basename "$TRANSPORT_OUTPUT")"
case "$OUTPUT_NAME" in
  .|..)
    die "transport output must name a file: $TRANSPORT_OUTPUT"
    ;;
esac
mkdir -p "$OUTPUT_PARENT"
OUTPUT_PARENT="$(cd "$OUTPUT_PARENT" && pwd -P)"
TRANSPORT_OUTPUT="$OUTPUT_PARENT/$OUTPUT_NAME"
[[ ! -d "$TRANSPORT_OUTPUT" ]] || die "transport output must not be a directory: $TRANSPORT_OUTPUT"

trap cleanup EXIT
rm -f "$TRANSPORT_OUTPUT"
TEMP_OUTPUT="$(mktemp "$OUTPUT_PARENT/.native-stage0-transport-output.XXXXXX")"
TEMP_STDERR="$(mktemp "$OUTPUT_PARENT/.native-stage0-transport-stderr.XXXXXX")"

set +e
(
  cd "$SOURCE_ROOT"
  "$COMPILER" "$ENTRY"
) >"$TEMP_OUTPUT" 2>"$TEMP_STDERR"
COMPILER_STATUS=$?
set -e

if [[ "$COMPILER_STATUS" -ne 0 ]]; then
  report_compiler_stderr
  die "native compiler failed with exit status $COMPILER_STATUS"
fi

if [[ -s "$TEMP_STDERR" ]]; then
  report_compiler_stderr
  die "native compiler wrote to stderr"
fi

[[ -s "$TEMP_OUTPUT" ]] || die "native compiler produced an empty transport output"

mv -f "$TEMP_OUTPUT" "$TRANSPORT_OUTPUT"
TEMP_OUTPUT=""
