# ADR: process 間 compile artifact cache の identity key

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs::CompileCacheKey`
- Related: `decisions-legacy-tooling-compile-session.md`, `decisions-legacy-module-deps-key.md`

## Context

`CompileSession` は process 内の `CompilationCache` lifetime を閉じたが、process 間で artifact を再利用するには、
entry file だけではなく、import 先 source、解決された module path、output target、value backend、compiler/schema の
変更を identity に含める必要がある。identity が不十分なまま disk cache を導入すると、古い Wasm を成功扱いする。

## Decision

- `CompileCacheKey::from_entry` は SCC 対応 module graph を解決し、全 module の canonical path と
  `SourceFingerprint` を deterministic な順序で manifest 化する。
- manifest には entry identity、`CompileTarget`、`CompileBackend`、`CARGO_PKG_VERSION`、
  `COMPILE_CACHE_KEY_SCHEMA` (`lsharp-compile-key-v1`) を含める。
- key の実体は manifest の SHA-256 `SourceFingerprint` とし、target/backend も構造体 fields として保持する。
- この slice では artifact の disk read/write は行わない。schema または compiler artifact contract を変える場合は
  schema version を更新して旧 artifact を採用しない。

## Evidence

- RED: `test_compile_cache_key_changes_when_imported_source_changes` と
  `test_compile_cache_key_includes_target_and_backend` は `CompileCacheKey` 未実装時に compile error となった。
- GREEN: import 先 `Lib.ls` の一文字変更で key が変わり、同一 source でも Preview1 / Component / WasmGC の
  target/backend 差分が別 key になることを確認した。
- `cargo test -p lsharp-tooling test_compile_cache_key -- --nocapture`: `2 passed; 0 failed`。
- `cargo clippy -p lsharp-tooling --lib --tests -- -D warnings`、対象 file の rustfmt check、`git diff --check` が成功した。

## Consequences

次の persistence slice はこの key を cache filename/manifest の identity に利用できる。現時点では既存 compile の
artifact bytes、CLI process 間 cache、selfhost/native stage0 の挙動を変更しないため、`LEGACY-MODULE-01` は未完了のまま残る。
