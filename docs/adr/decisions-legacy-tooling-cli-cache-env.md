# ADR: CLI artifact cache の環境変数 root

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-driver/src/main.rs`
- Related: `decisions-legacy-tooling-cli-artifact-cache.md`, `decisions-legacy-tooling-cli-cache-maintenance.md`,
  `decisions-legacy-tooling-cli-cache-byte-budget.md`

## Context

`--artifact-cache-dir` は project 外へ生成物を散らさないための明示的な filesystem boundary として安全だが、同じ root を
繰り返し指定する host script では argv の重複が残る。暗黙の user-wide cache location を導入せず、呼び出し元が所有する
一時ディレクトリを環境変数で明示できる opt-in が必要だった。

## Decision

- `LSHARP_ARTIFACT_CACHE_DIR` は `--artifact-cache-dir` が指定されていない場合だけ root として解決する。CLI flag が常に優先される。
- 環境変数が未設定なら cache は従来どおり無効で、既定 directory を作成しない。空の環境変数は current directory を意味せず、安定したエラーを返す。
- `--artifact-cache-max-entries` / `--artifact-cache-max-bytes` は解決済み root に対して適用し、root が flag または環境変数のどちらからも得られない場合だけ従来の併用エラーを返す。
- built-in embedded component delegation は compile/build で環境変数が設定されている場合に host Rust path へ残す。これにより guest が環境変数を偶然継承しても、host-only filesystem cache が未実装の guest 成功へ隠れない。
- 外部 `LSHARP_PATH` compiler の delegation、Native target、`emit_ir`、automatic eviction、selfhost/native persistence はこの ADR では変更しない。

## Evidence

- RED: helper と delegation guard を先に追加し、未実装時に `resolve_artifact_cache_dir_from_values` /
  `should_delegate_to_embedded_component_args_with_cache_env` の未定義 compile error を確認した。
- GREEN: env root の precedence / unset / empty value を検証する driver test 4 件と、embedded delegation の env guard test 1 件が通過した。
- `cargo test -p lsharp-driver --bin lsharp -- --nocapture --test-threads=1` は 119 件、既存 artifact cache の entry/byte maintenance と
  delegation regression を含めて全件成功した。`cargo test -p lsharp-tooling test_compile_session_opt_in_artifact_cache_reuses_across_sessions`
  は Wasm bytes/runtime を含む cross-session hit を再確認し、driver clippy (`-D warnings`) と `scripts/audit_docs.sh` も成功した。
- 手動 smoke では `LSHARP_ARTIFACT_CACHE_DIR=<tmp>` の compile を 2 回実行し、同じ root に envelope が 1 件だけ作成され、両 output の
  SHA-256 が一致した。空の環境変数は exit `1` と明示エラーになり、output を生成しなかった。

## Consequences

host script は project 外の共有 cache root を一度だけ環境変数で指定できるが、root の寿命と cleanup は指定した caller の責務である。
未設定時の挙動は変わらず、public cache key の SCC 公開 surface 統合、自動 eviction policy、Native/selfhost compiler への移植は別の未完了 task として残る。
