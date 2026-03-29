# package-layout

L# パッケージの標準レイアウト。

## 公開 package と内部 source root

- **公開 package**: `lsharp.toml` を持ち、`lsharp init` / `lsharp add` / `lsharp install` / `lsharp publish` の対象になる配布単位
- **内部 source root**: `selfhost` のように配布単位ではないが、同じ `src/` 配置規約とモジュール解決規約を使うソースツリー

`selfhost` は公開 package ではないが、正本ソースは `selfhost/src/**` に置き、`(import A.B)` → `src/A/B.ls` の対応を共有する。

## 推奨ディレクトリ構成

```text
my-package/
  lsharp.toml
  src/
    Main.ls
    MyModule.ls
    MyModule/
      Sub.ls
  examples/
  tests/
  docs/
    api.json
```

## 役割

- `lsharp.toml`: パッケージ名、バージョン、entry、依存関係を定義する
- `src/`: import 対象になる本体モジュールを置く
- `examples/`: 利用例や最小サンプルを置く
- `tests/`: package 単位の検証コードを置く
- `docs/`: `lsharp doc --json` が生成する `api.json` などの成果物を置く

## モジュールとファイルの対応

- `(import MyModule)` は `src/MyModule.ls` を解決する
- `(import MyModule.Sub)` は `src/MyModule/Sub.ls` を解決する
- 公開 package の解決順序は `src/` → `.lsharp/packages/*/src/` → `stdlib/`
- `lsharp.toml` がない内部 source root でも、entry file から最も近い `src/` 祖先を source root として同じ規約を使う
- selfhost compiler-mode は installed package を直接 directory scan せず、`lsharp install` が生成する `.lsharp/module-index/*.path` を内部 index として参照する
- 例: `selfhost/src/App/Main.ls` から `(import Syntax.Token)` は `selfhost/src/Syntax/Token.ls` を解決する

## 移行方針

- `selfhost/src/**` が正本であり、flat な `selfhost/*.ls` は互換移行のための一時コピーとして扱う
- 新規実装・検証・ドキュメントは `selfhost/src/**` と `selfhost/src/App/Main.ls` を基準に更新する

## `lsharp init` が生成する最小構成

`lsharp init <name>` は次を生成する。

- `lsharp.toml`
- `src/Main.ls`
- `examples/`
- `tests/`
- `docs/`
- `.gitignore`
