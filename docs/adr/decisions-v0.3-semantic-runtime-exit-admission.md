# ADR: v0.3 semantic runtime exit admission

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: Rust-oracle/native-stage0 semantic fixture report producers
- Related: [`decisions-v0.3-semantic-runtime-artifact-binding.md`](decisions-v0.3-semantic-runtime-artifact-binding.md)、[`decisions-v0.3-semantic-report-batch-transaction.md`](decisions-v0.3-semantic-report-batch-transaction.md)

## Context

Semantic fixture producers invoke an external Wasmtime boundary after compilation
and artifact validation. Before this slice、a valid fixtureの expected runtime exit
code と実行結果の exit code が異なっていても、producer は observed runtime として
report を書けた。後段の diff は不一致を検出できるが、producer 単体の出力が runtime
evidence として扱われる境界では、期待成功 fixture の実行失敗を成功観測に見せる余地がある。

## Decision

- valid fixture の canonical `expected.runtime.exit_code` が指定されている場合、Rust
  oracle と native stage0 の両 producer は Wasmtime の exit code を実行直後に比較する。
- 不一致は stable な `runtime exit ... does not match expected exit ...` 診断で fail-closed
  にし、report を生成しない。既存の caller-owned root と fixture cleanup semantics は維持する。
- invalid fixture は従来どおり runtime を実行せず、`not-run` report を保持する。
- stdout/stderr の observed 値は既存 schema のまま保持し、この slice では新しい report field や
  provider/runtime fallbackを追加しない。

## Evidence

- RED: 同じ `valid/syntax-basic` fixtureに fake Wasmtime exit `23` を与え、Rust/native とも
  reportを書いて成功終了することを確認した。
- GREEN: Rust 16 tests、native 17 testsで、exit mismatch の no-report/fail-closed と既存の
  compilation、validation、runtime input、source mutation、invalid fixture semanticsを確認した。
- 実 target の Wasmtime、current-source Mac/Linux stage0、packaged artifact、rollback、provider/auth
  の証拠はこの fixture gateに含めない。

## Boundary

これは runtime observation の exit admission に限る verified partial sliceである。
stdout/stderr semantic equality、real component instantiation、Rust/native full producer parity、
current-source Mac/Linux runtime、packaged/rollback parity、live provider/auth は未検証であり、
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。
