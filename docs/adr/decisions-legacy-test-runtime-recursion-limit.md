# ADR: WASI runtime recursion stack limit lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / Rust-oracle WASI runtime

## Context

imp-07 は、自己再帰を段階的に深くし、Wasmtime の stack 制限での失敗挙動を E2E に固定することを要求している。既存の型推論 depth/occurs-check と通常の recursive-function E2E は、この runtime call-stack boundary を検証していなかった。

## Decision

専用の E2E helper が `max_wasm_stack = 64 KiB` の Wasmtime engine を構築する。fixture は非末尾再帰として、depth 0 / 32 / 128 の成功出力と depth 100,000 の runtime trap を同一 test で確認する。failure contract は Wasmtime の全文に依存せず、`stack` と `trap` を含むことだけを固定する。通常の公開 WASI runner の stack 設定、コンパイラの tail-recursion lowering、Component/HTTP/native 経路は変更しない。

## Evidence

- RED: helper 未実装時に `runtime_recursion_limits` integration test が unresolved import で失敗。
- GREEN: `test_e2e_runtime_recursion_stack_limit_reports_trap` → 1 passed。
- Lane contract: `bash scripts/ci/test-runtime-recursion-limits-contract.sh` → passed。
- Exact lane: `scripts/ci/test-runtime-recursion-limits.sh` → 1 exact E2E passed。
- Validation record: `docs/development/validation/runtime-recursion-limit.md`

## Consequences

- runtime recursion limit の failure boundary を、tail-recursion optimization と混同せず再実行できる。
- exact frame threshold、native stage0/Linux evidence、Component/HTTP parity、product-level configurable limit は残件として維持する。
