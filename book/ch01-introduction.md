# はじめに

## L# とは何か

L# (エルシャープ) は、Lisp の S 式構文と F# の強力な型システムを融合したプログラミング言語である。WebAssembly にコンパイルし、ブラウザ・サーバー・エッジ環境で同一のコードを実行できることを目指している。

```lisp
;; フィボナッチ数列 -- L# の最初のプログラム
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(defn main []
  (print (fib 10)))
```

このコードは L# コンパイラによって WebAssembly バイナリにコンパイルされ、`wasmtime` で直接実行できる。

## なぜ L# を作るのか

既存の言語には多くの優れたものがある。では、なぜ新しい言語が必要なのか。

L# は以下の3つの課題に対する回答として設計された:

1. **型安全性と簡潔さの両立**: JavaScript/TypeScript は広く使われているが、型安全性に限界がある。Haskell や OCaml は強力だが、構文が複雑になりがちである。L# は S 式という最小限の構文で、Hindley-Milner 型推論による完全な型安全性を提供する
2. **ユニバーサルなターゲット**: WebAssembly をコンパイルターゲットにすることで、一つの言語でフロントエンド・バックエンド・エッジコンピューティングを統一する
3. **コンパイラ学習の教材**: L# コンパイラ自体が、コンパイラ実装を学ぶための生きた教材となることを目指している

## 本書の構成

本書は L# コンパイラの実装を通じて、言語処理系の基礎から応用までを解説する。

**第 I 部: 基礎 -- 動くコンパイラを作る**

- **第 2 章 字句解析**: ソースコードをトークン列に分解する
- **第 3 章 構文解析**: トークン列から抽象構文木 (AST) を構築する
- **第 4 章 型推論**: Hindley-Milner 型推論で型安全性を保証する
- **第 5 章 中間表現**: AST から実行可能な命令列に変換する
- **第 6 章 コード生成**: WebAssembly バイナリを出力する

**第 II 部: 拡張 -- 型システムを進化させる**

- **第 7 章 レコード型**: 構造化データと WasmGC
- **第 8 章 型エイリアスと制約付き型**: 型に名前と制約を付ける
- **第 9 章 モジュールシステム**: コードの組織化
- **第 10 章 トレイト**: アドホック多相とインタフェース
- **第 11 章 高度な型機能**: 高カインド型、GADT、Computation Expressions

**第 III 部: 実践**

- **第 12 章 エラー報告**: 開発者に優しいコンパイラを作る
- **第 13 章 テスト戦略**: コンパイラのテスト手法

## 前提知識

本書を読むにあたり、以下の知識があると望ましい:

- プログラミングの基本的な概念 (変数、関数、制御構造)
- Rust の基本的な文法 (L# コンパイラは Rust で実装されている)
- コマンドラインの基本操作

コンパイラや型理論の知識は前提としない。必要な概念はその都度解説する。

## 環境構築

L# コンパイラをビルド・実行するには以下のツールが必要である:

```bash
# Rust ツールチェインのインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# wasmtime (Wasm ランタイム) のインストール
curl https://wasmtime.dev/install.sh -sSf | bash

# L# コンパイラのビルド
git clone https://github.com/biwakonbu/lsharp.git
cd lsharp
cargo build
```

ビルドが成功したら、最初のプログラムをコンパイルして実行してみよう:

```bash
# examples/fib.ls をコンパイル
cargo run -- compile examples/fib.ls -o fib.wasm

# 実行
wasmtime fib.wasm
# => 55
```

`55` が表示されれば、環境構築は完了である。

## プロジェクト基盤 -- git と lsharp.toml

L# コンパイラはプロジェクトが **git リポジトリ**であることを前提とする。ドキュメント鮮度追跡・知識ベース・変更検知の全てが git に依存するためである。git リポジトリが見つからない場合、コンパイラはエラーを報告する:

```
$ lsharp build
error[PROJ001]: git リポジトリが見つかりません。

  L# はドキュメント追跡と知識管理に git を使用します。
  以下のコマンドでリポジトリを初期化してください:

    git init
    git add .
    git commit -m "Initial commit"
```

`lsharp init` コマンドで git 初期化を含むプロジェクトセットアップを自動化できる:

```
$ lsharp init my-project
  [1/4] ディレクトリ作成: my-project/
  [2/4] git リポジトリ初期化: git init
  [3/4] プロジェクト設定: lsharp.toml
  [4/4] 初期コミット: git commit -m "lsharp init"
```

`lsharp.toml` はプロジェクト設定ファイルである。制約付き型の自動テスト回数やドキュメントレビューの設定を記述する:

```toml
[project]
name = "my-project"

[constraints]
random-test-count = 100

[doc-review]
structured = "error"
comments = "warn"
pre-commit = "block"
```
