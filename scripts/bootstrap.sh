#!/usr/bin/env bash
# OPS-07: stage0 package から stage1/stage2 launcher + component を組み立てる
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE0_DIR="${STAGE0_DIR:-$ROOT/stage0}"
STAGE1_DIR="${STAGE1_DIR:-$ROOT/stage1}"
STAGE2_DIR="${STAGE2_DIR:-$ROOT/stage2}"
ENTRY_FILE="${ENTRY_FILE:-$ROOT/selfhost/src/App/EmbeddedCli.ls}"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "ERROR: required file not found: $path" >&2
    exit 1
  fi
}

resolve_launcher_bin() {
  local dir="$1"
  if [[ -f "$dir/lsharp" ]]; then
    printf '%s\n' "$dir/lsharp"
    return 0
  fi
  if [[ -f "$dir/lsharp.exe" ]]; then
    printf '%s\n' "$dir/lsharp.exe"
    return 0
  fi
  echo "ERROR: lsharp launcher not found under $dir" >&2
  exit 1
}

resolve_lsp_bin() {
  local dir="$1"
  if [[ -f "$dir/lsharp-lsp" ]]; then
    printf '%s\n' "$dir/lsharp-lsp"
    return 0
  fi
  if [[ -f "$dir/lsharp-lsp.exe" ]]; then
    printf '%s\n' "$dir/lsharp-lsp.exe"
    return 0
  fi
  return 1
}

copy_stage_launcher() {
  local source_dir="$1"
  local dest_dir="$2"
  local launcher_bin
  mkdir -p "$dest_dir"
  launcher_bin="$(resolve_launcher_bin "$source_dir")"
  cp -f "$launcher_bin" "$dest_dir/$(basename "$launcher_bin")"
  if lsp_bin="$(resolve_lsp_bin "$source_dir")"; then
    cp -f "$lsp_bin" "$dest_dir/$(basename "$lsp_bin")"
  fi
}

compile_stage_component() {
  local launcher_dir="$1"
  local output_dir="$2"
  local launcher_bin
  launcher_bin="$(resolve_launcher_bin "$launcher_dir")"
  "$launcher_bin" compile "$ENTRY_FILE" -o "$output_dir/lsharp.component.wasm" >/dev/null
}

resolve_launcher_bin "$STAGE0_DIR" >/dev/null
require_file "$STAGE0_DIR/lsharp.component.wasm"
require_file "$ENTRY_FILE"

echo "=== bootstrap: stage0 -> stage1 component ==="
rm -rf "$STAGE1_DIR" "$STAGE2_DIR"
copy_stage_launcher "$STAGE0_DIR" "$STAGE1_DIR"
compile_stage_component "$STAGE0_DIR" "$STAGE1_DIR"

echo "=== bootstrap: stage1 -> stage2 component ==="
copy_stage_launcher "$STAGE1_DIR" "$STAGE2_DIR"
compile_stage_component "$STAGE1_DIR" "$STAGE2_DIR"

echo "=== bootstrap: compare stage1/stage2 component ==="
cmp -s "$STAGE1_DIR/lsharp.component.wasm" "$STAGE2_DIR/lsharp.component.wasm" || {
  echo "ERROR: stage1/stage2 component bytes differ" >&2
  exit 1
}

echo "bootstrap: OK ($STAGE1_DIR, $STAGE2_DIR)"
