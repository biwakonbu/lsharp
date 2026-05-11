---
name: lsharp-usage
description: L# v0.1.0 / v0.1.0-native-rc1 のインストール、公開 CLI、Wasm 生成、metadata test/doc、LSP/MCP、native RC の扱いを案内する。Use when users want to install, try, run, or explain the current L# language distribution.
---

# L# 利用スキル

このスキルは、現在の L# (`lsharp`) を利用者向けに案内・実行するための手順をまとめる。自然言語の説明は日本語で行い、コマンドやコードはそのまま提示する。

## 現在の配布前提

- installable release: `v0.1.0-native-rc1`
- installed binary version: `lsharp 0.1.0`
- primary install location: `~/.local/bin`
- stable user-facing distribution: Rust host launcher + embedded/selfhost guest component
- experimental native-only RC: self-regeneration evidence bundle; stable CLI replacement ではない

`experimental-native-rc-*` の `program.native` は通常の利用者向け `lsharp` CLI として案内しない。日常利用には `lsharp-v0.1.0-native-rc1-aarch64-apple-darwin.tar.gz` に含まれる packaged `lsharp` を使う。

## インストール

macOS Apple Silicon の現在の RC は GitHub Release から `curl` で `~/.local/bin` に入れる。

```bash
curl -fsSL https://github.com/biwakonbu/lsharp/releases/download/v0.1.0-native-rc1/install.sh \
  | LSHARP_VERSION=v0.1.0-native-rc1 sh

export PATH="$HOME/.local/bin:$PATH"
lsharp --version
```

期待される出力:

```text
lsharp 0.1.0
```

install されるファイル:

```text
~/.local/bin/lsharp
~/.local/bin/lsharp-lsp
~/.local/bin/lsharp.component.wasm
```

別の場所へ入れる場合:

```bash
curl -fsSL https://github.com/biwakonbu/lsharp/releases/download/v0.1.0-native-rc1/install.sh \
  | LSHARP_VERSION=v0.1.0-native-rc1 LSHARP_INSTALL_DIR="$HOME/bin" sh
```

## Hello World / compile

公開 CLI の基本導線は `compile`。`parse` / `check` / `fmt` はユーザー向けの主導線として案内せず、LSP/MCP 内部 API 寄りとして扱う。

```bash
cat > hello.ls <<'EOF'
(defn main [] 42)
EOF

lsharp compile hello.ls -o hello.wasm
```

生成物が Wasm であることを確認する場合:

```bash
xxd -p -l 4 hello.wasm
```

期待値:

```text
0061736d
```

## metadata test

`:example` / `:invariant` を書くと `lsharp test` で検証できる。

```bash
cat > metadata.ls <<'EOF'
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
EOF

lsharp test metadata.ls
```

## docs

HTML docs:

```bash
lsharp doc metadata.ls -o metadata.html
```

JSON docs:

```bash
lsharp doc metadata.ls --json -o api.json
```

## IDE / AI backend

IDE:

```bash
lsharp lsp
```

AI / tool backend:

```bash
lsharp mcp-server
```

## 外部 compiler / 配布物の接続確認

`LSHARP_PATH` は、埋め込み guest component の代わりに外部 compiler / 配布物への process-entry delegation を試すための入口。

```bash
LSHARP_PATH=/path/to/lsharp lsharp --version
```

## よくある判断

- 利用者に「Rust toolchain が必要か」と聞かれたら、installable release を使うだけなら不要と答える。
- `~/.local/bin` が PATH にない場合は、`export PATH="$HOME/.local/bin:$PATH"` を案内する。
- native-only が正式置換済みか聞かれたら、現時点では experimental RC であり、official stable 置換には archive layout / release smoke / signing-notarization / rollback anchor / Tier1 target matrix が残ると説明する。
- smoke test は `lsharp compile hello.ls -o hello.wasm` と Wasm magic `0061736d` の確認を優先する。

