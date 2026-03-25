#!/usr/bin/env bash
# OPS-05: `cargo run` ではなくビルド済み `lsharp` バイナリが compile/check を実行できること（default path 移行の第1段）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "=== default-path-smoke: build lsharp binary ==="
cargo build -p lsharp-driver -q

LSHARP_BIN="${LSHARP_BIN:-$ROOT/target/debug/lsharp}"
if [[ ! -x "$LSHARP_BIN" ]]; then
  echo "ERROR: lsharp binary not executable: $LSHARP_BIN"
  exit 1
fi

echo "=== default-path-smoke: check / compile (examples/fib.ls) ==="
"$LSHARP_BIN" check examples/fib.ls
OUT_WASM="${TMPDIR:-/tmp}/lsharp_default_path_smoke_$$.wasm"
"$LSHARP_BIN" compile examples/fib.ls -o "$OUT_WASM"
if [[ ! -s "$OUT_WASM" ]]; then
  echo "ERROR: wasm output empty: $OUT_WASM"
  exit 1
fi
rm -f "$OUT_WASM"
echo "default-path-smoke: OK"
