# テスト戦略 -- コンパイラのテスト手法

## コンパイラのテストが難しい理由

コンパイラは多段階のパイプラインで構成される。各段階の出力が次の段階の入力になるため、一つのバグが後段で予想外の形で現れることがある。

L# のパイプライン:
```
ソース → Lexer → Parser → 型推論 → IR → Wasm
```

このパイプラインの各段階をどのようにテストするか、L# の実践的なアプローチを解説する。

## テストのピラミッド

L# は 3 層のテスト戦略を採用している:

### 1. ユニットテスト (各クレートの内部テスト)

各クレートの `#[cfg(test)]` モジュールで個別の機能をテストする:

```rust
// Lexer のテスト
#[test]
fn test_simple_addition() {
    let tokens = lex("(+ 1 2)");
    assert_eq!(tokens, vec![LParen, Symbol("+"), Int(1), Int(2), RParen, Eof]);
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
1. 初回実行時: 実際の出力がスナップショットファイルに保存される
2. 以降の実行: 出力がスナップショットと比較される
3. 変更時: `cargo insta review` で差分を確認し、意図的な変更なら承認する

### 3. E2E テスト (wasmtime 実行)

パイプライン全体の正しさを検証する最も強力なテスト:

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

## エラーケースのテスト

コンパイラのテストでは「正しく動くこと」だけでなく「正しくエラーになること」も重要:

```rust
#[test]
fn test_type_error_mismatch() {
    let result = infer("(defn bad [] (+ 1 true))");
    assert!(result.is_err());
}

#[test]
fn test_undefined_var() {
    let result = infer("(defn bad [] x)");
    assert!(result.is_err());
}
```

## L# のテストカバレッジ

各クレートのテスト数 (2026-03-22 時点):

| クレート | テスト数 | 種別 |
|----------|----------|------|
| lsharp-syntax | 61 | ユニット (Lexer + Parser) |
| lsharp-types | 120 | ユニット (型推論 + 制約 + メタデータ) |
| lsharp-ir | 46 | スナップショット + エラー |
| lsharp-wasm | 15 | codegen + E2E |
| lsharp-docs | 18 | ドキュメント追跡 |
| lsharp-driver | 5 | 統合テスト |
| **合計** | **265** | |

Phase 1〜5 の機能追加に伴い、テスト数は初期の 63 から 265 へと約 4 倍に増加した。特に `lsharp-types` のテストが大幅に増えているのは、レコード型推論、型エイリアス展開、制約付き型検証、トレイト解決など多くの機能が型推論層で処理されるためである。

## 新機能追加時のテスト手順

L# に新しい言語機能を追加する際は、以下の順序でテストを追加する:

1. **Lexer テスト**: 新しいトークンが正しく生成されるか
2. **Parser テスト**: 新しい構文が正しく AST になるか
3. **型推論テスト**: 新しい構文の型が正しく推論されるか
4. **IR スナップショット**: 新しい構文の IR 出力を記録する
5. **E2E テスト**: 新しい機能が実行時に正しく動作するか

この手順により、どの段階でバグが入り込んだかを素早く特定できる。
