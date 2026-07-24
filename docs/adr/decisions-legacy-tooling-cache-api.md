# ADR: lsharp-tooling の cache compile API

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs`
- Related: `decisions-legacy-module-cache-api.md`, `decisions-legacy-module-deps-key.md`

## Context

`lsharp-tooling::compile_file_with_backend` は公開 compile 導線だが、caller が
`CompilationCache` を保持できず、同一 process で複数回 compile する host session が lsharp-ir の
incremental path を明示的に選べなかった。CLI の一回実行互換性を保ったまま、cache を所有する境界が必要だった。

## Decision

- `compile_file_with_backend_and_cache(..., cache)` を公開する。
- 既存 `compile_file_with_backend` は一時 `CompilationCache` を生成して新 API へ委譲する。
- file import を含む Linear compile は `lsharp_ir::compile_multi_file_with_cache` へ接続する。
- WasmGC の file import 拒否と single-file compile の既存挙動は維持する。
- driver の process-wide session 保持、disk persistence、selfhost/native stage0 parity は後続 C-2 とする。

## Evidence

- RED: `test_compile_file_with_backend_and_cache_reuses_multi_file_cache` は API 未実装時に
  コンパイル失敗した。
- GREEN: 同テストは tooling 層で cold compile 後に 2 module cache を確認し、warm compile の
  Wasm artifact bytes と cache scope の parity を確認する。
- 新規 focused test、clippy、rustfmt、docs audit を通過した。crate 全体 106 tests は 104 pass / 2
  pre-existing metadata `LS2005` vacuous failures（`metadata_test.rs`）で、今回の compile 差分とは無関係である。

## Residual risk

これは caller-owned cache の tooling 境界を追加した verified partial slice であり、既定 CLI driver
が複数 command を跨いで session cache を保持するわけではない。依存 SCC key の統合、process 間永続化、
selfhost/native stage0 parity、`LEGACY-MODULE-01` aggregate 完了条件は未完了である。
