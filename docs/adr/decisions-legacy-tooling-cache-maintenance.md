# ADR: 明示 artifact cache の bounded maintenance

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/artifact_cache.rs::ArtifactCache::trim_to_entries`
- Related: `decisions-legacy-tooling-compile-artifact-cache.md`, `decisions-legacy-tooling-cli-artifact-cache.md`, `decisions-legacy-tooling-cache-runtime.md`

## Context

process 間 artifact cache は明示 root へ成果物を蓄積するが、cache directory の寿命や容量を compile path が勝手に管理すると、
CLI の既定挙動や caller の保存方針を変えてしまう。一方、無制限に `.artifact` が増え続けると明示 root を長期利用する tooling の
運用境界がない。fingerprint は opaque な SHA-256 であり、file の mtime を LRU と解釈する共通契約もまだ存在しない。

## Decision

- `ArtifactCache::trim_to_entries(max_entries)` を明示 root の maintenance API として提供する。
- `.artifact` の file name を辞書順に並べ、上限を超えた分の辞書順最小 entry を削除する。これにより opaque key に時間的意味を
  仮定せず、同じ directory に対する選択を deterministic にする。
- cache schema directory が存在しない場合は no-op (`0`) とし、schema directory 内の非 `.artifact` file、schema 外の file、既定
  compile path は変更しない。
- CLI の自動 eviction、mtime/LRU policy、default cache location、Native/selfhost compiler への接続はこの slice に含めず、caller
  が明示的に API を呼ぶ境界を維持する。

## Evidence

- RED: `trim_to_entries` 未実装時に bounded maintenance test が `no method named trim_to_entries` で compile error となった。
- GREEN: `cargo test -p lsharp-tooling test_artifact_cache_trim_to_entries_removes_deterministic_lowest_keys -- --nocapture --test-threads=1`
  (`1 passed; 0 failed`)。
- test で 3 entry を 1 entry へ trim し、辞書順最小 2 entry の削除、残存 payload、非 `.artifact` metadata の保持、未作成 directory の no-op を確認する。

## Consequences

explicit cache root の disk growth に bounded maintenance の契約を追加できる。選択は deterministic だが recency-aware ではないため、
実利用で LRU や byte budget が必要になった場合は別 ADR と互換性を検討する。`LEGACY-MODULE-01`、CLI 自動 policy、native 2 target、
selfhost persistence は未完了のまま残る。
