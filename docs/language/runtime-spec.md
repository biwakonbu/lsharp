# L# Runtime 仕様

## 目的

本書は、Wasm backend、Component Model backend、Native backend (deferred) が共有する L# runtime の契約を定義する。
runtime はメモリ確保、値表現、GC root 管理、I/O、診断の共通基盤を提供し、compiler core が host 環境へ直接依存しないようにする。

## 適用範囲

本書が扱うのは次の領域である。

- runtime API の公開契約
- `LsharpWord` とヒープオブジェクトの値表現
- GC root 管理と GC-safe point
- ファイル I/O、時刻、環境値の取り扱い
- runtime 起動シーケンスと診断モデル

以下は v1 の対象外とする。

- スレッド
- async / task scheduler
- 動的ロード
- JIT
- socket
- subprocess
- watch mode
- 標準入力を前提にした共通 API

## 設計原則

runtime は次の原則に従う。

1. compiler は runtime API を通じてのみメモリと host 機能へアクセスする
2. Wasm / Component Model / Native (deferred) は同一の値表現と root 管理モデルを共有する
3. GC 導入前後で compiler 側の API 契約を変えない
4. host 依存の差分は runtime boundary で吸収し、compiler core へ漏らさない

## Runtime API

### 共通 API

runtime 実装は少なくとも次の API を提供しなければならない。

| 関数 | シグネチャ | 役割 | Component Model (WIT) |
|------|-----------|------|----------------------|
| `alloc_words` | `(size: Int, tag: Int) -> LsharpWord` | ワード単位のヒープ確保 | guest 内部 (host 非公開) |
| `alloc_bytes` | `(size: Int, tag: Int) -> LsharpWord` | バイト単位のヒープ確保 | guest 内部 (host 非公開) |
| `print` | `(value: LsharpWord) -> void` | 標準出力への出力 | `wasi:io/streams.output-stream` |
| `eprint` | `(value: LsharpWord) -> void` | 標準エラー出力への出力 | `wasi:io/streams.output-stream` |
| `read_file` | `(path: LsharpWord) -> LsharpWord` | ファイル読み込み | `wasi:filesystem/types` |
| `write_file` | `(path: LsharpWord, content: LsharpWord) -> LsharpWord` | ファイル書き込み | `wasi:filesystem/types` |
| `file_exists` | `(path: LsharpWord) -> LsharpWord` | ファイル存在確認 | `wasi:filesystem/types` |
| `read_dir` | `(path: LsharpWord) -> LsharpWord` | ディレクトリ一覧取得 | `wasi:filesystem/types` |
| `clock_now_millis` | `() -> LsharpWord` | 現在時刻をミリ秒で返す | `wasi:clocks/monotonic-clock` |

compiler core は `malloc` や OS syscall を直接呼び出してはならず、必ず上記 API か同等の runtime service を経由する。

native backend では、これらの runtime service をそのままの論理名で扱いつつ、外部 ABI では `lsharp_` 接頭辞付き symbol へ写像してよい。v1 の標準的な対応は次を基準にする。

| Runtime API | Native ABI symbol |
|-------------|-------------------|
| `alloc_words` | `lsharp_alloc_words` |
| `alloc_bytes` | `lsharp_alloc_bytes` |
| `print` | `lsharp_print` |
| `eprint` | `lsharp_eprint` |
| `read_file` | `lsharp_read_file` |
| `write_file` | `lsharp_write_file` |
| `file_exists` | `lsharp_file_exists` |
| `read_dir` | `lsharp_read_dir` |
| `clock_now_millis` | `lsharp_clock_now_millis` |

### 内部管理 API

GC を導入する runtime は、少なくとも次の root 管理 API を備える。

| 関数 | 役割 |
|------|------|
| `root_push` | root stack にポインタを追加する |
| `root_pop` | root stack の末尾を除去する |
| `root_set` | 既存 root slot を更新する |

境界挙動として、次の 1 点を定める。

- **空の root stack に対する `root_pop` は trap せず、root stack を変更せずに `0` を返す。**
  backend は均衡した push/pop を前提にしてよいが、不均衡を未定義動作として扱ってはならない。

これは wasm backend の emitter (`crates/lsharp-wasm/src/wasi/root.rs` の `emit_root_pop_func`)
が既に実装している挙動を契約へ引き上げたものである。`root_push` / `root_set` の戻り値、
root stack の容量上限、失敗時の観測可能性は**まだ未定義**であり、その追跡は
[`ISSUES.md` の `I-17`](../../ISSUES.md#i-17) が正本。

これらは主に compiler が生成するコードや runtime 内部から利用されるものであり、ユーザー向け API ではない。

native backend では内部管理 API も必要に応じて `lsharp_root_push`, `lsharp_root_pop`, `lsharp_root_set` のような symbol へ写像してよい。GC 未導入段階では no-op 互換実装を許容する。

## 値表現

### LsharpWord

`LsharpWord` は、L# の実行時値を表すタグ付き machine word である。
runtime と backend は、少なくとも次の分類を共有しなければならない。

- **immediate**: 整数や `Bool` のように machine word へ直接格納できる値
- **heap object**: String、Vector、ADT、Closure、Ref Cell など、ヒープ領域を参照する値

### ヒープオブジェクトレイアウト

ヒープオブジェクトは、共通の先頭ヘッダを持つ。

```text
+--------+--------+--------+--------+-----+
|  tag   |  size  | field0 | field1 | ... |
+--------+--------+--------+--------+-----+
```

- `tag` はオブジェクト種別を表す
- `size` は後続領域の大きさを表す
- `field0` 以降はオブジェクト本体である

backend ごとに独自のオブジェクトヘッダを導入してはならない。Wasm / Native 間で同一レイアウトを維持する。

### 所有権モデル

値の所有権は runtime が一元管理する。

- ユーザーコードへ `free` は露出しない
- compiler は `alloc_words` / `alloc_bytes` 以外の解放 API を前提にしない
- 複合値の寿命は runtime のメモリ管理方針に従う

## メモリ管理と GC

### GC-safe point

collector が走ってよい地点は、v1 では次に限定する。

1. 関数呼び出し地点
2. ループ backedge
3. runtime API 呼び出し直前

compiler は GC-safe point の前後で、必要な root 集合を必ず明示管理しなければならない。

### Root 管理

root stack は GC から到達可能な参照を保持するための明示的な管理領域である。
例外や異常終了経路を含め、root stack の整合性を壊してはならない。

### GC 導入前の互換性

GC 未導入の段階では bump allocator 互換 runtime を許容する。
ただし、その場合でも compiler から見える API 契約は変えない。

- `root_push` / `root_pop` / `root_set` は no-op 互換で提供してよい
- compiler 側へ GC 有無の条件分岐を持ち込まない
- 将来 collector 実装へ差し替えても同一コード生成規約を保てるようにする

## 文字列・パス・環境値

### 文字列 ABI

L# の文字列は UTF-8 bytes と length を保持するヒープオブジェクトとして表現する。
NUL 終端への変換は runtime boundary でのみ行い、compiler core や言語内部表現へ C 文字列を持ち込まない。

### OS 値の正規化

CLI 引数、環境変数、パスなどの OS 依存値は、runtime が L# の文字列表現へ正規化してから渡す。

| サービス | 役割 |
|---------|------|
| `argv` | コマンドライン引数を提供する |
| `env` | 環境変数を提供する |
| `cwd` | カレントディレクトリを提供する |
| `tempdir` | 一時ディレクトリを提供する |
| `homedir` | ホームディレクトリを提供する |

compiler core は、これらの値を直接 OS syscall から取得してはならない。

### パス操作

パス操作の言語側正本は `stdlib/Path.ls` とする。
OS 差分は主に path separator と canonicalize 挙動に閉じ込め、runtime が吸収する。

## I/O と診断

### I/O 契約

v1 で共通契約に含める I/O は `print`、`eprint`、`read_file`、`write_file`、`file_exists`、`read_dir`、`clock_now_millis` に限定する。
LSP や REPL で必要なストリーム処理は compiler core の共通 API ではなく、ツールチェイン層の adapter として実装する。

### エラー表現

失敗しうる I/O は、例外ではなく `Result` 相当のタグ付き値で返す。
runtime は OS 固有のエラー表現を、L# が扱えるエラー値へ写像しなければならない。

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
| `stderr` | 診断、障害、内部エラー |

| 終了コード | 意味 |
|-----------|------|
| `0` | 正常終了 |
| `1` | ユーザー入力や I/O を含む回復可能な失敗 |
| `2` | runtime または compiler の内部エラー |

compiler 診断と runtime 障害は別経路で扱い、同一のエラー種別へ曖昧に混ぜない。

## 起動シーケンス

runtime は概ね次の順序で初期化される。

```text
runtime_init
  -> argv/env/path 正規化
  -> GC 初期化
  -> compiler main 呼び出し
  -> runtime_shutdown
```

CLI、LSP、REPL、formatter、doc generator は同一の runtime 初期化経路を共有し、ツールごとの差分は `main` 以降に閉じ込める。

## 再初期化と観測

stageN-native が別プロセスに分離されない形で再帰的に自己コンパイルできるよう、runtime は再初期化可能であることが望ましい。
また、profiling や統計情報は v1 では内部フラグに限定し、ユーザー向け標準出力へ混在させない。

## 関連文書

- [`backend-boundary.md`](./backend-boundary.md)
- [`native-backend-spec.md`](./native-backend-spec.md)
