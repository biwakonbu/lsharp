# ADR: Mac Native executable の atomic link output

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-tooling::native::compile_native_executable` / `aarch64-apple-darwin`

## Context

Wasm output は atomic artifact writer へ移行した一方、Mac native backend は `cc -o` の destination
へ linker が直接書いていた。link failure や process interruption の間に destination が途中の
executable へ置き換わると、次の run が壊れた artifact を実行する。既存 destination を保持したまま
link 成功だけを公開する境界が必要だった。

## Decision

- destination と同じ親 directory に process-unique な `.destination.tmp-*` path を作る。
- `cc -o` は temporary output へ向け、成功した場合のみ `rename` で destination を atomic に置換する。
- linker 起動 failure、non-zero status、rename failure では assembly と temporary executable を cleanup
  し、destination を変更しない。
- この ADR は `aarch64-apple-darwin` の既存 native backend に限定する。Linux x86_64 native backend、
  selfhost stage0、release bundle provenance は別の境界として残す。

## Evidence

- Unit: `native::tests::native_output_temp_path_is_a_unique_sibling`
- Failure contract: `native::tests::native_link_failure_cleans_temporary_output_before_returning`
- Runtime: `compile::tests::test_compile_file_native_target_*` 8 tests passed and executed generated
  binaries on Mac Apple Silicon.

## Residual risk

durable fsync、source commit/fingerprint manifest、Linux x86_64 native implementation、selfhost stage0
release/rollback、external release bundle は未完了である。atomic rename の成功を cross-target release
readiness や `LEGACY-BOOT-01` 完了へ拡大解釈しない。
