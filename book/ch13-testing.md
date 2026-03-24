# テスト戦略 -- コンパイラのテスト手法

## コンパイラのテストが難しい理由

コンパイラは多段階のパイプラインで構成される。各段階の出力が次の段階の入力になるため、一つのバグが後段で予想外の形で現れることがある。

L# のパイプライン:
```
ソース → Lexer → Parser → 型推論 → IR → Wasm
```

例えば、パーサーが演算子の結合性を間違えると、型推論は通るのにコード生成で不正な計算結果になる。逆に、型推論のバグがあっても IR 降位で偶然キャッチされることもある。このような多段階のバグを体系的にテストするのがコンパイラテストの課題である。

## TDD ワークフロー

L# プロジェクトでは TDD (テスト駆動開発) を必須としている。新しい機能の実装は以下の手順で進める:

### RED: テストを先に書く

新しい言語機能を追加する前に、まずテストを書く。例えば「レコード型のフィールドアクセス」を実装する場合:

```rust
#[test]
fn test_record_field_access() {
    let output = compile_and_run(
        "(type Point (record (: x Int) (: y Int)))
         (defn main []
           (let [p {Point x 10 y 20}]
             (print (Point.x p))))"
    );
    assert_eq!(output.trim(), "10");
}
```

この時点では `compile_and_run` はコンパイルエラーになる。レコード型の構文もフィールドアクセスもまだ実装されていないからである。これが RED の状態。

### GREEN: 実装を書く

テストが通るように実装する。レコード型の場合:

1. Lexer に `Record` トークンを追加
2. Parser に `(record ...)` 構文を追加
3. 型推論に `RecordInfo` の登録を追加
4. IR にフィールドアクセスの命令を追加
5. Codegen に対応する Wasm 命令を追加

各段階で `cargo test` を実行し、テストが通ったら GREEN。

### REFACTOR: リファクタリング

テストが通った状態でコードを整理する。テストが壊れないことを確認しながらリファクタリングを行う。

## テストのピラミッド

L# は 3 層のテスト戦略を採用している:

### 1. ユニットテスト (各クレートの内部テスト)

各クレートの `#[cfg(test)]` モジュールで個別の機能をテストする:

```rust
// Lexer のテスト
#[test]
fn test_simple_addition() {
    let tokens = lex("(+ 1 2)");
    assert_eq!(tokens, vec![
        LParen, Symbol("+"), Int(1), Int(2), RParen, Eof
    ]);
}

// Parser のテスト
#[test]
fn test_fib() {
    let prog = parse(
        "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))",
    );
    assert_eq!(prog.decls.len(), 1);
}

// 型推論のテスト
#[test]
fn test_recursive() {
    let result = infer_one(
        "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))",
    );
    assert_eq!(result, "(Int) -> Int");
}
```

ユニットテストは各段階の出力を直接検証する。パーサーの出力 (AST)、型推論の結果 (型)、IR の命令列がそれぞれ期待通りであることを確認する。

### 2. スナップショットテスト (insta)

IR の出力は複雑で、手書きの期待値を維持するのが難しい。**insta** クレートを使ったスナップショットテストで解決する:

```rust
#[test]
fn test_lower_arithmetic() {
    let ir = lower("(defn add [x y] (+ x y))");
    insta::assert_snapshot!(ir.dump());
}
```

スナップショットテストの流れ:

1. **初回実行時**: 実際の出力がスナップショットファイル (`.snap`) に保存される
2. **以降の実行**: 出力がスナップショットと比較される。一致すればパス、不一致なら失敗
3. **意図的な変更時**: `cargo insta review` で差分を確認し、承認する

```bash
$ cargo insta review
--- old
+++ new
-  I64Const(42)
+  I64Const(42)
+  Drop

Accept? [y/n/s]
```

スナップショットテストの利点は、IR の出力全体を検証できることである。手書きの期待値では見落としがちな細かい命令の変化も検出できる。

### 3. E2E テスト (wasmtime 実行)

パイプライン全体の正しさを検証する最も強力なテスト。`crates/lsharp-wasm/tests/e2e.rs` に集約されている:

```rust
#[test]
fn test_wasi_fibonacci() {
    let output = compile_and_run(
        "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
         (defn main [] (print (fib 10)))"
    );
    assert_eq!(output.trim(), "55");
}
```

このテストは以下の全段階を一気通貫で検証する:

- ソースコードが正しくパースされる
- 型推論が成功する
- IR が正しく生成される
- Wasm バイナリが正しく出力される
- 実行結果が期待通りである

## E2E テストヘルパー

E2E テストには複数のヘルパー関数が用意されている。テストの目的に応じて使い分ける:

### compile_and_run

フルパイプラインを実行し、標準出力を返す。最も一般的なヘルパー:

```rust
fn compile_and_run(source: &str) -> String
```

### compile_and_run_with_dir

ファイルシステムアクセスが必要なテスト用。マルチモジュールのテストで使用:

```rust
fn compile_and_run_with_dir(source: &str, dir: &Path) -> String
```

### compile_only

Wasm バイナリの生成までを検証する。実行は行わない。GC 機能など、wasmtime でサポートされていない機能のテストに使用:

```rust
fn compile_only(source: &str) -> Vec<u8>
```

### typecheck_only

型推論までを検証する。IR 生成・コード生成は行わない:

```rust
fn typecheck_only(source: &str) -> TypeEnv
```

### should_fail_typecheck / should_fail_parse

「正しくエラーになること」を検証する:

```rust
fn should_fail_typecheck(source: &str) -> TypeError
fn should_fail_parse(source: &str) -> ParseError
```

使用例:

```rust
#[test]
fn test_type_error_mismatch() {
    should_fail_typecheck("(defn bad [] (+ 1 true))");
}

#[test]
fn test_parse_error_unclosed() {
    should_fail_parse("(defn bad [");
}
```

## エラーケースのテスト

コンパイラのテストでは「正しく動くこと」だけでなく「正しくエラーになること」も重要である。型エラー、構文エラー、未定義変数、引数の数の不一致——これらが全て適切なエラーメッセージで報告されることを検証する:

```rust
#[test]
fn test_undefined_var() {
    should_fail_typecheck("(defn bad [] x)");
}

#[test]
fn test_arity_mismatch() {
    should_fail_typecheck(
        "(defn f [x y] (+ x y))
         (defn main [] (f 1))"
    );
}
```

## メタデータテスト

L# のメタデータアノテーション (`:example`, `:invariant`) からテストを自動生成する機能がある:

```lisp
(defn abs [n] : Int
  :doc "絶対値を返す"
  :example [(== (abs 5) 5)]
  :example [(== (abs -3) 3)]
  :invariant [(>= (abs n) 0)]
  (if (< n 0) (- 0 n) n))
```

`cargo run -- test examples/abs.ls` を実行すると、`:example` の式がコンパイル・実行され、結果が `true` であることが検証される。`:invariant` はランダムな入力で複数回テストされる (Property-Based Testing の一種)。

メタデータテストは以下の利点を持つ:

1. **ドキュメントとテストの一体化**: ドキュメントに書いた例がそのままテストになる
2. **不変条件の明示**: 関数が満たすべき性質を型定義の近くに書ける
3. **自動テスト生成**: テスト用のボイラープレートが不要

## テストカバレッジの推移

L# のテスト数はプロジェクトの成長に伴い増加してきた:

| Phase | テスト数 | 主な追加内容 |
|-------|---------|-------------|
| 初期 (Phase 0) | 63 | 基本的な算術、再帰、パターンマッチ |
| Phase 1-2 | 120 | 文字列操作、動的コレクション |
| Phase 3-4 | 170 | レコード型、モジュール、型エイリアス |
| Phase 5 | 200 | トレイト、制約付き型 |
| Phase 6-7 | 230 | HKT, GADT, 標準ライブラリ |
| Phase 8 | 265 | セルフホスティング、LSP |

現在のテスト構成:

| クレート | テスト数 | 種別 |
|----------|----------|------|
| lsharp-syntax | 61 | ユニット (Lexer + Parser) |
| lsharp-types | 120 | ユニット (型推論 + 制約 + メタデータ) |
| lsharp-ir | 46 | スナップショット + エラー |
| lsharp-wasm | 15 | codegen + E2E |
| lsharp-docs | 18 | ドキュメント追跡 |
| lsharp-driver | 5 | 統合テスト |
| **合計** | **265** | |

`lsharp-types` のテストが最も多いのは、レコード型推論、型エイリアス展開、制約付き型検証、トレイト解決など多くの機能が型推論層で処理されるためである。

## 新機能追加時のテスト手順

L# に新しい言語機能を追加する際は、以下の順序でテストを追加する:

1. **E2E テスト**: 最終的にどう動くべきかを記述 (RED)
2. **Lexer テスト**: 新しいトークンが正しく生成されるか
3. **Parser テスト**: 新しい構文が正しく AST になるか
4. **型推論テスト**: 新しい構文の型が正しく推論されるか
5. **IR スナップショット**: 新しい構文の IR 出力を記録する
6. **E2E テスト再実行**: 全段階を通して正しく動作するか (GREEN)

この手順により、どの段階でバグが入り込んだかを素早く特定できる。E2E テストを最初に書くことで、実装のゴールが明確になる。

## ベンチマーク

テスト以外に、パフォーマンスのベンチマークも実施している (`crates/lsharp-wasm/benches/`):

```rust
// コンパイラの各フェーズを個別に計測
fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_fib", |b| {
        b.iter(|| parse(FIB_SOURCE));
    });
}

fn bench_infer(c: &mut Criterion) {
    c.bench_function("infer_fib", |b| {
        b.iter(|| infer(FIB_SOURCE));
    });
}
```

Criterion クレートを使い、各フェーズ (parse, infer, lower, codegen) とフルパイプラインのベンチマークを計測する。HTML レポートが生成され、性能の変化を視覚的に追跡できる。
