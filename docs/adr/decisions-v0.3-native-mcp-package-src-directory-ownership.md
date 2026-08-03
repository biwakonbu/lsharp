# ADR: v0.3 native MCP package src directory ownership

## 状態

Verified partial slice（2026-08-03）。`I-09` / `M3-05-N9` / `EC-M3-05` は `[~]` のまま維持する。

## 背景

regular な installed package directory でも、root `src/` が directory symlink なら source enumerator の
`is_dir` / directory traversal が package root 外へ進み、外部の `.ls` file を package API として投影できた。
native routeでは、その外部 sourceに対して `doc --json` を実行するため、package ownership boundaryを越えた
実行も発生していた。package entry、manifest、`docs/api.json` の ownership guardだけではこの nested source
boundaryを閉じられない。

## 決定

- regular package の root `src/` は regular non-symlink directory の場合だけ source-owned tree として扱う。
- native source enumerator は root `src/` symlinkを空の source setとして扱い、missing `api.json` の既存 fail-closed
  診断を返す。
- Rustの package API route と共通 API-doc builder は `symlink_metadata` で root `src/` symlinkを検出し、外部 sourceを
  読み込まず、API responseへ module/function/metadataを投影しない。
- package API生成で sourceが拒否された場合、native programを起動しない。
- `src/` 配下の child symlink、個別 source file、`docs/` directory、installer/provider取得はこの decisionの対象外とし、
  別の ownership boundaryとして残す。

## 証跡

同一 fixtureで、regular package `demo-1.0.0` の `src -> <external>/` に外部 `Geometry.ls`を置き、
`docs/api.json` は作成しない状態を固定した。

- REDではRust/nativeとも外部 `Geometry`をAPIへ投影した。native側では fake native program の `doc` invocationも
  発生した。
- GREENでは `lsharp_package_api` が missing-source errorで終了し、`Geometry`のname/content/metadataを返さず、
  native logも作成しないことを確認した。
- Rust MCP package APIは9 tests、Rust API-doc package fixtureは2 tests、native MCP suiteは107 testsがpassした。
- `rustfmt --edition 2024 --check`、Python `py_compile`、`git diff --check`もpassした。
- 実行した代表コマンドは次のとおり。

```text
PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py
cargo test -p lsharp-driver mcp_server::tests::test_package_api_tool -- --nocapture
cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package -- --nocapture
```

実装とテストの code checkpoint は `f90a5f8963bf0df6cc4479963cbb29470798a40e`。
この offline MCP boundaryにLinux VM、stage regeneration、target runtime、provider networkは必要ないため実行していない。

## 影響と残タスク

この決定は package root `src/` directory symlinkの source traversal と native no-executionだけを閉じる。
regular `src/` 配下の child symlink、個別 source file、`docs/` directoryのownership、package installer/registry/provider/auth、
current-source Mac Apple Silicon / Linux x86_64 runtime、packaged/release/rollback parityは未検証であり、
`I-09` / `M3-05-N9` / `EC-M3-05` と Rust-free全体の完了を意味しない。
