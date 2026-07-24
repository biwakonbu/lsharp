# ADR: module cache の dependency surface key

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: Rust host の `lsharp-ir::ModuleCacheEntry`
- Related: `decisions-legacy-module-cache-api.md`, `decisions-legacy-module-cache-scope.md`

## Context

module 自身の source fingerprint だけでは、依存 module の公開型が変わったときに downstream
entry を再利用してしまう契約を cache entry から読み取れない。現在の process 内 incremental path
には surface 比較があるが、再利用理由を entry に保存する明示 key が必要だった。

## Decision

- `ModuleCacheEntry` に `deps_key: u64` を保存し、公開 getter を提供する。
- key は direct dependency 名と、依存 module の公開 `TypeScheme` / private 名の surface key を
  module 名順に hash したものとする。
- 実装だけが変わり公開 surface が同じ場合は downstream key を維持し、公開 surface が変わった場合は
  downstream の cache hit を拒否して再推論する。
- SCC 全体を一つの key にする設計、CLI driver の既定経路、process 間永続化、selfhost/native stage0
  parity は後続 C-2 とする。

## Evidence

- RED: `test_compile_multi_file_with_cache_tracks_dependency_surface_key` は `deps_key` 未実装時に
  コンパイル失敗した。
- GREEN: 同テストは dependency implementation-only change で key が維持され、`Int` → `Bool` の
  公開型変更で key が変わることを確認する。
- 既存の dependent reinfer skip/signature-change tests、cache focused tests、lsharp-ir 回帰（既知の
  Formatter blocker を除外）、clippy、rustfmt、docs audit を通過した。

## Residual risk

これは direct dependency の公開 surface を key 化した verified partial slice であり、SCC 内の型結果を
まとめた key ではない。`LEGACY-MODULE-01` / C-2 の aggregate 完了条件（Formatter SCC、CLI/native
public command、両対応 target、selfhost parity）には未到達である。
