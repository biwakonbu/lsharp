# ADR: v0.2 runtime free-list のサイズクラス化

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: I-04 / imp-03 Phase B-2 / Rust core-WASI runtime

## Context

現行 allocator は解放 block を単一 free-list に積み、`__alloc` が first-fit の線形走査を
行っていた。長寿命 workload では allocation size が小さくても list 全体を走査し得る。
一方、既存の `heap_ptr`、object table、GC telemetry の値と Wasm linear-memory ABI は
変更せず、段階的に再利用経路だけを改善する必要がある。

## Decision

free-list を次の 8 class に分割する。

- 16 / 32 / 64 / 128 / 256 / 512 / 1024 bytes
- 1024 bytes を超える oversize fallback

各 class は専用 mutable global を head とし、解放済み block の payload 先頭 8 bytes を
`next: i32` と `capacity: i32` の singly-linked node として使う。small class は該当 head
から O(1) pop し、oversize だけは capacity を比較する first-fit scan にフォールバックする。
scan step は internal export `__lsharp_gc_free_list_scan_steps` で観測する。bump allocation
は class 上限へ丸めず、従来の aligned requested size を physical capacity として保存し、
`heap_ptr` / allocation telemetry の ABI を維持する。

GC sweep は object table entry の capacity (`offset 12`) を必ず読み直し、その値で class を
選ぶ。古い table-based free-list grow/append 命令列は ABI 差分を抑えるため残すが、新しい
class node を登録した後は実行しない。

## Evidence

- `runtime_allocator_size_classes::test_e2e_runtime_allocator_reuses_small_blocks_without_linear_scan`
  — repeated `_start` で free block を回収し、small class reuse の scan step が 0。
- `runtime_allocator_size_classes::test_e2e_runtime_allocator_uses_oversize_fallback_scan`
  — oversize block を回収・再利用し、fallback scan step が正数。
- `CARGO_PROFILE_DEV_DEBUG=0 cargo test -p lsharp-wasm --test e2e runtime_allocator_size_classes -- --nocapture`
  — 2 passed。
- `cargo check -p lsharp-wasm --tests`、既存の allocation-alignment / runtime-telemetry focused test
  — pass。

## Consequences and remaining work

- small allocation の free-list 再利用は list 全体を走査しない。
- oversize class は意図的に線形探索を残すため、I-04 の性能 exit criterion 全体は未完了。
- I-03 dynamic table/root grow、HTTP/component parity、Mac Apple Silicon / Linux x86_64 native
  stage0、CI artifact の scan-step 集計、D-10 sentinel precise discrimination はこの ADR の
  scope 外の残件であり、TODO の `[~]` を維持する。
