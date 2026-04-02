# セルフホスティング -- L# で L# コンパイラを書く

## ブートストラップとは何か

コンパイラのセルフホスティング (self-hosting) とは、コンパイラを自身がコンパイルする言語で書くことである。C コンパイラは C で書かれ、Rust コンパイラは Rust で書かれている。

ブートストラップの正本フローは以下のようになる:

```
Stage 0: 既存のコンパイラ (Rust 版 L# コンパイラ)
    ↓ selfhost/src/**/*.ls をコンパイル
Stage 1: L# で書かれた L# コンパイラ (stage1.wasm)
    ↓ stage1.wasm で selfhost/src/**/*.ls を再コンパイル
Stage 2: stage1 が生成した L# コンパイラ (stage2.wasm)
    ↓ stage2.wasm で selfhost/src/**/*.ls を再コンパイル
Stage 3: stage2 が生成した L# コンパイラ (stage3.wasm)
```

この章では Wasm backend による `stage0 -> stage1 -> stage2 -> stage3` を bootstrap の正本とみなす。固定点の成立条件は `stage2.wasm == stage3.wasm` であり、stage1 は「Rust 実装が L# 実装を起動できること」の確認、stage2/stage3 は「L# 実装だけで自己再生成が閉じること」の確認を担う。

## なぜセルフホスティングを行うのか

セルフホスティングには実用的・教育的な価値がある:

1. **言語の検証**: コンパイラは非自明なプログラムである。自身をコンパイルできることは、言語の表現力と正しさの証明になる
2. **ドッグフーディング**: 自分の言語で大きなプログラムを書くことで、使いにくい点や不足している機能が見える
3. **循環依存の解消**: 他の言語への依存を減らし、言語が自立する
4. **達成感**: コンパイラが自分自身を生み出す瞬間は、プログラミングの醍醐味の一つである

## 移植戦略

### Rust から L# への段階的移植

L# コンパイラの Rust 実装は約 18,000 行ある。これを一度に L# に移植するのは現実的でない。そこで段階的なアプローチを取る:

1. 各コンパイラフェーズを個別の L# モジュールとして実装
2. 各モジュールを Rust 版の出力と比較テスト
3. 全モジュールを統合して stage1 を生成

### multi-backend 設計と 2026-03-30 の方針転換

Phase 11 のセルフホスト化では、selfhost compiler の frontend / lowering を 1 つに保ち、その後段だけを backend ごとに分岐させる。用語は `docs/language/backend-boundary.md` に合わせ、`FrontendResult -> LoweredModule -> CodegenArtifact` の 3 層で責務を固定する。

```text
Source (.ls)
  -> FrontendResult
  -> LoweredModule
  -> CodegenArtifact
       |- WasmArtifact
       `- NativeArtifact
```

- **Wasm backend** は `LoweredModule` から決定的な `.wasm` を生成し、bootstrap と fixed-point 検証の正本を担う
- **Native backend** は同じ `LoweredModule` から `program.o` / `runtime.o` / `linker-response.txt` / `program.native` を生成する将来探索用 backend として保持する
- backend 固有の ABI、section、relocation、linker 連携は codegen に閉じ込め、frontend や lowering へ漏らさない

この設計により、「selfhost compiler の意味」は共通 IR で 1 回だけ定義し、Wasm と Native の違いは最終成果物の作り方に限定できる。もっとも、2026-03-30 の Component Model pivot 以後、Phase 11 の completion gate は Wasmtime embedding + Component Model を正式配布モデルとし、native self-regeneration / native-only 配布は deferred 扱いになった。したがって現時点では Wasm を bootstrap と配布の基準線とし、Native は Phase 13+ 以降の探求対象として読むのが正しい。

### 整数タグ方式の採用

L# の ADT は WasmGC の struct で表現されるが、セルフホストコンパイラではより単純な**整数タグ方式**を採用している:

```lisp
;; トークン種別を整数定数で定義
(defn tok-lparen [] 0)
(defn tok-rparen [] 1)
(defn tok-int [] 10)
(defn tok-symbol [] 20)
(defn tok-defn [] 30)
(defn tok-eof [] 99)
```

この方式を選んだ理由:

- WasmGC の struct/subtyping は複雑で、ブートストラップの初期段階では使いにくい
- 整数タグと Vector の組み合わせで十分な表現力がある
- 実装が単純で、バグを見つけやすい

## セルフホストの構成

セルフホストコンパイラの正本ソースは `selfhost/src/**` にある。2026-03-30 時点で `.ls` は 43 ファイル、合計 12,672 行で構成される。
canonical entrypoint は `selfhost/src/App/Main.ls` で、公開 package ではなく内部 source root として運用する。

| 名前空間 | ファイル数 | 概要 |
|----------|------------|------|
| `App` | 2 | entrypoint と CLI |
| `Syntax` | 8 | 字句解析、構文解析、マクロ展開、Span/Token/AST |
| `Types` | 6 | 型 ADT、型スキーム、制約、型推論 |
| `IR` | 7 | IR 定義、lowering、module graph |
| `Backend` | 10 | Wasm/native backend |
| `Runtime` | 1 | GC |
| `Tools` | 9 | formatter, linter, docs, LSP, test runner |

flat な `selfhost/*.ls` 互換コピーは撤去済みで、モジュール解決は `selfhost/src/**` と dotted namespace を前提にする。

## Host launcher と guest component の役割分担

現在の公開 CLI は、L# compiler 全体を単独の Rust 製 CLI として説明するよりも、**host launcher + embedded guest component** の 2 層で捉えるほうが実装に近い。

```text
single-binary distribution
  = Rust host launcher
      + embedded guest component (.component.wasm)
      + stdlib / host capabilities
```

- **host launcher** は `crates/lsharp-driver` が担い、Wasmtime 上で guest component を起動する
- **embedded guest component** は build-time に `selfhost/src/App/EmbeddedCli.ls` から生成・同梱され、既定の `parse` / `check` / `compile` / `build` / `test` / `review` / `doc-ack` / `doc-check` / `fmt` を担当する。`review` は text surface に加えて `--json` / `--format json` も guest 側で処理し、runtime `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` では `review` / simple `doc-ack` / simple `doc-check` も host 別契約へ暗黙 fallback させず delegation hint に戻す
- **host capability** はファイル I/O や process など、guest 側が単独では扱えない境界を提供する
- **Rust host 側に残る surface** は `install` / `repl` / `lsp` / `doc` と、`compile` / `build` の Rust-only fallback (`--emit-ir`, `web-wasm`, `native`) である

このため、セルフホスト化は「Rust を完全に取り除く」ことではなく、**semantic の中心を guest component 側へ寄せつつ、host launcher を capability provider と配布器として残す** 運用に変わっている。`LSHARP_PATH` はこの構成を壊さずに、外部 host launcher executable / 配置ディレクトリ / `.wasm` / `.component.wasm` へ process-entry delegation するための hook である。

## 各フェーズの L# 実装

### Token.ls: トークン定数

各トークン種別を整数定数として定義する:

```lisp
(defn tok-lparen [] 0)   ;; (
(defn tok-rparen [] 1)   ;; )
(defn tok-lbracket [] 2) ;; [
(defn tok-rbracket [] 3) ;; ]
(defn tok-int [] 10)
(defn tok-symbol [] 20)
(defn tok-defn [] 30)
(defn tok-let [] 31)
(defn tok-if [] 32)
(defn tok-eof [] 99)
```

Rust 版の `TokenKind` enum が 30 以上のバリアントを持つのに対し、セルフホスト版はコンパイラに必要な最小限のトークンのみを定義する。

### Lexer.ls: 字句解析

文字判定関数と走査関数で構成される:

```lisp
;; 空白文字か
(defn is-ws [c]
  (if (== c 32) true    ;; space
    (if (== c 9) true   ;; tab
      (if (== c 10) true ;; newline
        (== c 13)))))    ;; return

;; 数字か (0-9: ASCII 48-57)
(defn is-digit-char [c]
  (if (>= c 48) (<= c 57) false))
```

L# にはパターンマッチの `match` 式があるが、Lexer では `if` の連鎖を使っている。これはセルフホストの初期段階では `match` のコンパイルが複雑すぎるためである。

メインのトークナイズ関数 `lex-one` は現在のバイト位置から1つのトークンを読み取り、トークン種別と終了位置を返す。`tokenize` はこれを繰り返し呼んでトークン列 (Vector) を構築する。

### Parser.ls: 再帰降下パーサー

S 式パーサーは再帰降下で実装する。L# の構文は括弧ベースであるため、パーサーは驚くほど単純になる:

```lisp
;; S 式をパース
(defn parse-sexp [tokens pos]
  (let [tok-kind (vector-get tokens (* pos 3))]
    (if (== tok-kind (tok-lparen))
      ;; 開き括弧: 内部の式列をパース
      (parse-list tokens (+ pos 1))
      ;; それ以外: アトム (整数、シンボル等)
      (parse-atom tokens pos))))
```

### Type.ls と TypeScheme.ls: 型システム

Hindley-Milner 型推論の核心を L# で再実装する:

```lisp
;; 型の種別 (整数タグ方式)
(defn type-con [] 0)  ;; 具体型 (Int, String, Bool)
(defn type-var [] 1)  ;; 型変数 (推論中の未知の型)
(defn type-fun [] 2)  ;; 関数型 ((Int, Int) -> Bool)
```

**Substitution** (型変数の束縛) は HashMap で実装:

```lisp
;; 新しい Substitution を作成
(defn subst-new [] (map-new))

;; 型変数を束縛
(defn bind [subst var ty] (map-insert subst var ty))

;; 型変数を検索
(defn lookup [subst var] (map-get subst var))
```

**単一化 (Unification)** は2つの型を照合し、型変数の束縛を生成する:

```lisp
(defn unify-simple [subst t1 t2]
  ;; t1 が型変数の場合、t2 に束縛
  ;; t2 が型変数の場合、t1 に束縛
  ;; 両方が具体型の場合、名前が一致するか検証
  ...)
```

TypeScheme.ls では `instantiate` (型スキームの具体化) と `generalize` (型の一般化) を実装し、let 多相を実現する。

### Compiler.ls: AST → IR 変換

AST ノードを IR 命令列に変換する:

```lisp
;; 式をコンパイル
(defn compile-expr [ast instructions]
  (let [kind (vector-get ast 0)]
    (if (== kind (ast-lit-int))
      ;; 整数リテラル → I64Const 命令
      (vector-push instructions (ir-i64-const))
      (if (== kind (ast-var))
        ;; 変数参照 → LocalGet 命令
        (vector-push instructions (ir-local-get))
        ...))))
```

### WasmEmit.ls: Wasm バイナリ生成

IR 命令列を Wasm バイナリに変換する。LEB128 エンコーディングを含む:

```lisp
;; LEB128 符号なしエンコーディング
(defn leb128-unsigned [buf value]
  (if (< value 128)
    (vector-push buf value)
    (do
      (vector-push buf (+ 128 (% value 128)))
      (leb128-unsigned buf (/ value 128)))))
```

Wasm バイナリは以下のセクションで構成される:

1. マジックナンバー (`\0asm`)
2. バージョン (1)
3. Type セクション (関数シグネチャ)
4. Function セクション
5. Code セクション (命令列)

### Main.ls: 統合パイプライン

全モジュールを統合して、AST の構築から Wasm バイナリ生成までのパイプラインを実行する:

```lisp
;; 統合パイプライン
;; 1. AST を手動構築 (Lexer/Parser は既知の制限により除外)
;; 2. AST → IR 変換 (Compiler.ls)
;; 3. IR → Wasm バイナリ (WasmEmit.ls)
```

Main.ls は 288 行と最大のファイルで、Token/AST/IR/Compiler/WasmEmit モジュールの定義を統合し、E2E テストで検証される。

## Wasm backend と Native backend の使い分け

セルフホスト化では、backend と配布経路を分けて読む必要がある。2026-03-30 時点の正式な配布モデルは host launcher + embedded guest component であり、`compile` の target もこれに合わせて整理されている。

| 用途 | 主に使う backend | 理由 |
|------|------------------|------|
| stage0 -> stage1 -> stage2 -> stage3 bootstrap | Wasm | 出力が決定的で、`stage2.wasm == stage3.wasm` を byte 単位で比較しやすい |
| fixed-point 検証と CI の正本比較 | Wasm | section / symbol / data の差分を機械的に回収しやすい |
| CLI / server / single-binary 配布 | `wasi-component` | host launcher が `.component.wasm` を埋め込み、guest component を既定起動できる |
| ブラウザ向け配布 | `web-wasm` | WASI import を持たない core `.wasm` を出力する |
| Native backend の調査 | Native | Phase 13+ 以降の探索用。現時点の正式配布経路ではない |

`lsharp compile --target wasi-component` がデフォルトで、`--target wasm` はその alias として扱う。`--target web-wasm` は browser 向け core `.wasm` を指し、現時点では host launcher の Rust fallback 経路が担う。したがって Wasm は「bootstrap と配布の基準線」、`web-wasm` は別 delivery target、Native は deferred backend と読むのが current architecture に合う。

## 既知の制限

現在のセルフホストコンパイラにはいくつかの制限がある:

### 深いネスト if

`if` 式が深くネストすると (10 段階以上)、Wasm のスタック使用量が急増し、コンパイルに失敗する場合がある。Lexer.ls の `is-symbol-start` 関数がこの問題に直面した。

### 相互再帰

関数の前方参照 (定義前の関数を呼び出す) は現在サポートされていない。これは関数インデックスの割り当てが定義順に行われるためである。

### 高度な型機能

HKT、GADT、トレイト制約はセルフホストコンパイラでは未使用。整数タグ方式で代替している。

### host launcher / component 境界

公開 CLI の全サブコマンドが guest component 側へ完全移行したわけではない。`install` / `repl` / `lsp` / `doc` は Rust host 側の built-in surface が残っており、`compile` / `build` でも `--emit-ir` / `web-wasm` / `native` は Rust fallback に戻る。また `review` は text surface に加えて `--json` / `--format json` まで guest default path へ寄った一方、`review --help` / `doc-ack --help` / `doc-check --help` などの clap surface や `doc-ack` / `doc-check` の richer argv shape は host launcher 側に残る。`LSHARP_DISABLE_EMBEDDED_COMPONENT=1` はこの guest-backed subset を止める safety valve だが、`review` / simple `doc-ack` / simple `doc-check` を host の別契約へ切り替えるための fallback ではなく、外部 selfhost への delegation hint を復帰させるためのスイッチとして扱う。

### bootstrap fixed-point の未完了範囲

最小 subset では `stage1.wasm -> stage2.wasm` の実生成が確認済みだが、full input set に対する `stage1 -> stage2 -> stage3` の実体生成・比較・固定点成立は未提示である。したがって「セルフホストは完了した」と断定するより、「fixed-point の意味は定義済みで、full gate は継続追跡中」と表現するのが適切である。

## ブートストラップ検証

E2E テストで各モジュールと統合パイプラインを検証している:

```rust
// 個別モジュールの E2E テスト (9件)
#[test]
fn test_selfhost_token() { ... }
#[test]
fn test_selfhost_lexer() { ... }
#[test]
fn test_selfhost_type() { ... }
// ...

// 統合パイプライン E2E テスト (2件)
#[test]
fn test_selfhost_main_pipeline() { ... }
#[test]
fn test_selfhost_main_ast_to_wasm() { ... }
```

fixed-point 検証の設計は、現在は Wasm bootstrap を正本として読む。

1. **Wasm bootstrap の正本検証**
    - `stage0 -> stage1.wasm -> stage2.wasm -> stage3.wasm` を生成する
    - 固定点条件は `stage2.wasm == stage3.wasm`
    - 不一致時は raw wasm bytes, exported symbol list, data section bytes, compiler diagnostics の 4 層に分けて原因を切り分ける

Component Model pivot 以前は Native backend の追従検証も completion gate に含めていたが、現在は deferred である。このため、セルフホスト化の主要ゲートは「stage1 が動くこと」ではなく、**Wasm 側で `stage2.wasm == stage3.wasm` の fixed-point が full input set で閉じること**にある。

現時点の到達点:

- Rust 版コンパイラが `selfhost/src/**/*.ls` をコンパイルして stage1.wasm を生成できる
- stage1.wasm が簡単なプログラム (整数演算、条件分岐) をコンパイルできる
- 最小 subset では `stage1.wasm -> stage2.wasm` の実生成まで確認されている
- full input set に対する `stage1.wasm -> stage2.wasm -> stage3.wasm` の実体生成・比較と `stage2.wasm == stage3.wasm` の固定点成立は、引き続き Phase 11 の完了条件として追跡中である
- Native backend の自己再生成と native-only 配布経路は、Phase 13+ 以降の deferred 項目として保持される

## セルフホスティングから見える言語の課題

セルフホスティングは言語自体のドッグフーディングである。L# で L# コンパイラを書く過程で、以下の課題が浮き彫りになった:

1. **パターンマッチの深さ制限**: 複雑なマッチが必要な箇所で if チェインに頼らざるを得ない
2. **文字列操作の不足**: バイナリ生成に必要な低レベルのバイト操作が煩雑
3. **エラー処理**: Result 型なしでのエラー伝播が困難
4. **デバッグ**: print デバッグ以外のデバッグ手段がない

これらの課題は、今後の言語改善の重要なフィードバックとなる。セルフホスティングは単なる技術的挑戦ではなく、言語設計を改善するための実践的なプロセスである。
