# ADR: v0.2 source validate の negative sampling fail-closed 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: selfhost `App.Cli` / `EmbeddedCli` の `validate --source` source/report boundary
- Related: `EC-M2-02`, `EC-M2-03`, `docs/adr/decisions-v0.2-native-validation-evidence-negative-cases.md`

## Context

source evidence の `cases` / `seed` / `shrinks` / `coverage` は parser→registry の入力境界で
非負値だけを受理する。従来は registry の直接 consumer と native source-file smoke に
negative sampling の証跡があったが、公開 `validate --source` が同じ入力診断で停止し、report や
manifest を生成しないことを固定していなかった。

## Decision

- `validate --source <file> --format json --emit-manifest <path>` が sampling の不正値を検出したら、
  `source validation error:11`、exit `1` を返す。
- 入力診断は validation report の `status` と混ぜず、stdout に JSON report を出力しない。
- graph が構築できない段階では `--emit-manifest` の出力ファイルも作成しない。
- 既存の `App.Cli` / `EmbeddedCli` source graph error path を契約テストで固定し、sampling の意味論、
  count と `cases` の関係、current-source stage0 artifact/runtime parity はこの slice の対象外とする。

## Evidence

- `test_e2e_selfhost_cli_validate_source_rejects_negative_sampling_without_report_or_manifest` が、
  `:cases -1` の source fixtureに対して exit `1`、`source validation error:11`、reportなし、
  manifestなしを Rust-host actual Wasm で確認する。
- `test_e2e_selfhost_embedded_cli_validate_source_rejects_negative_sampling_without_report_or_manifest` が、
  同じ `:cases -1` の source fixtureに対して exit `1`、`source validation error:11`、reportなし、
  manifestなしを EmbeddedCli の Rust-host actual Wasm で確認する。
- `test_e2e_selfhost_embedded_cli_validate_source_rejects_negative_seed_and_shrinks_without_report_or_manifest`
  が `:seed -1` と `:shrinks [-1]` の source fixtureを同じ EmbeddedCli actual Wasmへ通し、両方で
  exit `1`、`source validation error:11`、reportなし、manifestなしを確認する。bundle は一度だけ compileし、
  fixtureごとの実行結果を分離して検証する。
- 既存の source parser→Evidence registry test、Rust syntax parser test、native source-file smoke
  は同じ negative sampling boundary を下位層と native harness で確認する。
- focused 実行:
  `cargo test -p lsharp-wasm --test e2e selfhost_cli_core::test_e2e_selfhost_cli_validate_source_rejects_negative_sampling_without_report_or_manifest -- --nocapture --test-threads=1`
  `cargo test -p lsharp-wasm --test e2e selfhost_cli_actual_main_args::test_e2e_selfhost_embedded_cli_validate_source_rejects_negative_sampling_without_report_or_manifest -- --nocapture --test-threads=1`

## Boundary

これは `App.Cli` と `EmbeddedCli` の Rust-host actual Wasm に限定した verified partial slice である。
`cases`、`seed`、`shrinks` の入力値を各 CLI surface が fail-closed に拒否することを固定したが、
current-source packaged stage0、Mac Apple Silicon / Linux x86_64 の artifact/runtime matrix、sampling
generator の実行意味論、`lsharp validate` 全体の完了を意味しない。
