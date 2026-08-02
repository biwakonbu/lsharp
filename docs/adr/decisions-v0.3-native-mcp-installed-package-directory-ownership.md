# ADR: v0.3 native MCP installed package directory ownership

## 状態

Verified partial slice（2026-08-02）。

## 背景

Rust/native の installed package discovery は `.lsharp/packages/<entry>` に `is_dir` を使っていたため、
symlink directory の target にある `lsharp.toml`、`docs/api.json`、source を package root 所有の package として
`lsharp_search`、`lsharp_project_context`、`lsharp_package_api` へ投影できた。直前の `docs/api.json`
regular-file boundary は最終 JSON entryだけを検査するため、package directory 自体の traversal は閉じない。

## 決定

installed package entry は `.lsharp/packages` 直下の regular non-symlink directory だけを discovery 対象にする。
enumeration surface の `lsharp_search` と `lsharp_project_context` は、既存の non-directory entry と同様に
symlink directory を無効な entry として無視する。explicit lookup の `lsharp_package_api` は対象を見つけず、
既存の「インストール済みパッケージが見つかりません」で fail closed にする。

## 証跡

- RED: package root 外の package directory を指す同一 symlink fixture が、search/context に現れ、package API
  lookupにも採用された。
- GREEN: Rust/native の search/context は regular directory だけを返し、explicit symlink lookupは not-foundとなる。
- native program、外部 package root の metadata/source は読み取られない。
- Linux replay、stage regeneration、live provider/auth、実 target runtime は実行していない。

## 影響

これは offline installed-package discovery ownership に限る verified partial である。installerが作る package tree
全体の nested symlink policy、registry/provider取得、current-source Mac/Linux runtime、packaged/rollback parity は
未検証のため、`EC-M3-05` と `M3-05-N9` は `[~]` のまま維持する。
