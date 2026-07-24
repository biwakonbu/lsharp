# ADR: Wasm runtime GC leak / limit の検証済み slice

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / Rust-oracle Wasm runtime

## Context

imp-07 は、不要化した allocation を collect した後の live 数安定性と、初期容量を超える GC metadata の扱いを限界値テストで固定することを求めている。既存の runtime telemetry helper は `gc_freed_count`、`gc_live_alloc_count`、free-list 件数、heap usage の反復値を観測できる。

## Decision

既存の actual Wasm E2E を、GC leak / limit の verified slice として正本に記録する。

- 4097 個の unrooted allocation が collect 後に全回収され、live allocation が 0 になる。
- collect 後の free-list を次の実行で再利用でき、2 回目も live allocation が 0 になる。
- 10 回の repeated-start churn で heap usage の tail が plateau し、collector が走って unrooted allocation を回収する。

これらは Rust driver で生成した Wasm と Wasmtime の actual runtime telemetry に対する契約である。selfhost native stage0、両 supported target、GC の全公開 surface、rooting stress の完了証拠には拡大解釈しない。

## Evidence

- `e2e::runtime_allocator_closures::test_e2e_runtime_free_list_grows_past_initial_capacity`
- `e2e::runtime_allocator_closures::test_e2e_runtime_free_list_growth_reuses_moved_entries`
- `e2e::runtime_allocator_closures::test_e2e_runtime_collector_reuses_unrooted_allocations_across_repeated_start_series`
- Gate: `cargo test -p lsharp-wasm --test e2e <test-filter> -- --test-threads=1`
- Result: 各 1 passed、失敗 0

## Consequences

- GC leak / free-list limit の actual runtime evidence が current docs から追跡できる。
- GC slot 32768、runtime memory.grow 上限、rooting stress、native 2 target gate、`LEGACY-TEST-01` aggregate は残件のまま維持する。
