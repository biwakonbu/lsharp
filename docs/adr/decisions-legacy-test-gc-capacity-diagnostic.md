# ADR: WASI GC capacity failure diagnostic

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-RUNTIME-01` / `LEGACY-TEST-01` / imp-02 + imp-03 / WASI core

## Context

WASI の `memory.grow` 失敗は、Wasmtime の generic `unreachable` trap として表面化していた。安全に停止することは runtime limit lane で確認できていたが、利用者が容量上限を識別できる stable error code がなく、`imp-02` の診断契約と `imp-03` の memory boundary を閉じられなかった。

## Decision

core WASI の runtime runner で、`unreachable` trap が内部 helper の固定 index（`__alloc` = 10、`root_push` = 22）から発生した場合だけ、元の trap を保持した `LS4002: GC / linear memory の容量上限に達しました` へ分類する。ユーザー関数の trap は分類せず、Component Model / HTTP / native stage0 の実行経路はこの ADR の scope 外として generic trap boundary を維持する。

`LS4002` は driver の error-code table と `docs/guides/error-reference.md` に登録し、MCP lookup が同じ table を参照する契約を保つ。

## Evidence

- Classifier unit: `wasi_runner::tests::test_classify_wasi_runtime_failure_maps_allocator_capacity_trap`、`...maps_root_capacity_trap`、`...preserves_other_traps` → 3 passed
- Stable diagnostic E2E: `test_e2e_alloc_memory_grow_failure_reports_ls4002` → 1 passed
- Safety regression: `test_e2e_alloc_memory_grow_failure_does_not_return_out_of_bounds_address` → 1 passed
- Runtime lane: `scripts/ci/test-runtime-limits.sh` → 8 exact E2E passed
- Error reference: driver table and `docs/guides/error-reference.md` both contain `LS4002`

## Consequences

- core WASI の allocator/root capacity failure は、元の Wasmtime backtrace を失わず stable `LS4002` で識別できる。
- 固定 helper index に依存するため、WASI helper ordering を変更する場合は classifier unit/E2E とこの ADR を同時に更新する。
- size class、precise sentinel、HTTP/component/native parity、native stage0 GC gate、recursive runtime limit は残件として `[~]` を維持する。
