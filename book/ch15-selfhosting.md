# セルフホスティング -- L# で L# コンパイラを書く

## ブートストラップとは何か

コンパイラのセルフホスティング (self-hosting) とは、コンパイラを自身がコンパイルする言語で書くことである。C コンパイラは C で書かれ、Rust コンパイラは Rust で書かれている。

ブートストラップの正本フローは以下のようになる:

```
Stage 0: 既存のコンパイラ (Rust 版 L# コンパイラ)
    ↓ selfhost/*.ls をコンパイル
Stage 1: L# で書かれた L# コンパイラ (stage1.wasm)
    ↓ stage1.wasm で selfhost/*.ls を再コンパイル
Stage 2: stage1 が生成した L# コンパイラ (stage2.wasm)
    ↓ stage2.wasm で selfhost/*.ls を再コンパイル
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

### Native backend を含む multi-backend 設計

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
- **Native backend** は同じ `LoweredModule` から `program.o` / `runtime.o` / `linker-response.txt` / `program.native` を生成し、配布とローカル実行を担う
- backend 固有の ABI、section、relocation、linker 連携は codegen に閉じ込め、frontend や lowering へ漏らさない

この設計により、「selfhost compiler の意味」は共通 IR で 1 回だけ定義し、Wasm と Native の違いは最終成果物の作り方に限定できる。セルフホスト章では Wasm を bootstrap の基準線、Native を完成後の常用 backend として扱う。

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

セルフホストコンパイラは `selfhost/` ディレクトリに 10 ファイル、合計 1,455 行で構成される:

| ファイル | 行数 | 役割 |
|----------|------|------|
| Token.ls | 60 | トークン定数定義 |
| Lexer.ls | 189 | 字句解析 (文字走査) |
| AST.ls | 56 | AST ノード種別定義 |
| Parser.ls | 93 | 再帰降下パーサー |
| Type.ls | 179 | 型 ADT (Con, Var, Fun) |
| TypeScheme.ls | 192 | 多相型 (instantiate, generalize) |
| IR.ls | 63 | 中間表現定義 |
| Compiler.ls | 165 | AST → IR 変換 |
| WasmEmit.ls | 170 | Wasm バイナリ生成 |
| Main.ls | 288 | 統合パイプライン |

これは Rust 版の約 8% のコード量だが、コンパイラの核心的なアルゴリズムを全て含んでいる。

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

セルフホスト化では、両 backend は競合するのではなく役割分担する。

| 用途 | 主に使う backend | 理由 |
|------|------------------|------|
| stage0 -> stage1 -> stage2 -> stage3 bootstrap | Wasm | 出力が決定的で、`stage2.wasm == stage3.wasm` を byte 単位で比較しやすい |
| fixed-point 検証と CI の正本比較 | Wasm | section / symbol / data の差分を機械的に回収しやすい |
| 日常開発での高速なローカル実行 | Native | `program.native` を直接起動でき、WASI runner への依存を減らせる |
| エンドユーザー向け配布 | Native | プラットフォーム別バイナリを配布でき、CLI/LSP/REPL を単独実行しやすい |
| backend 間の観測差分チェック | Wasm + Native | 同じ `LoweredModule` から生成した結果を比較し、exit code / stdout / stderr / 生成物 / diagnostics の一致を確認する |

実装順序としては、まず Wasm backend で bootstrap 閉路を安定化し、その後 Native backend に同じ `LoweredModule` を食べさせて self-regeneration と配布経路を閉じる。つまり Wasm は「検証の物差し」、Native は「常用・配布の出口」である。

## 既知の制限

現在のセルフホストコンパイラにはいくつかの制限がある:

### 深いネスト if

`if` 式が深くネストすると (10 段階以上)、Wasm のスタック使用量が急増し、コンパイルに失敗する場合がある。Lexer.ls の `is-symbol-start` 関数がこの問題に直面した。

### 相互再帰

関数の前方参照 (定義前の関数を呼び出す) は現在サポートされていない。これは関数インデックスの割り当てが定義順に行われるためである。

### 高度な型機能

HKT、GADT、トレイト制約はセルフホストコンパイラでは未使用。整数タグ方式で代替している。

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

fixed-point 検証の設計は次の 2 段構えである。

1. **Wasm bootstrap の正本検証**
   - `stage0 -> stage1.wasm -> stage2.wasm -> stage3.wasm` を生成する
   - 固定点条件は `stage2.wasm == stage3.wasm`
   - 不一致時は raw wasm bytes, exported symbol list, data section bytes, compiler diagnostics の 4 層に分けて原因を切り分ける
2. **Native backend の追従検証**
   - `stage1-native -> stage2-native -> stage3-native` を生成し、stage2-native と stage3-native の観測値が一致することを確認する
   - Wasm/native differential test では exit code、stdout、stderr、generated file bytes、diagnostics JSON の 5 観測点を比較する

このため、セルフホスト化の「完了」は単に stage1 が動くことではなく、Wasm 側で fixed-point が閉じ、Native 側でも同じ compiler core を使って自己再生成と差分ゼロ検証が通ることを意味する。

現時点の到達点:

- Rust 版コンパイラが selfhost/*.ls をコンパイルして stage1.wasm を生成できる
- stage1.wasm が簡単なプログラム (整数演算、条件分岐) をコンパイルできる
- 最小 subset では `stage1.wasm -> stage2.wasm` の実生成まで確認されている
- full input set に対する `stage2.wasm == stage3.wasm` の固定点成立と `stage1-native -> stage2-native -> stage3-native` の自己再生成は、引き続き Phase 11 の完了条件として追跡中である

## セルフホスティングから見える言語の課題

セルフホスティングは言語自体のドッグフーディングである。L# で L# コンパイラを書く過程で、以下の課題が浮き彫りになった:

1. **パターンマッチの深さ制限**: 複雑なマッチが必要な箇所で if チェインに頼らざるを得ない
2. **文字列操作の不足**: バイナリ生成に必要な低レベルのバイト操作が煩雑
3. **エラー処理**: Result 型なしでのエラー伝播が困難
4. **デバッグ**: print デバッグ以外のデバッグ手段がない

これらの課題は、今後の言語改善の重要なフィードバックとなる。セルフホスティングは単なる技術的挑戦ではなく、言語設計を改善するための実践的なプロセスである。
