# ADR: WASI root stack 32768-slot growth contract

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / Rust-oracle Wasm runtime

## Context

imp-03 / imp-07 は、初期 32768 slot の root stack が満杯になったときの限界値を、GC の heap payload と root metadata の移動を分けて検証することを要求している。実装と E2E fixture は既に存在するが、imp-07 の verified slice と ADR の対応が抜けていた。

## Decision

WASI actual runtime では、32768 slot を超える root push を root stack の倍増・移動契約として固定する。`root_set` と `root_pop` は移動後の table base を参照し、既存の root value を壊さないことを別 fixture で確認する。この ADR の scope は Rust-oracle Wasm runtime に限定し、HTTP/component/native stage0 の parity は別残件とする。

## Evidence

- Growth contract: `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs::test_e2e_runtime_root_stack_grows_past_initial_capacity`
- API preservation contract: `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs::test_e2e_runtime_root_stack_growth_preserves_root_api`
- Gate: `cargo test -p lsharp-wasm --test e2e e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_grows_past_initial_capacity -- --exact --nocapture --test-threads=1` → 1 passed
- Gate: `cargo test -p lsharp-wasm --test e2e e2e::runtime_allocator_closures::test_e2e_runtime_root_stack_growth_preserves_root_api -- --exact --nocapture --test-threads=1` → 1 passed
- Verified values: `root_stack_top=32769`、`root_stack_capacity=65536`、移動後の `root_set/root_pop` output `0`, `32768`, `42`

## Consequences

- 32768 root slot の WASI limit evidence を imp-07 から追跡できる。
- object table 4096、free-list 4096、root stack 32768 の Rust/WASI capacity slices は個別に verified とする。
- allocation failure の上限診断、HTTP/component parity、native stage0 gate、rooting stress/static lint は `LEGACY-TEST-01` の残件として維持する。
