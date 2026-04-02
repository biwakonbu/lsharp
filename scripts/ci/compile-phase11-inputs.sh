#!/bin/bash

set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/../.." && pwd)
OUT_DIR="${OUT_DIR:-$ROOT_DIR/target/ci/phase11-compile}"
LSHARP_BIN="${LSHARP_BIN:-$ROOT_DIR/target/debug/lsharp}"
RUN_BOOTSTRAP_FIXED_POINT="${RUN_BOOTSTRAP_FIXED_POINT:-0}"
BOOTSTRAP_DIFF_ARTIFACT_ID="${BOOTSTRAP_DIFF_ARTIFACT_ID:-${GITHUB_SHA:-local}}"

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

resolve_selfhost_source() {
  local module="$1"
  local path
  path="$(find selfhost/src -name "${module}.ls" -print -quit)"
  if [[ -z "$path" ]]; then
    echo "ERROR: canonical selfhost source for ${module}.ls not found"
    exit 1
  fi
  printf '%s\n' "$path"
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
mkdir -p "$ROOT_DIR/ci-artifacts/bootstrap-diff/$BOOTSTRAP_DIFF_ARTIFACT_ID"
ensure_lsharp_bin

echo "=== Phase 11 fixed input set compile gate ==="
echo "output dir: ${OUT_DIR}"
echo "compiler: ${LSHARP_BIN}"
echo ""

for module in "${SELFHOST_MODULES[@]}"; do
  compile_target "selfhost" "$(resolve_selfhost_source "$module")" "selfhost_${module}.wasm"
done

for module in "${STDLIB_MODULES[@]}"; do
  compile_target "stdlib" "stdlib/${module}.ls" "stdlib_${module}.wasm"
done

for file in "${EXAMPLE_FILES[@]}"; do
  name=$(basename "${file}" .ls)
  compile_target "example" "${file}" "example_${name}.wasm"
done

if [[ "$RUN_BOOTSTRAP_FIXED_POINT" == "1" ]]; then
  echo ""
  echo "=== Bootstrap fixed-point verification ==="
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_point_stage2_stage3 -- --exact --nocapture
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage2_self_feed_fixed_input_set -- --exact --nocapture
fi

echo ""
echo "Phase 11 fixed input set compile gate complete (no known compile blockers)."
