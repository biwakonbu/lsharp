# ADR: CLI の明示 artifact cache byte budget

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/artifact_cache.rs::ArtifactCache::trim_to_bytes`, `crates/lsharp-driver/src/main.rs`
- Related: `decisions-legacy-tooling-cli-cache-maintenance.md`, `decisions-legacy-tooling-cache-maintenance.md`

## Context

entry 数だけの上限では、個々の artifact envelope が大きい場合に cache directory の容量を予測できない。既存の bounded
maintenance は opaque key の辞書順で entry を削除するため、同じ deterministic policy を byte budget にも適用し、mtime/LRU を
暗黙に導入せずに caller が保存容量を制御できる必要がある。

## Decision

- `ArtifactCache::trim_to_bytes(max_bytes)` は `.artifact` file の envelope size 合計を計測し、上限を超える間、file name が辞書順で
  小さい entry から削除する。
- directory 未作成は no-op とし、schema 外の file と非 `.artifact` entry は変更しない。単一 artifact が上限を超える場合はその entry を
  削除する。
- `compile` / `build` に `--artifact-cache-max-bytes <N>` を追加し、`--artifact-cache-dir` との併用を必須にする。compile/build 成功後に
  entry limit、続けて byte limit を適用する。
- embedded component delegation では host-only flag として拒否し、既定 cache location、Native/selfhost persistence、mtime/LRU、
  automatic policy は導入しない。

## Evidence

- RED: `trim_to_bytes` 未実装時に byte trim test が `no method named trim_to_bytes` で compile error となった。CLI field/helper 未実装時も
  `Command::Build` field と validation signature の compile error を確認した。
- GREEN: `cargo test -p lsharp-tooling test_artifact_cache_trim_to_bytes -- --nocapture --test-threads=1`
  (`1 passed; 0 failed`)。
- `cargo test -p lsharp-driver test_cli_compile_artifact_cache -- --nocapture --test-threads=1` (`5 passed; 0 failed`) と
  `cargo test -p lsharp-driver test_maintain_artifact_cache -- --nocapture --test-threads=1` (`1 passed; 0 failed`)。
- 2 artifact を entry limit 1 へ trim した後、残存 envelope の size 未満を byte budget に指定して全 artifact が削除される helper test を通過した。
- CLI manual smoke で `fib` (`5375` bytes) を budget `10000` に置き、続けて `factorial` (`5388` bytes) を budget `1` で `build` した。
  それぞれ exit `0`、artifact count は `1` から `0` になった。byte budget 単独指定は exit code `1` と `--artifact-cache-dir` 併用要求を返した。

## Consequences

caller は entry 数と envelope bytes の両方を明示的に制限できる。辞書順 policy は deterministic だが recency-aware ではなく、payload
圧縮・LRU・project-wide byte budget は別 task とする。`LEGACY-MODULE-01`、native 2 target、selfhost persistence は未完了のまま残る。
