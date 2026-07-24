# ADR: artifact key と module dependency key の責務境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-tooling/src/compile.rs`, `crates/lsharp-ir/src/cache.rs`
- Related: `decisions-legacy-tooling-compile-key.md`, `imp-04-module-system-strengthening.md`

## Context

imp-04 は module の公開 type surface を依存 key に含める設計と、process 間 artifact cache の identity を同じ方向へ拡張する設計を
併記していた。しかし Wasm artifact の bytes は依存 module の実装変更にも変わるため、public surface だけを process 間 artifact key に
使うと、型が同じでも古い bytes を再利用する stale hit になる。

## Decision

- process 間の `CompileCacheKey` は artifact identity として、SCC 解決後の全 module の module name、canonical path、source fingerprint、entry、target、backend、compiler version/schema を deterministic manifest に含める。
- process 内の `ModuleCacheEntry::deps_key` は direct dependency の公開 `ModuleTypeSurface` を使う incremental inference/lowering 境界として維持する。実装だけの変更では downstream の public surface key を維持し、公開型変更では再解析する。
- `deps_key` を process 間 artifact key へ流用しない。artifact bytes の意味論を保持するため、実装変更は source manifest による artifact miss とする。
- SCC、module graph、target/backend の変更は manifest の identity を変える。graph の再解決や cache hit の結果が deterministic であることを focused test で固定する。

## Evidence

- `test_compile_cache_key_changes_when_imported_source_changes` は imported implementation の変更を miss にする。
- `test_compile_cache_key_changes_when_import_graph_changes` は同じ helper shape でも `Lib` → `Alt` の dependency graph 変更を miss にする。
- `test_compile_cache_key_includes_target_and_backend` は output target/backend の分離を固定し、`test_compile_multi_file_with_cache_tracks_dependency_surface_key` は module-level public surface key の実装変更/型変更境界を固定する。
- focused tooling key test、既存 artifact cache/runtime test、clippy、rustfmt、docs audit を通過させる。Native stage0、Linux x86_64、公開 command aggregate はこの ADR の evidence scope 外で未完了とする。

## Consequences

process 間 cache は保守的に miss するが stale Wasm を成功扱いしない。process 内 cache は public surface の再解析削減を継続できる。
public-only artifact reuse、automatic eviction、Native/selfhost persistence、`LEGACY-MODULE-01` aggregate 完了は別タスクとして残る。
