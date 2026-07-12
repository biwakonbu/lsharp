#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE0_DIR="${STAGE0_DIR:-$ROOT/stage0}"
STAGE1_DIR="${STAGE1_DIR:-$ROOT/stage1}"
STAGE2_DIR="${STAGE2_DIR:-$ROOT/stage2}"
ENTRY_FILE="${ENTRY_FILE:-$ROOT/selfhost/src/App/EmbeddedCli.ls}"
force_bootstrap=0

usage() {
  cat <<'EOF'
usage: scripts/selfhost-dev.sh [options] <command> [args...]

options:
  --stage0-dir DIR  stage0 package directory
  --stage1-dir DIR  stage1 output directory
  --stage2-dir DIR  stage2 output directory
  --entry-file FILE selfhost compiler entry file
  --bootstrap       rebuild stage1 and stage2 before running
  --help            show this help

stage2 is rebuilt automatically when the selfhost source fingerprint changes.
EOF
}

require_option_value() {
  if [[ $# -lt 2 || -z "$2" ]]; then
    echo "error: $1 requires a value" >&2
    usage >&2
    exit 2
  fi
}

resolve_stage2_launcher() {
  if [[ -x "$STAGE2_DIR/lsharp" ]]; then
    printf '%s\n' "$STAGE2_DIR/lsharp"
    return 0
  fi
  if [[ -x "$STAGE2_DIR/lsharp.exe" ]]; then
    printf '%s\n' "$STAGE2_DIR/lsharp.exe"
    return 0
  fi
  return 1
}

hash_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required for source freshness" >&2
    exit 1
  fi
}

hash_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required for source freshness" >&2
    exit 1
  fi
}

stage2_source_stamp_path() {
  printf '%s\n' "$STAGE2_DIR/.selfhost-dev-source.sha256"
}

source_fingerprint() {
  {
    printf '%s  %s\n' "$(hash_file "$ENTRY_FILE")" "$ENTRY_FILE"
    while IFS= read -r source_path; do
      if [[ "$source_path" != "$ENTRY_FILE" ]]; then
        printf '%s  %s\n' "$(hash_file "$source_path")" "$source_path"
      fi
    done < <(find "$ROOT/selfhost/src" -type f -name '*.ls' -print | LC_ALL=C sort)
  } | hash_stream
}

write_stage2_source_stamp() {
  local stamp_path fingerprint
  stamp_path="$(stage2_source_stamp_path)"
  fingerprint="$(source_fingerprint)"
  printf '%s\n' "$fingerprint" > "$stamp_path"
}

stage2_is_ready() {
  local stamp_path expected actual
  [[ -f "$STAGE2_DIR/lsharp.component.wasm" ]] || return 1
  resolve_stage2_launcher >/dev/null || return 1
  stamp_path="$(stage2_source_stamp_path)"
  [[ -f "$stamp_path" ]] || return 1
  expected="$(source_fingerprint)"
  actual="$(<"$stamp_path")"
  [[ "$actual" == "$expected" ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stage0-dir)
      require_option_value "$@"
      STAGE0_DIR="$2"
      shift 2
      ;;
    --stage1-dir)
      require_option_value "$@"
      STAGE1_DIR="$2"
      shift 2
      ;;
    --stage2-dir)
      require_option_value "$@"
      STAGE2_DIR="$2"
      shift 2
      ;;
    --entry-file)
      require_option_value "$@"
      ENTRY_FILE="$2"
      shift 2
      ;;
    --bootstrap)
      force_bootstrap=1
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

if [[ $# -eq 0 ]]; then
  echo "error: a stage2 command is required" >&2
  usage >&2
  exit 2
fi

if [[ ! -f "$ENTRY_FILE" ]]; then
  echo "error: selfhost entry file not found: $ENTRY_FILE" >&2
  exit 1
fi

unset LSHARP_PATH
unset LSHARP_DISABLE_EMBEDDED_COMPONENT

if [[ "$force_bootstrap" == "1" ]] || ! stage2_is_ready; then
  STAGE0_DIR="$STAGE0_DIR" \
    STAGE1_DIR="$STAGE1_DIR" \
    STAGE2_DIR="$STAGE2_DIR" \
    ENTRY_FILE="$ENTRY_FILE" \
    bash "$ROOT/scripts/bootstrap.sh"
  write_stage2_source_stamp
fi

stage2_launcher="$(resolve_stage2_launcher)" || {
  echo "error: stage2 launcher is unavailable after bootstrap: $STAGE2_DIR" >&2
  exit 1
}

exec "$stage2_launcher" "$@"
