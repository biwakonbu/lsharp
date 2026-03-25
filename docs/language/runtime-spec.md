# L# Runtime 仕様 (v1)

## 概要

Wasm/Native 両 backend から呼べる共通 runtime API を定義する。
ネイティブ runtime v1 は selfhost compiler 実行に必要な最小機能だけを持たせ、スレッド、async、動的ロード、JIT は scope 外にする。
GC 導入前は bump allocator 互換 runtime で selfhost を成立させ、GC 導入後に同一 runtime API の実装だけを差し替える。

## Runtime API (P11-2c)

| 関数 | シグネチャ | 説明 |
|------|-----------|------|
| `alloc_words` | `(size: Int, tag: Int) -> LsharpWord` | ワード単位のヒープ確保 |
| `alloc_bytes` | `(size: Int, tag: Int) -> LsharpWord` | バイト単位のヒープ確保 |
| `print` | `(value: LsharpWord) -> void` | stdout への出力 |
| `eprint` | `(value: LsharpWord) -> void` | stderr への出力 |
| `read_file` | `(path: LsharpWord) -> LsharpWord` | ファイル読み込み (Result を返す) |
| `write_file` | `(path: LsharpWord, content: LsharpWord) -> LsharpWord` | ファイル書き込み (Result を返す) |
| `file_exists` | `(path: LsharpWord) -> LsharpWord` | ファイル存在チェック (Bool) |
| `read_dir` | `(path: LsharpWord) -> LsharpWord` | ディレクトリ読み込み (Result を返す) |
| `clock_now_millis` | `() -> LsharpWord` | 現在時刻 (ミリ秒) |

compiler 側は直接 `malloc` 相当を呼ばず、上記 API を通じてのみメモリを確保する。

## 値表現 (P11-2c-1)

### LsharpWord

タグ付き machine word。immediate と heap pointer を統一的に表現する。

- **immediate**: 整数、Bool はタグ付き即値として machine word に直接格納
- **heap objects**: String, Vector, ADT, Closure, Ref Cell はヒープに確保

### ヒープオブジェクトレイアウト

```
+--------+--------+--------+--------+-----+
|  tag   |  size  | field0 | field1 | ... |
+--------+--------+--------+--------+-----+
```

- ヒープヘッダは `[tag, size, ...fields]` の固定レイアウト
- backend ごとの独自レイアウトは禁止し、Wasm/native で共通化する

### 所有権モデル

- すべてランタイム管理
- ユーザーコードに `free` は露出しない
- compiler 側は `alloc_words` / `alloc_bytes` のみを呼び出す

## GC と Root 管理 (P11-2c-2)

### Root API

| 関数 | 説明 |
|------|------|
| `root_push` | root stack にポインタを追加 |
| `root_pop` | root stack からポインタを除去 |
| `root_set` | root stack の指定位置を更新 |

### GC-safe point

以下の地点を GC-safe point とし、それ以外では collector が走らない (v1 契約):

1. **call site** -- 関数呼び出し地点
2. **loop backedge** -- ループの末尾ジャンプ
3. **runtime call 直前** -- runtime API 呼び出しの直前

compiler は GC-safe point の前後で必ず root 集合を明示管理する。

### GC 導入前の互換性

- bump allocator 実装でも同じ root API を no-op 互換で提供する
- compiler 側に条件分岐を持ち込まない
- 例外/異常終了経路でも root stack が破壊されないよう、runtime abort パスと compiler 生成 epilogue の整合を保証する

## 文字列・パス・環境 (P11-2c-3)

### 文字列 ABI

- UTF-8 bytes + length を保持する heap object
- NUL 終端への変換は runtime boundary のみで行う
- compiler core には L# 文字列だけを渡す

### OS 値の正規化

ファイルパス、環境変数、CLI 引数は runtime で OS 文字列から L# 文字列へ正規化する。

| サービス | 説明 |
|---------|------|
| `argv` | コマンドライン引数 |
| `env` | 環境変数 |
| `cwd` | カレントディレクトリ |
| `tempdir` | 一時ディレクトリ |
| `homedir` | ホームディレクトリ |

これらは runtime service として切り出し、直接 OS syscall を compiler core に露出しない。

### パス操作

- 既存 `stdlib/Path.ls` を正本とする
- OS 差分は separator と canonicalize 挙動だけ runtime で吸収する

## I/O と時刻 (P11-2c-4)

### v1 API

`print`, `eprint`, `read_file`, `write_file`, `file_exists`, `read_dir`, `clock_now_millis` に固定する。

### v1 scope 外

以下は v1 scope 外とし、必要になった時点で別 Phase を切る:

- 標準入力 (stdin)
- watch mode
- socket
- subprocess

### ツールチェイン層 adapter

LSP/REPL 用の stdin/stdout ストリームは compiler core 共通 API ではなく、ツールチェイン層の adapter として実装する。

### エラー表現

失敗しうる I/O API は `Result` 相当のタグ付きオブジェクトを返す。native runtime が errno/OS error を L# エラー値へ写像する。

## エラーと診断 (P11-2c-5)

### Runtime Error 分類

| エラー種別 | 説明 |
|-----------|------|
| `panic` | 回復不能なプログラムエラー |
| `io_error` | I/O 操作の失敗 |
| `alloc_error` | メモリ確保の失敗 |
| `internal_error` | runtime 内部エラー |

### 出力規約

| チャネル | 用途 |
|---------|------|
| `stdout` | 通常出力 |
| `stderr` | 診断/障害 |

| 終了コード | 意味 |
|-----------|------|
| `0` | 正常終了 |
| `1` | ユーザーエラー |
| `2` | 内部エラー |

### 診断の分離

- compiler 診断 (型エラー、構文エラー) は L# 診断値で表現する
- runtime 障害は runtime error 値で表現する
- 両者は別経路とし、混合しない

## 起動シーケンス (P11-2c-6)

### 起動フロー

```
runtime_init
  -> argv/env/path 正規化
  -> GC 初期化
  -> compiler main 呼出し
  -> runtime_shutdown
```

### 共有初期化

CLI, LSP, REPL, formatter, doc generator は同一 runtime 初期化経路を共有する。ツール別の差分は main 以降に閉じ込める。

### 再初期化

stageN-native が selfhost compiler として別プロセスを起動せずに再帰的自己コンパイルできるよう、runtime は再初期化可能にする。

### profiling/statistics

v1 では内部フラグに限定し、ユーザー向けデフォルト出力へ混ぜない。
