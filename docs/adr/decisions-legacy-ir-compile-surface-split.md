# ADR: IR compile surface production seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/compile_surface.rs`, `crates/lsharp-ir/src/lib_tests.rs`

## Context

`lsharp-ir/src/lib.rs` は incremental compile orchestration と import visibility、型
surface のキー計算・集約を同じ parent に保持していた。Instruction、model、linker の
seam を分離した後も、`ImportVisibilitySpec`、`ModuleTypeSurface` と関連 helper が
compile pipeline の実装詳細に埋め込まれていた。cache invalidation と multi-file inference
の observable contract を変えずに、compile surface の ownership を分ける必要がある。

## Decision

`ImportVisibilitySpec`、`ModuleTypeSurface`、`type_surface_key`、
`dependency_surface_key`、`push_defn_origins_infer_order`、`collect_import_visibility`、
`collect_import_modules` を `compile_surface.rs` に移動する。`lib.rs` は `mod
compile_surface` と内部 `use` で既存の private/crate-private call path を再利用する。
`ModuleTypeSurface` の fields と surface comparison は `pub(super)` に限定し、既存の
`lsharp_ir` public API は変更しない。

## Evidence

- RED: `mod compile_surface` と内部 import を先に追加し、child file がない状態で focused
  test を実行して `E0583`（`compile_surface.rs` 不在）を確認した。
- GREEN: body を移動後、`test_compile_surface_preserves_import_visibility_aggregation_contract`
  は pass。`(import Lib :only [a b])` と追加 import の union、および module deduplication
  の既存 semantics を固定した。
- `cargo test -p lsharp-ir --lib -- --nocapture`: 288 tests、287 pass / 1 fail。失敗は
  origin/main から継続する `selfhost/src/Tools/Validation/IntentSource.ls` の
  `vector-push-pair-rooted-v3` 未定義であり、本分割の変更起因ではない。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、workspace
  `cargo check --workspace`（専用 target）、対象 Rust 2024 rustfmt、`git diff --check`、
  docs audit は pass。

## Consequences

compile surface の ownership が `compile_surface.rs` に集約され、`lib.rs` は 2139 行から
2016 行へ縮小した。既存の cache key、import visibility、definition ordering、型 surface
comparison semantics と `ModuleTypeSurface` の crate-private API は維持される。残る
compile/lowering orchestration、native/selfhost parity、I-01 / I-08 aggregate は未完了で
あり、`LEGACY-MAINT-01` は `[~]` のまま継続する。
