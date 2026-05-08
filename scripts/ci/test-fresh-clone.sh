#!/usr/bin/env bash
# OPS-07a/OPS-07b: clean checkout smoke と downloaded artifact ベースの binary-only smoke
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCHIVE_PATH="${1:-}"

if [[ $# -gt 1 ]]; then
  echo "Usage: $0 [release-archive]" >&2
  exit 1
fi

if [[ -n "$ARCHIVE_PATH" ]]; then
  WORK_DIR="${WORK_DIR:-$ROOT/target/ci/test-fresh-clone}"
else
  WORK_DIR="${WORK_DIR:-$ROOT/target/ci/fresh-clone-smoke}"
fi

cleanup() {
  local exit_code=$?
  if [[ $exit_code -eq 0 && "${KEEP_WORK_DIR:-0}" != "1" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

resolve_selfhost_source() {
  local source_root="$1"
  local module="$2"
  local path
  path="$(find "$source_root/selfhost/src" -name "${module}.ls" -print -quit)"
  if [[ -z "$path" ]]; then
    echo "ERROR: canonical selfhost source for ${module}.ls not found" >&2
    exit 1
  fi
  printf '%s\n' "$path"
}

compile_representative_slices() {
  local source_root="$1"
  local lsharp_bin="$2"
  local smoke_out_dir="$3"

  echo "=== test-fresh-clone: compile representative selfhost / stdlib slices ==="
  mkdir -p "$smoke_out_dir"
  "$lsharp_bin" compile "$(resolve_selfhost_source "$source_root" Token)" -o "$smoke_out_dir/selfhost_Token.wasm"
  "$lsharp_bin" compile "$source_root/stdlib/Core.ls" -o "$smoke_out_dir/stdlib_Core.wasm"

  if [[ ! -s "$smoke_out_dir/selfhost_Token.wasm" ]]; then
    echo "ERROR: canonical selfhost Token compile output is empty" >&2
    exit 1
  fi

  if [[ ! -s "$smoke_out_dir/stdlib_Core.wasm" ]]; then
    echo "ERROR: stdlib/Core.ls compile output is empty" >&2
    exit 1
  fi
}

resolve_packaged_lsharp_bin() {
  local extract_dir="$1"
  local candidate
  candidate="$(find "$extract_dir" -mindepth 1 -maxdepth 3 -type f \( -name 'lsharp' -o -name 'lsharp.exe' \) -print -quit)"
  if [[ -z "$candidate" ]]; then
    echo "ERROR: packaged lsharp binary not found under $extract_dir" >&2
    exit 1
  fi
  printf '%s\n' "$candidate"
}

run_binary_only_smoke() {
  local archive_path="$1"
  local release_work_dir="$WORK_DIR/release-smoke"
  local smoke_out_dir="$WORK_DIR/compile"
  local packaged_lsharp

  if [[ ! -f "$archive_path" ]]; then
    echo "ERROR: archive not found: $archive_path" >&2
    exit 1
  fi

  echo "=== test-fresh-clone: downloaded artifact binary-only smoke ==="
  rm -rf "$WORK_DIR"
  mkdir -p "$WORK_DIR"

  KEEP_WORK_DIR=1 WORK_DIR="$release_work_dir" bash "$ROOT/scripts/ci/release-smoke.sh" "$archive_path"
  packaged_lsharp="$(resolve_packaged_lsharp_bin "$release_work_dir/extract")"

  echo "=== test-fresh-clone: reuse default-path smoke ==="
  OUT_DIR="$WORK_DIR/default-path-smoke" \
  LSHARP_BIN="$packaged_lsharp" \
    bash "$ROOT/scripts/ci/default-path-smoke.sh"

  echo "=== test-fresh-clone: README Quick Start smoke ==="
  SMOKE_DIR="$WORK_DIR/readme-smoke" \
  LSHARP_BIN="$packaged_lsharp" \
    bash "$ROOT/scripts/smoke_test_readme.sh"

  compile_representative_slices "$ROOT" "$packaged_lsharp" "$smoke_out_dir"
  echo "test-fresh-clone (binary-only): OK"
}

run_clean_checkout_smoke() {
  local clone_dir="$WORK_DIR/repo"
  local build_dir="$WORK_DIR/build"
  local smoke_out_dir="$WORK_DIR/compile"
  local lsharp_bin

  echo "=== fresh-clone-smoke: stage clean checkout ==="
  rm -rf "$WORK_DIR"
  mkdir -p "$clone_dir" "$build_dir" "$smoke_out_dir"

  # git archive 相当の clean checkout を、未コミット変更も含めて tar で再現する
  tar -cf - \
    --exclude='.git' \
    --exclude='target' \
    --exclude='.copilot' \
    -C "$ROOT" . | tar -xf - -C "$clone_dir"

  cd "$clone_dir"

  echo "=== fresh-clone-smoke: build lsharp binary ==="
  cargo build -p lsharp-driver -q --target-dir "$build_dir"

  lsharp_bin="$build_dir/debug/lsharp"
  if [[ ! -x "$lsharp_bin" ]]; then
    echo "ERROR: lsharp binary not executable: $lsharp_bin" >&2
    exit 1
  fi

  echo "=== fresh-clone-smoke: reuse default-path smoke ==="
  OUT_DIR="$clone_dir/target/ci/default-path-smoke" \
  LSHARP_BIN="$lsharp_bin" \
    bash scripts/ci/default-path-smoke.sh

  compile_representative_slices "$clone_dir" "$lsharp_bin" "$clone_dir/target/ci/compile"
  echo "fresh-clone-smoke: OK"
}

if [[ -n "$ARCHIVE_PATH" ]]; then
  run_binary_only_smoke "$ARCHIVE_PATH"
else
  run_clean_checkout_smoke
fi
