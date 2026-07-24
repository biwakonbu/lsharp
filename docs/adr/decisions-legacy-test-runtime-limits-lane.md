# ADR: WASI runtime limit verification lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / `LEGACY-RUNTIME-01` partial

## Context

GC object table、free-list、root stack、collector、`memory.grow` failure の E2E は同じ `runtime_allocator_closures` integration test に分散していた。個別 test の存在だけでは、次の runtime limit contract を一回の再現可能な gate として実行しにくい。また、`memory.grow` failure は安全な trap 境界までであり、imp-03 が要求する stable `LS4002` 診断とは区別する必要がある。

## Decision

[`scripts/ci/test-runtime-limits.sh`](../../scripts/ci/test-runtime-limits.sh) を、WASI actual runtime の capacity / collector / allocation-failure lane とする。引数なしで次の 7 exact E2E を直列実行し、`--dry-run` は command 集合だけを出力する。

- allocation `memory.grow` failure が out-of-bounds address を返さない
- object table の初期 4096 超過
- free-list の初期 4096 超過
- 移動後 free-list entry の再利用
- root stack の初期 32768 超過
- root table 移動後の `root_set` / `root_pop` preservation
- repeated-start の unrooted allocation reuse

この lane は Rust-oracle / WASI runtime に限定する。generic trap の safety evidence を `LS4002` stable diagnostic、HTTP/component parity、native stage0 evidence へ拡大解釈しない。

## Evidence

- Contract: `bash scripts/ci/test-runtime-limits-contract.sh` → `runtime limit lane contract passed`
- Wide lane: `scripts/ci/test-runtime-limits.sh` → 7 tests passed
- Exact gate: `cargo test -p lsharp-wasm --test e2e ... -- --exact --nocapture --test-threads=1` を 7 fixture へ適用
- Runtime result: memory.grow failure、object/free-list/root capacity、root API preservation、repeated-start collector reuse が各 1 passed
- Script syntax: `bash -n scripts/ci/test-runtime-limits.sh scripts/ci/test-runtime-limits-contract.sh`

## Consequences

- runtime limit の代表 fixture を一つのローカル gate で再実行できる。
- `LEGACY-TEST-01` の GC leak/limit verified slice が capacity と failure-boundary の evidence まで追跡可能になる。
- stable `LS4002` の trap/diagnostic contract、free-list size class、precise sentinel、HTTP/component parity、native stage0 GC gate、recursion-depth limit は残件として維持する。
