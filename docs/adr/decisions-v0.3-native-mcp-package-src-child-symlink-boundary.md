# ADR: v0.3 native MCP package src child symlink boundary

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

## 証跡

`src/linked -> <external>/` の directory symlinkと外部 `Geometry.ls`を同一形状で作成し、Rust tooling、Rust MCP、module-index、
native MCPの各経路を確認した。

- `cargo test -p lsharp-tooling api_doc::tests::test_build_api_doc_for_package_` — 3 tests passed。
- `cargo test -p lsharp-driver --bin lsharp mcp_server::tests::test_package_api_tool` — 10 tests passed。
- `cargo test -p lsharp-driver --bin lsharp tests::test_cmd_install` — 24 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py` — 108 tests passed。
- `cargo test -p lsharp-driver --bin lsharp tests::test_collect_package_source_files_rejects_nested_symlink -- --exact` — passed。

native成功経路では fake programのlogが作られず、`Geometry`はresponseへ投影されない。Linux VM、stage regeneration、provider networkは
このoffline ownership contractに必要ないため実行していない。

## 残る境界

この決定はAPI-doc/module-indexのchild symlink traversalだけを閉じる。regular child source fileの直接fixture、`docs/` directoryや
package installer/provider/auth、current-source Mac Apple Silicon / Linux x86_64 runtime、packaged/release/rollback parityは未検証であり、
`I-09` / `M3-05-N9` / `EC-M3-05`とRust-free全体の完了を意味しない。
