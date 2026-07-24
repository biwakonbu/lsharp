# ADR: multi-file cache compile の明示 API

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: Rust host の `lsharp-ir` multi-file compile API
- Related: `decisions-legacy-module-scc-inference.md`

## Context

`CompilationCache` と `compile_multi_file_incremental` は既に LSP / benchmark 側で実装済みだが、
名前だけでは CLI compile が cache を使う入口か判断しにくかった。設計の C-2 は
`compile_multi_file_with_cache(entry_file, cache)` を公開し、既存 `compile_multi_file` の互換性を
維持する契約を求めている。

## Decision

- `compile_multi_file_with_cache` を公開し、既存の incremental compile 実装へ委譲する。
- `compile_multi_file_incremental` は既存 caller のため残す。
- fresh compile (`compile_multi_file`)、cold cache、warm cache の IR parity を同一 fixture で固定する。
- warm cache では型推論を再実行しないことを tracker で確認する。
- CLI driver の既定経路、依存 SCC key、process 間永続化、selfhost cache は後続 C-2 とする。

## Evidence

- RED: `test_compile_multi_file_with_cache_matches_fresh_and_warm_compile` は API 未実装時に
  コンパイル失敗した。
- GREEN: 同テストは cold cache の 2 module compile、cache entry 2 件、warm IR parity、warm 型推論 0 件を確認する。
- lsharp-ir focused test、clippy、rustfmt、docs audit を通過した。

## Residual risk

これは既存 process 内 incremental 実装を明示名で公開した verified partial slice である。CLI driver
の利用、依存 SCC を含む cache key、source override の SCC 統合、disk persistence、selfhost/native
stage0 parity は未完了で、`LEGACY-MODULE-01` / C-2 の aggregate 完了条件には到達していない。
