# ADR: 明示的 process 間 compile artifact cache

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/artifact_cache.rs::ArtifactCache`
- Related: `decisions-legacy-tooling-compile-key.md`, `decisions-legacy-tooling-compile-session.md`

## Context

`CompileCacheKey` は process 間で同一入力を識別できるが、artifact bytes の永続化境界がないまま CLI の既定経路へ
disk cache を接続すると、破損ファイルや古い schema を成功扱いする危険がある。また、compile の既存 output path と
cache root を暗黙に兼用すると、プロジェクト外へ生成物が散在し、呼び出し元の cleanup 責務も曖昧になる。

## Decision

- `ArtifactCache::new(root)` で root を明示した caller だけが process 間 cache を使用する。既定の compile path はこの
  slice では変更しない。
- cache entry は `lsharp-compile-artifact-v1/<graph fingerprint>.artifact` に置き、envelope に artifact schema、compile
  key schema、key fingerprint、payload fingerprint を含める。
- 保存は既存の `write_wasm_artifact` を使い、一時ファイル、file sync、parent directory sync、atomic rename の境界を
  再利用する。
- file 不在、schema/key 不一致、payload fingerprint 不一致は `Ok(None)` の cache miss とし、fresh compile が stale / corrupt
  bytes を成功扱いしない。permission などの filesystem error は error として返す。
- この slice は compile/session へ自動接続しない。target/runtime validation、CLI opt-in、cache eviction、selfhost/native
  persistence は後続タスクで閉じる。

## Evidence

- RED: `ArtifactCache` 未実装時に roundtrip、key miss、corrupt envelope の 3 test が compile error となった。
- GREEN: `cargo test -p lsharp-tooling artifact_cache -- --nocapture` (`3 passed; 0 failed`)。
- payload 変更時の fingerprint mismatch、target 差分、atomic writer の一時 file 非残留を focused test で確認した。

## Consequences

明示 root の artifact cache を後続の `CompileSession` integration へ安全に接続できる。既定 compile 挙動と native stage0
の成功経路は変更していないため、`LEGACY-MODULE-01` は未完了のまま残る。
