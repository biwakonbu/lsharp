# ADR: v0.3 native MCP package src child ownership boundary

## 状態

Verified partial slice（2026-08-04）。`I-09` / `M3-05-N9` / `EC-M3-05` は `[~]` のまま維持する。

## 背景

package rootの `src/` directory symlinkは既存の境界で拒否していたが、RustのAPI-doc collectorとmodule-index collectorは
`Path::is_dir()` を使っていた。このため、regularな `src/` の下にある child directory symlinkが外部 treeへ追跡され、外部の
`.ls` sourceがpackage APIやmodule indexへ投影される可能性があった。native MCP shimのsource enumeratorはこのfixtureを
既にfail-closedに扱っていたため、Rust/native parityが崩れていた。

## 決定

- package `src/` treeの各 entryは、`is_dir` / `is_file`を使う前に`symlink_metadata`で検査する。
- child symlinkを検出した場合は、外部sourceを読む前に `package src tree must not contain symlinks` で拒否する。
- 同じ境界をAPI-doc生成とinstalled module-index生成に適用する。
- native MCPは既存のsource exclusionを維持し、同一fixtureで外部moduleの名前・内容・metadataを投影せず、native programも起動しないことを確認する。
- `src/` の通常 directory と通常 `.ls` file は再帰的に所有し、`docs/` は source collector の対象外として扱う。nested moduleの API projectionは
  source pathではなく native documentの module identityを使い、native `doc` invocationには実際の nested source pathを渡す。

## 証跡

`src/linked -> <external>/` の directory symlinkと外部 `Geometry.ls`を同一形状で作成し、Rust tooling、Rust MCP、module-index、
native MCPの各経路を確認した。続いて `src/Geometry/Vec2.ls` の通常 source fileと `docs/guides/README.txt` を追加し、nested moduleの
projectionと docs exclusionを確認した。

- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package_` — 3 tests passed。
- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package` — 4 tests passed。
- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package` — 6 tests passed。直接 source file symlinkと Unix socket
  special entryを追加し、それぞれ symlink拒否、source列挙からの除外を確認した。
- `cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_package_api_tool` — 13 tests passed。Rust MCPでも同じ境界を確認した。
- `cargo test -p lsharp-driver --bin lsharp tests::test_cmd_install` — 24 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py` — 111 tests passed。native MCPは直接 source file symlinkを
  fail-closedにし、special entryを source列挙から除外して fake `doc` programを起動しない。
- `cargo test -p lsharp-driver --bin lsharp tests::test_collect_package_source_files_rejects_nested_symlink -- --exact` — passed。
- `f76dafb9` の native fixtureでは fake programの log が `["doc", "<package>/src/Geometry/Vec2.ls", "--json"]` となり、docs entryは native executionへ渡らない。

native成功経路では fake programのlogが作られず、`Geometry`はresponseへ投影されない。Linux VM、stage regeneration、provider networkは
このoffline ownership contractに必要ないため実行していない。

## 残る境界

この決定はpackage source treeの regular directory/file、`.ls` filtering、directory/直接 file symlink拒否、special filesystem entryの除外、docs exclusionまでを閉じる。package installer/provider/auth、
current-source Mac Apple Silicon / Linux x86_64 runtime、packaged/release/rollback parityは未検証であり、
`I-09` / `M3-05-N9` / `EC-M3-05`とRust-free全体の完了を意味しない。
