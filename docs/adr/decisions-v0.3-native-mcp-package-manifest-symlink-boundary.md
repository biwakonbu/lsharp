# ADR: v0.3 native MCP package manifest symlink boundary

## 状態

Verified partial slice（2026-08-02）。

## 背景

`.lsharp/packages/<entry>` 自体を regular directory に限定しても、その内部の `lsharp.toml` が symlink なら package root 外の name/version を `lsharp_search` と `lsharp_project_context` へ投影でき、`lsharp_package_api` の expected identity としても利用できた。これは package entry boundaryや `docs/api.json` final-file boundaryとは異なる nested metadata ownership である。

## 決定

installed package discovery は、`lsharp.toml` が存在する場合に symlink でないことを必須化する。manifestがないpackageをsourceから扱う既存経路は維持する。enumeration surfaceのsearch/project-contextは symlink-manifest packageを無視し、explicit package-api lookupは既存not-found診断でfail closedにする。

## 証跡

- RED: regular package directory内の`lsharp.toml`を外部manifestへ向けた同一fixtureがsearch/contextへ現れ、package-api lookupにも採用された。
- GREEN: Rust/nativeともsearch/contextから除外し、package-apiはnot-foundを返す。
- native fake programは実行されず、外部manifestのidentityを応答へ投影しない。
- Linux replay、stage regeneration、live provider/auth、実target runtimeは実行していない。

## 影響

これはnested `lsharp.toml` ownershipだけのverified partialである。`src/`、個別source、`docs/`、その他のnested symlink、installer、provider、current-source Mac/Linux runtime、packaged parityは未検証のため、`EC-M3-05`と`M3-05-N9`は`[~]`のまま維持する。
