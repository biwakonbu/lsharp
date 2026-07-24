# ADR: selfhost rooting 規約と shadowed root_set guard

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 Phase B-4 / Rust-oracle Wasm runtime

## Context

`imp-07` の初期記述には、selfhost の heap 値を allocating call の前後で保持する規約が未文書化と残っていた。その後、memory-management roadmap に規約を追加し、lexical shadowing 中の `root_set` 更新を検出する actual Wasm guard が実装されている。設計文書の状態と実証を同じ current truth に揃える必要がある。

## Decision

selfhost の heap 値 (`Vector` / `String` / record など) は、allocating call を跨ぐ前に `root_push` し、最後の使用後に `root_pop` する。shadowing や loop 内で slot の値を更新するときは、slot を先に確保し、allocating value の評価後に `root_set` する。

この ADR では、shadowed binding の旧値 `42` を allocating `vector-push` の新値 `7` へ `root_set` し、`root_pop` 後も `7` を観測できる Rust/Wasm runtime slice を verified とする。全 selfhost source の static lint、GC stress mode、Mac/Linux native stage0 は別の残件である。

## Evidence

- Contract: `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs::test_e2e_selfhost_root_set_preserves_shadowed_slot_during_allocating_value`
- Gate: `cargo test -p lsharp-wasm --test e2e e2e::selfhost_cli_core::test_e2e_selfhost_root_set_preserves_shadowed_slot_during_allocating_value -- --exact --nocapture --test-threads=1`
- Result: 1 passed; output `7\n`
- Rule source: `docs/development/planning/memory-management-roadmap.md` の `Selfhost rooting 規約`

## Consequences

- 規約、guard、残る native/static-lint boundary が imp-07 から追跡できる。
- `LEGACY-TEST-01` aggregate、GC stress、Linux x86_64 / Mac Apple Silicon native evidence は未完了のまま維持する。
