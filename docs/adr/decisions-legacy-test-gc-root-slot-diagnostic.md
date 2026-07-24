# ADR: WASI GC root slot invariant diagnostic

- Status: Accepted (verified diagnostic slice; compiler fix remains open)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / `GC-05` / imp-02 runtime diagnostics

## Context

長寿命 selfhost REPL telemetry の RED は、host と Wasmtime の stack を 128 MiB に拡大しても
`_start` の Wasm backtrace `<wasm function 24>` で停止した。WASI function 24 は `root_set` であり、
function 10 (`__alloc`) / 22 (`root_push`) の容量超過を表す `LS4002` と同じ分類にすると、
compiler-side safe-point spill の slot 不整合を容量不足と誤認してしまう。

## Decision

core WASI runner は `unreachable` trap が function 24 (`root_set`) から発生した場合、元の backtrace
を保持した `LS4003: GC root slot の整合性が壊れました` として分類する。LS4002 は allocator/root-stack
capacity (`__alloc` / `root_push`) に限定し、ユーザー関数や Component Model の trap は generic boundary
のまま維持する。

`LS4003` は driver の error-code table と `docs/guides/error-reference.md` に登録し、MCP の
error-code lookup が同じ table を参照する契約を保つ。これは failure を観測可能にする診断 slice であり、
root slot の push/set/pop lifetime や compiler-side safe-point spill の修正完了を意味しない。

## Evidence

- RED: `cargo test -p lsharp-wasm wasi_runner::tests::test_classify_wasi_runtime_failure_maps_root_slot_invariant_trap -- --exact --nocapture` → function 24 が未分類の generic trap。
- GREEN: `cargo test -p lsharp-wasm wasi_runner::tests::test_classify_wasi_runtime_failure -- --nocapture` → 5 passed。`unreachable` trap 文言が無い実際の Wasmtime backtraceも含め、`LS4003` と元の backtrace を保持し、function 27 の user trap は generic のまま維持。
- Contract: `cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_error_reference_doc_mentions_all_mcp_error_codes -- --exact --nocapture` → 1 passed。
- Stateful failure boundary: in-session REPL telemetry exact test は 128 MiB stack 拡大後も 452.67 秒で function 24 に到達した。これは compiler-side root slot ledger の次の RED として扱う。
- Runtime ledger RED: `test_root_set_invalid_slot_records_failure_ledger_before_trap` は、failure ledger の export が存在しないため `get_global(...).unwrap()` で失敗した。
- Runtime ledger GREEN: 同じ fixtureで Wasmtime trap 前に `__lsharp_root_slot_failure_slot=0`、`__lsharp_root_slot_failure_top=0`、`__lsharp_root_slot_failure_count=1` を観測できるようになった。これは runtime の観測可能性だけを追加し、compiler-side safe-point ledger の修正完了を意味しない。

## Consequences

runtime 容量不足と root slot invariant failure を machine-readable に区別できる。次の実装 task は
compiler の safe-point ごとに `root_push` が返す slot、`root_set` の更新対象、`root_pop` の lexical lifetime
を記録する ledger/contract test を追加し、REPL と LSP の stateful telemetry を再び GREEN にすることである。
Mac Apple Silicon / Linux x86_64 native stage0、Component parity、全 selfhost GC-safe-point 列挙はこの ADR の完了条件に含めない。

## 2026-07-25 compiler safe-point ledger slice

`crates/lsharp-ir/src/root_lifetime.rs` に、lowering が生成した `Instruction::Call(14/15/16)` を対象とする
抽象 ledger を追加した。ledger は local に保存された slot identity を追跡し、active slot が無い
`root_pop`、pop 済み slot の `root_set`、分岐ごとの root depth 不一致、関数 exit 時の残留 slot を検出する。
`Lower::lower_program_with_expr_types` は codegen 前に `validate_module` を実行し、違反は `LS3003`
(`root-lifetime-invariant`) として fail-closed にする。`LS3003` は driver error table と error reference にも登録した。

この RED で selfhost Compiler.ls の `compile-user-call-arg-instrs-step-with-source`、
`compile-recordupdate-with-ftable`、`register-adt-variants`、`compile-let-with-ftable-impl-body-impl-3`
の push/pop 不整合を検出し、対応する pop の不足・過剰を修正した。これは compiler の生成物を
runtime failure ledger の slot/top/count と対応付けるための最初の fail-closed sliceである。

Evidence: `cargo test -p lsharp-ir --lib`（255 passed）、
`cargo test -p lsharp-wasm --test e2e e2e::selfhost_rooting_parity::test_e2e_selfhost_compiler_root_lifetime_ledger_tracks_nested_map_safe_point -- --nocapture`
（1 passed）。rooting parity の並列 full filter は multifile stateful fixtureで stack overflow となったため、
この slice の全公開 surface / native runtime gate の GREEN とは扱わない。

残る作業は REPL/LSP stateful native/runtime gate、Mac Apple Silicon と Linux x86_64 の current-source
stage0、indirect branch/call を含む ledger coverage、runtime ledger との actual slot/top/count differential である。
