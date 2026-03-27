# package-layout

L# パッケージの標準レイアウト。

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
- 解決順序は `src/` → `.lsharp/packages/*/src/` → `stdlib/`

## `lsharp init` が生成する最小構成

`lsharp init <name>` は次を生成する。

- `lsharp.toml`
- `src/Main.ls`
- `examples/`
- `tests/`
- `docs/`
- `.gitignore`
