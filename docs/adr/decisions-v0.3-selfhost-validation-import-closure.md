# ADR: v0.3 selfhost validation module import closure

- Status: Accepted (verified build-unblocking slice)
- Date: 2026-07-31
- Scope: `selfhost/src/Tools/Validation/Evidence.ls` / `ReviewIdentity.ls`
- Related: [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)、
  [`decisions-v0.3-selfhost-embedded-cli-review-evidence-identity.md`](decisions-v0.3-selfhost-embedded-cli-review-evidence-identity.md)、
  `EC-M3-04` / `EC-M3-05`

## Context

origin/main の `lsharp-driver` build script は `EmbeddedCli.ls` から selfhost の validation
modules を compile する。`Evidence.ls` は `ReviewIdentity.ls` の identity JSON projection を
呼び出していたが、module import がなく、通常の embedded component build が undefined symbol
で停止していた。さらに `ReviewIdentity.ls` 自身も rooted vector builder と JSON escape helper を
直接使うのに、それぞれを定義する module を import していなかった。

この状態では Rust 側の review CLI/MCP test が既存 component artifact の明示指定なしに実行できず、
source/selfhost parity を検証する通常の driver build boundary が閉じない。

## Decision

- `Evidence.ls` は `Tools.Validation.ReviewIdentity` を直接 import し、identity JSON projection
  の依存を module graph に明示する。
- `ReviewIdentity.ls` は rooted vector builder を提供する `Syntax.Parser` と、既存の JSON escape
  helper を提供する `Tools.Lsp.JsonRpc` を直接 import する。
- validation module は transitive import に依存せず、使用する helper の owner module を直接宣言する。
  production semantics、wire shape、review state は変更しない。

## Evidence

- RED: 通常の `cargo build -p lsharp-driver` が `Evidence.ls` の
  `source-review-evidence-identity-json` undefined、続いて `ReviewIdentity.ls` の
  `vector-push-quad-rooted-v3` / `json-escape-string` undefined で失敗した。
- GREEN: import 修正後の通常 `cargo build -p lsharp-driver` が成功し、selfhost embedded
  component を current checkout から生成できた。
- `cargo test -p lsharp-driver --test review_input_cli`: 17 passed。
- `cargo test -p lsharp-driver 'mcp_server::tests::test_validate_tool_'`: 27 passed。
- 上記テストは `LSHARP_EMBED_COMPONENT_PATH` を指定せず、通常の embedded component build で
  実行した。
- package 全体の `cargo test -p lsharp-driver` は unit 195件と review suite を通過したが、
  `default_path_delegation` の12件は origin/main に既存の selfhost semantic/output mismatch と
  別の `SmokeCli.ls` undefined symbol で失敗した。これらは active branch の型推論/App 経路と
  重なるため、この import closure の完了証拠へは拡大解釈しない。

## Boundary

これは selfhost module dependency と Rust driver の default embedded build を閉じる verified
slice である。native stage0 の current-source/package provenance、Mac Apple Silicon / Linux
x86_64 artifact/runtime parity、EC-M3-04 の全 producer parity、EC-M3-05 release gate は未完了で、
関連 TODO は `[~]` / `[ ]` のまま維持する。
