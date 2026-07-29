# ADR: v0.2 evidence coverage count と cases の整合 boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: canonical `SamplingPlan`、Rust source adapter、version 1 manifest input、公開 `lsharp validate`、Rust MCP `lsharp_validate`
- Related: `EC-M2-02` / `EC-M2-03`、`docs/adr/decisions-v0.2-native-validation-evidence-negative-coverage-count.md`

## Context

`coverage` bucket は実行した cases の分類を表すが、これまで Rust canonical model と source/manifest
入力境界で bucket count の合計を `cases` と照合していなかった。そのため、実行件数より少ない、または
多い coverage を含む evidence が graph に登録され、coverage の存在だけを assurance と誤解できた。
一方、既存の observational evidence には `coverage` を省略する入力もあるため、入力を一律に必須化する
と後方互換性を壊す。

## Decision

- `coverage` は互換性のため省略可能とする。bucket が一つ以上宣言されている場合、count の合計は
  `cases` と完全一致する必要がある。bucket count は checked-add で合計し、表現範囲を超える合計は
  `CoverageCountOverflow` として fail-closed にする。bucket 名の空白検査は既存 policy のまま維持する。
- canonical `SamplingPlan::validate_required_fields` がこの invariant を検査し、graph 登録前に
  `EvidenceValidationError::CoverageCountMismatch { cases, covered }` または overflow を返す。
- Rust source adapter は同じ条件を graph 登録前に検査し、不一致を `InvalidEvidenceField` の
  `sum=<covered>,cases=<cases>`、overflow を `sum-overflow` として evidence directive/form span 付きで返す。
- version 1 JSON manifest は canonical graph の検査へ接続し、公開 `lsharp validate` は入力エラーとして
  exit `1`、stdout と `--emit-manifest` の成果物を空にする。Rust MCP `lsharp_validate` の
  `manifest` object、JSON string、`manifest_file` route は同じ parser error を `isError: true`、
  `structuredContent` なし、coverage/cases/covered を含む text error として返す。
- selfhost/native producer/runtime と MCP の source/file route の current-source parity はこの sliceでは
  新たな完了とは扱わない。
- coverage の実行生成、generator policy、runtime trace の意味論、未検証 target の artifact/runtime は
  この invariant と分離し、EC-M2-02/03 の未完了境界に残す。

## Evidence

- RED を先に追加し、canonical `SamplingPlan` の不一致、source directive span 付き不一致、manifest
  input、公開 CLI の fail-closed boundary を固定した。
- canonical partition（`positive=2`、`negative=1`、`cases=3`）は受理し、不一致と checked-add overflow
  は拒否するテストを追加した。
- `cargo test -p lsharp-types --test evidence_required_fields -- --nocapture`
- `cargo test -p lsharp-types --test validation_source -- --nocapture`
- `cargo test -p lsharp-types --test validation_input -- --nocapture`
- `LSHARP_EMBED_COMPONENT_PATH=... cargo test -p lsharp-driver --test manifest_input_cli validate_rejects_coverage_total_that_does_not_match_cases_without_output -- --nocapture`
- `LSHARP_EMBED_COMPONENT_PATH=... cargo test -p lsharp-driver coverage_count_mismatch -- --nocapture`

## Boundary and follow-up

これは Rust canonical/source/manifest/CLI/MCP manifest routes の coverage count invariant に限定した
verified partial sliceである。coverage 省略を拒否する契約、generator/trace の実行証跡、MCP source/file
route、selfhost/native stage0 parity、current-source artifact/runtime、Mac Apple Silicon と Linux
x86_64 の matrix、EC-M2-02/03 aggregate は未完了であり、TODO の `[~]` を維持する。
