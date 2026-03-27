#!/bin/bash

set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/../.." && pwd)
OUT_DIR="${OUT_DIR:-$ROOT_DIR/target/ci/phase11-compile}"
LSHARP_BIN="${LSHARP_BIN:-$ROOT_DIR/target/debug/lsharp}"

SELFHOST_MODULES=(
  AST
  Cli
  Closure
  Codegen
  Compiler
  Constraints
  Derive
  DocTools
  Emit
  Formatter
  GC
  HtmlDoc
  Hygiene
  IR
  JsonRpc
  Lexer
  Linker
  Linter
  Lower
  LowerDecl
  LowerExpr
  LowerPattern
  LspServer
  MacroExpand
  Main
  MetadataCheck
  ModuleGraph
  NativeCodegen
  NativeEmit
  NativeTarget
  Parser
  Span
  TestRunner
  Token
  Type
  TypeInfer
  TypeScheme
  WasiBackend
  WasiRunner
  WasmEmit
)

STDLIB_MODULES=(
  Core
  Char
  Debug
  IO
  List
  Map
  Path
  Set
  String
  Vector
  Json
)

EXAMPLE_FILES=(
  examples/fib.ls
  examples/module.ls
  examples/trait.ls
)

compile_target() {
  local label="$1"
  local source="$2"
  local output_name="$3"

  echo "=== [${label}] ${source} ==="
  "$LSHARP_BIN" compile "${source}" -o "${OUT_DIR}/${output_name}"
}

ensure_lsharp_bin() {
  if [[ -x "$LSHARP_BIN" ]]; then
    return
  fi

  echo "=== compile-phase11-inputs: build lsharp binary ==="
  cargo build -p lsharp-driver -q

  if [[ ! -x "$LSHARP_BIN" ]]; then
    echo "ERROR: lsharp binary not executable: $LSHARP_BIN"
    exit 1
  fi
}

cd "$ROOT_DIR"
mkdir -p "$OUT_DIR"
ensure_lsharp_bin

echo "=== Phase 11 fixed input set compile gate ==="
echo "output dir: ${OUT_DIR}"
echo "compiler: ${LSHARP_BIN}"
echo ""

for module in "${SELFHOST_MODULES[@]}"; do
  compile_target "selfhost" "selfhost/${module}.ls" "selfhost_${module}.wasm"
done

for module in "${STDLIB_MODULES[@]}"; do
  compile_target "stdlib" "stdlib/${module}.ls" "stdlib_${module}.wasm"
done

for file in "${EXAMPLE_FILES[@]}"; do
  name=$(basename "${file}" .ls)
  compile_target "example" "${file}" "example_${name}.wasm"
done

echo ""
echo "Phase 11 fixed input set compile gate complete (no known compile blockers)."
