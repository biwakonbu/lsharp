#!/bin/bash

set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/../.." && pwd)
OUT_DIR="${OUT_DIR:-$ROOT_DIR/target/ci/phase11-compile}"
LSHARP_BIN="${LSHARP_BIN:-$ROOT_DIR/target/debug/lsharp}"
RUN_BOOTSTRAP_FIXED_POINT="${RUN_BOOTSTRAP_FIXED_POINT:-0}"
RUN_BOOTSTRAP_LEGACY_STAGE1="${RUN_BOOTSTRAP_LEGACY_STAGE1:-0}"
RUN_INCREMENTAL_COMPARE="${RUN_INCREMENTAL_COMPARE:-0}"
RUN_INCREMENTAL_BENCHMARK="${RUN_INCREMENTAL_BENCHMARK:-0}"
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
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_point_stage2_stage3 -- --exact --ignored --nocapture
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage2_self_feed_fixed_input_set -- --exact --ignored --nocapture
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
     e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_input_set_stage_chain_match_cli_module -- --exact --ignored --nocapture
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
     e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_input_set_stage_chain_match_lsp_server_module -- --exact --ignored --nocapture
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
     e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_input_set_stage_chain_match -- --exact --ignored --nocapture
fi

if [[ "$RUN_BOOTSTRAP_LEGACY_STAGE1" == "1" ]]; then
  echo ""
  echo "=== Legacy stage1 bootstrap verification ==="
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_stage1_pipeline_verification -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_stage1_binary_structure -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_stage1_fixed_point_sections -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_ci_all_modules_compile -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_ci_stdlib_compile -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_ci_examples_compile -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_bootstrap_selfhost_modules_deterministic -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::bootstrap_selfhost_lsp_integration::test_e2e_selfhost_main_compile_if_let -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_compile_phase_probe_reaches_compile_complete -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_build_phase_probe_reaches_build_complete -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_cache_probe_emits_cache_marker -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_ast_chunked_step_progress_probe_reaches_first_pair_complete -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_module_resolver_first_defn_source_probe_reaches_prefix -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_module_resolver_ast_chunked_step_probe_reaches_completion -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_backend_compiler_pair_progress_probe_reaches_final_markers -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_compiler_mode_pair_progress_probe_reaches_final_markers -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_compiler_mode_token_debug_emits_token_count -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_pipeline_smoke_pair_progress_probe_reaches_final_markers -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_native_codegen_pair_progress_probe_reaches_final_markers -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_native_codegen_cache_pairs_probe_emits_counts -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_native_codegen_cache_probe_emits_marker -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_native_codegen_ir_debug_emits_decl_count -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_native_codegen_token_debug_emits_token_count -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_stage2_match -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_stage2_match_fib_runtime_layout -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_fixed_point_minimal_build_progress_matches_stage2_stage3 -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage2_compiler_wasmemit_modules_deterministic -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_section_stability -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_bootstrap_stage1_symbol_stability -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_contracts::test_e2e_selfhost_main_import_only_pipeline -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_wasi_start_signature -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_four_layer::test_e2e_boot04_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_four_layer::test_e2e_bootstrap_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_four_layer::test_v2_11_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_four_layer::test_v2_12_self_hosted_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_four_layer::test_validate_stage2_wasm -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_cli_actual_main_args::test_e2e_selfhost_cli_main_with_args_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_cli_core::test_e2e_selfhost_cli_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_cli_core::test_e2e_selfhost_test_runner_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_doctools_cli_diagnostics::test_e2e_selfhost_cli_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_doctools_cli_diagnostics::test_e2e_selfhost_doctools_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_doctools_cli_diagnostics::test_e2e_selfhost_htmldoc_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_lsp_docs_ops::test_e2e_selfhost_formatter_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_lsp_docs_ops::test_e2e_selfhost_lsp_real_shapes_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_lsp_docs_ops::test_e2e_selfhost_lsp_runtime_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_macro_compiler::test_e2e_selfhost_typeinfer_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_e2e_bootstrap_selfhost_full_deterministic -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_e2e_selfhost_main_full_compile -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_native_codegen_emits_full_const_instruction_bytes -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_native_codegen_emits_aarch64_direct_call_bundle_bytes -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_native_codegen_processes_multiple_ir_instructions -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_native_emit_elf_object_keeps_full_native_payload -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_native_emit_object_keeps_full_native_payload -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_differential::test_native_codegen_emits_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_differential::test_native_emit_object_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_e2e_native_actual_stage23_gap_report_for_representative_entry -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_e2e_native_actual_stage23_gap_report_includes_selfhost_runtime_blockers -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_aarch64_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_chunk_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_function_size_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_host_binary_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_native_host_bundle_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_selfhost_main_native_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_selfhost_main_representative_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_selfhost_native_aarch64_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_selfhost_pipeline_smoke_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_stage1_native_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_stage23_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_e2e_zero_diff_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_native_stage_chain::test_native_codegen_emits_x86_ -- --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    test_e2e_wasm_native_differential_uses_actual_self_regenerated_stage_artifacts -- --exact --ignored --nocapture
  cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_typeinfer_pipeline_bootstrap::test_e2e_bootstrap_ -- --ignored --nocapture
fi

if [[ "$RUN_INCREMENTAL_COMPARE" == "1" ]]; then
  echo ""
  echo "=== Incremental compile compare ==="
  BOOTSTRAP_DIFF_ARTIFACT_ID="$BOOTSTRAP_DIFF_ARTIFACT_ID" \
    cargo test -p lsharp-wasm --test e2e \
    e2e::selfhost_bootstrap_acceptance::test_e2e_incremental_compile_matches_full_compile_fixed_input_set -- --exact --ignored --nocapture
fi

if [[ "$RUN_INCREMENTAL_BENCHMARK" == "1" ]]; then
  echo ""
  echo "=== Incremental compile benchmark ==="
  cargo bench -p lsharp-wasm --bench compiler_pipeline incremental_compile_selfhost
fi

echo ""
echo "Phase 11 fixed input set compile gate complete (no known compile blockers)."
