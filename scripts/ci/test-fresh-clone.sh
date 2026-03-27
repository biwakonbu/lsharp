#!/usr/bin/env bash
# OPS-07a: Rust 依存のままでも clean checkout 由来のビルド経路を継続検証する
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK_DIR="${WORK_DIR:-$ROOT/target/ci/fresh-clone-smoke}"
CLONE_DIR="$WORK_DIR/repo"
BUILD_DIR="$WORK_DIR/build"
SMOKE_OUT_DIR="$WORK_DIR/compile"

cleanup() {
  local exit_code=$?
  if [[ $exit_code -eq 0 && "${KEEP_WORK_DIR:-0}" != "1" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

echo "=== fresh-clone-smoke: stage clean checkout ==="
rm -rf "$WORK_DIR"
mkdir -p "$CLONE_DIR" "$BUILD_DIR" "$SMOKE_OUT_DIR"

# git archive 相当の clean checkout を、未コミット変更も含めて tar で再現する
tar -cf - \
  --exclude='.git' \
  --exclude='target' \
  --exclude='.copilot' \
  -C "$ROOT" . | tar -xf - -C "$CLONE_DIR"

cd "$CLONE_DIR"

echo "=== fresh-clone-smoke: build lsharp binary ==="
cargo build -p lsharp-driver -q --target-dir "$BUILD_DIR"

LSHARP_BIN="$BUILD_DIR/debug/lsharp"
if [[ ! -x "$LSHARP_BIN" ]]; then
  echo "ERROR: lsharp binary not executable: $LSHARP_BIN"
  exit 1
fi

echo "=== fresh-clone-smoke: reuse default-path smoke ==="
OUT_DIR="$WORK_DIR/default-path-smoke" \
LSHARP_BIN="$LSHARP_BIN" \
  bash scripts/ci/default-path-smoke.sh

echo "=== fresh-clone-smoke: compile representative selfhost / stdlib slices ==="
"$LSHARP_BIN" compile selfhost/Token.ls -o "$SMOKE_OUT_DIR/selfhost_Token.wasm"
"$LSHARP_BIN" compile stdlib/Core.ls -o "$SMOKE_OUT_DIR/stdlib_Core.wasm"

if [[ ! -s "$SMOKE_OUT_DIR/selfhost_Token.wasm" ]]; then
  echo "ERROR: selfhost/Token.ls compile output is empty"
  exit 1
fi

if [[ ! -s "$SMOKE_OUT_DIR/stdlib_Core.wasm" ]]; then
  echo "ERROR: stdlib/Core.ls compile output is empty"
  exit 1
fi

echo "fresh-clone-smoke: OK"
