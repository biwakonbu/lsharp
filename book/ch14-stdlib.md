# 標準ライブラリ -- L# 自身で書かれた基盤

## 標準ライブラリの設計思想

多くのプログラミング言語では、標準ライブラリはホスト言語 (C, C++, Rust など) で実装される。L# はこれとは異なるアプローチを採る。**標準ライブラリを L# 自身で記述する**という方針だ。

この設計には三つの意図がある:

1. **ブートストラップへの布石** -- セルフホスティングコンパイラ (第 15 章) を実現するためには、言語自身でデータ構造やアルゴリズムを記述できなければならない。標準ライブラリは、その基盤となる。

2. **言語機能の検証** -- 標準ライブラリが自言語で書ければ、ADT、パターンマッチ、高階関数、再帰といった言語機能が実用に耐えることの証明になる。

3. **教育的価値** -- 利用者が標準ライブラリのソースコードを読むことで、L# のイディオムを学べる。

L# の標準ライブラリは `stdlib/` ディレクトリに配置されている。9 つのモジュールで構成され、それぞれが独立した `.ls` ファイルとして存在する:

| モジュール | ファイル | 役割 |
|-----------|----------|------|
| Core | `stdlib/Core.ls` | 基本関数、Option/Result 型 |
| List | `stdlib/List.ls` | コンスリスト |
| String | `stdlib/String.ls` | 文字列操作 |
| Vector | `stdlib/Vector.ls` | 動的配列 |
| Map | `stdlib/Map.ls` | ハッシュマップ |
| Set | `stdlib/Set.ls` | 集合 |
| Char | `stdlib/Char.ls` | 文字判定 |
| IO | `stdlib/IO.ls` | 入出力 |
| Debug | `stdlib/Debug.ls` | デバッグ |

各モジュールはビルトイン関数 (Rust 側でコンパイラに組み込まれた関数) の上に高レベルな操作を構築する階層構造を持つ。

## Core -- 基本関数と代数的データ型

`stdlib/Core.ls` は標準ライブラリの土台である。数学関数、`Option` 型、`Result` 型、関数合成ユーティリティを提供する。

### 基本数学関数

```lisp
;; 絶対値
(defn abs [x] (if (< x 0) (- 0 x) x))

;; 最大値
(defn max [a b] (if (> a b) a b))

;; 最小値
(defn min [a b] (if (< a b) a b))

;; クランプ: lo <= x <= hi の範囲に収める
(defn clamp [x lo hi] (max lo (min x hi)))
```

`abs` の実装に注目してほしい。`(- 0 x)` という表現は、L# に単項マイナス演算子がないことを反映している。S 式構文では `-` は二項演算子であり、`0` からの減算で符号反転を実現する。

排他的論理和 `xor` もある:

```lisp
;; 排他的論理和
(defn xor [a b] (if a (if b 0 1) (if b 1 0)))
```

L# では真偽値は整数として表現されるため (0 = false, 非 0 = true)、`xor` の戻り値も `0` / `1` の整数になる。

### Option 型

値の有無を表す代数的データ型。Rust の `Option<T>` に相当する:

```lisp
(type (Option a) (Some a) None)

;; Option から値を取り出す。None の場合はデフォルト値を返す
(defn unwrap [opt default]
  (match opt
    [(Some x) x]
    [None default]))

;; Option に関数を適用する
(defn map-option [f opt]
  (match opt
    [(Some x) (Some (f x))]
    [None None]))
```

`(type (Option a) (Some a) None)` は型変数 `a` を持つ多相 ADT を定義する。`Some` は 1 フィールドのコンストラクタ、`None` はフィールドなしのコンストラクタだ。

`unwrap` 関数のシグネチャは型推論により `(Option a) -> a -> a` と推論される。パターンマッチで `Some` と `None` を網羅的に処理する。

### Result 型

成功/失敗を表す型。エラーハンドリングの基盤となる:

```lisp
(type (Result a e) (Ok a) (Err e))

;; Result から成功値を取り出す。Err の場合はデフォルト値を返す
(defn unwrap-ok [res default]
  (match res
    [(Ok x) x]
    [(Err _) default]))

;; Result に関数を適用する (Ok の場合のみ)
(defn map-result [f res]
  (match res
    [(Ok x) (Ok (f x))]
    [(Err e) (Err e)]))
```

`Result` は 2 つの型変数 `a` (成功値) と `e` (エラー値) を持つ。`map-result` は `Ok` の中身だけを変換し、`Err` はそのまま伝搬する -- モナディックな操作の基本パターンだ。

### 関数合成ユーティリティ

```lisp
;; 恒等関数
(defn identity [x] x)

;; 定数関数: 常に x を返す関数を返す
(defn const [x] (fn [_] x))

;; 関数を 2 回適用する
(defn twice [f x] (f (f x)))
```

`const` はクロージャを返す高階関数だ。`(fn [_] x)` のワイルドカードパターン `_` は引数を無視することを示す。この関数は `(const 42)` のように部分適用して使うことを想定している。

## List -- コンスリスト

`stdlib/List.ls` は Lisp の伝統を受け継ぐ再帰的リスト型を提供する。

### 型定義と基本操作

```lisp
(type (List a) (Cons a (List a)) Nil)
```

`(List a)` は型変数 `a` を持つ多相 ADT で、`Cons` (要素と残りのリスト) と `Nil` (空リスト) の二つのコンストラクタを持つ。Wasm のリニアメモリ上では、`Cons` はタグ (0) + 要素 (i64) + 次ノードへのポインタ (i32) の構造体として配置される。

```lisp
;; リストの長さを返す
(defn length [xs]
  (match xs
    [Nil 0]
    [(Cons _ t) (+ 1 (length t))]))

;; リストの先頭要素を返す (空リストの場合はデフォルト値)
(defn head [xs default]
  (match xs
    [Nil default]
    [(Cons h _) h]))
```

`length` は末尾再帰ではないため、長大なリストではスタックオーバーフローの可能性がある。実用上は `Vector` モジュールの使用が推奨される。

### 高階関数

関数型プログラミングの三大操作 -- `map`, `filter`, `fold` を提供する:

```lisp
;; 各要素に関数を適用する
(defn map [f xs]
  (match xs
    [Nil Nil]
    [(Cons h t) (Cons (f h) (map f t))]))

;; 条件を満たす要素だけを残す
(defn filter [f xs]
  (match xs
    [Nil Nil]
    [(Cons h t) (if (f h) (Cons h (filter f t)) (filter f t))]))

;; 左畳み込み
(defn fold [f init xs]
  (match xs
    [Nil init]
    [(Cons h t) (fold f (f init h) t)]))
```

`fold` は末尾再帰の形になっている点に注目してほしい。最後の式が自分自身の呼び出しになっているため、TCO (末尾呼び出し最適化) の対象にできる。一方、`map` と `filter` は `Cons` でラップしてから再帰するため、末尾再帰ではない。

`fold` を使えば多くの操作を表現できる:

```lisp
;; リストを逆順にする
(defn reverse [xs]
  (fold (fn [acc x] (Cons x acc)) Nil xs))

;; 全要素の合計 (Int リスト用)
(defn sum [xs]
  (fold (fn [acc x] (+ acc x)) 0 xs))
```

### スライス操作

```lisp
;; n 番目の要素を取得 (0-indexed, 範囲外はデフォルト値)
(defn nth [xs n default]
  (match xs
    [Nil default]
    [(Cons h t) (if (== n 0) h (nth t (- n 1) default))]))

;; 先頭 n 個を取得
(defn take [n xs]
  (if (<= n 0) Nil
    (match xs
      [Nil Nil]
      [(Cons h t) (Cons h (take (- n 1) t))])))
```

## String -- 文字列操作

`stdlib/String.ls` はビルトイン文字列関数の上に高レベルな操作を構築する。ビルトインとして `string-length`, `string-concat`, `string-eq`, `string-char-at`, `substring`, `int-to-string` が提供されている。

### 判定関数

```lisp
;; 文字列が空かどうか
(defn string-empty? [s]
  (== (string-length s) 0))

;; 文字列が指定のプレフィックスで始まるか
(defn starts-with [s prefix]
  (if (> (string-length prefix) (string-length s))
    false
    (string-eq (substring s 0 (string-length prefix)) prefix)))
```

`starts-with` の実装は、まずプレフィックスが元の文字列より長くないことを確認し、先頭から同じ長さの部分文字列を切り出して比較する。`string-eq` はビルトインの文字列等価判定だ。

### 検索関数

```lisp
;; string-index-of の内部ヘルパー: 位置 i から検索
(defn string-search-from [haystack needle hlen nlen i]
  (if (> (+ i nlen) hlen)
    (- 0 1)
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      i
      (string-search-from haystack needle hlen nlen (+ i 1)))))

;; 文字列内に部分文字列が含まれるか (O(n*m))
(defn string-index-of [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (> nlen hlen)
      (- 0 1)
      (string-search-from haystack needle hlen nlen 0))))
```

`string-index-of` はナイーブな線形探索で、計算量は O(n*m) だ。KMP や Boyer-Moore のような高度なアルゴリズムは、L# の現在の言語機能で実装するには配列操作が必要であり、将来の課題とされている。

戻り値の `-1` は `(- 0 1)` で表現されている。これは先述のとおり、単項マイナスが存在しないためだ。

## Char -- 文字判定

`stdlib/Char.ls` は ASCII 文字コードに基づく文字判定関数を提供する。L# では文字は整数 (ASCII コードポイント) として扱われる。

```lisp
;; 数字か (0-9: ASCII 48-57)
(defn is-digit [c]
  (if (>= c 48)
    (<= c 57)
    false))

;; 空白文字か (space=32, tab=9, newline=10, return=13)
(defn is-whitespace [c]
  (if (== c 32) true
    (if (== c 9) true
      (if (== c 10) true
        (== c 13)))))
```

`is-whitespace` の実装はネストした `if` の連鎖になっている。L# には `or` 演算子が短絡評価されるかどうかの保証がないため、明示的に `if` を連鎖させる方が安全だ。この書法はセルフホスティングコンパイラの Lexer (第 15 章) でも多用される。

## Wasm 上のデータ構造

### Vector -- 動的配列

`stdlib/Vector.ls` は Wasm のリニアメモリ上に実装された動的配列のラッパーを提供する。ビルトインとして `vector-new`, `vector-push`, `vector-get`, `vector-set`, `vector-length` が用意されている。

Vector の内部構造は、リニアメモリ上で以下のように配置される:

```
アドレス  内容
+0        length (i32)   -- 現在の要素数
+4        capacity (i32) -- 確保済みのスロット数
+8        data[0] (i64)  -- 要素 0
+16       data[1] (i64)  -- 要素 1
...
```

`vector-push` はキャパシティを超えた場合、新しいメモリ領域を確保してデータをコピーする。Rust の `Vec<T>` と同じ倍増戦略を採用しているため、償却計算量は O(1) だ。

高階関数は内部ヘルパーのパターンで実装される:

```lisp
;; 各要素に関数を適用して新しいベクタを返す
(defn vector-map-impl [f v i len result]
  (if (>= i len)
    result
    (vector-map-impl f v (+ i 1) len
      (vector-push result (f (vector-get v i))))))

(defn vector-map [f v]
  (vector-map-impl f v 0 (vector-length v) (vector-new (vector-length v))))
```

`vector-map-impl` は末尾再帰で実装されている。インデックス `i` を 0 から `len` まで進めながら、新しい Vector に変換後の要素を追加していく。公開関数 `vector-map` は初期値を設定してヘルパーを呼び出すだけだ。

`vector-fold` も同じパターンだ:

```lisp
;; 左畳み込み
(defn vector-fold-impl [f acc v i len]
  (if (>= i len)
    acc
    (vector-fold-impl f (f acc (vector-get v i)) v (+ i 1) len)))

(defn vector-fold [f init v]
  (vector-fold-impl f init v 0 (vector-length v)))
```

List の `fold` と比較すると、パターンマッチの代わりにインデックスベースの走査を使う点が異なる。Vector は O(1) のランダムアクセスが可能なため、`nth` のような操作が高速だ。

### Map -- ハッシュマップ

`stdlib/Map.ls` はビルトインのハッシュマップ操作のラッパーを提供する。ビルトインとして `map-new`, `map-insert`, `map-get`, `map-contains?`, `map-remove`, `map-size` がある。

```lisp
;; マップが空かどうか
(defn map-empty? [m]
  (== (map-size m) 0))

;; キーに関数を適用してデフォルト値を返す (キーが存在しない場合)
(defn map-get-or [m key default]
  (if (map-contains? m key)
    (map-get m key)
    default))
```

Wasm のリニアメモリ上では、Map はオープンアドレス法のハッシュテーブルとして実装されている。各エントリは `(hash, key, value, occupied)` のタプルで、キーと値は i64 として格納される。文字列キーの場合はヒープ上の文字列ポインタがキーとして使われる。

### Set -- 集合

`stdlib/Set.ls` は Map を内部データ構造として利用し、集合を実現する:

```lisp
;; 空の集合を作る
(defn set-new []
  (map-new))

;; 要素を追加
(defn set-add [s x]
  (map-insert s x 1))

;; 要素を含むか
(defn set-contains? [s x]
  (map-contains? s x))
```

値として常に `1` を格納し、キーの有無で集合のメンバーシップを表現する。この手法は Rust の `HashSet` が `HashMap<K, ()>` の薄いラッパーであることと同じ発想だ。

## IO と Debug

### IO モジュール

`stdlib/IO.ls` はファイル入出力のラッパーを提供する。ビルトインとして `read-file`, `write-file`, `file-exists?` がある:

```lisp
;; ファイルの内容を読み込み、デフォルト値で返す (ファイルが存在しない場合)
(defn read-file-or [path default]
  (if (file-exists? path)
    (read-file path)
    default))
```

WASI (WebAssembly System Interface) 上のファイルアクセスはサンドボックス化されており、明示的に許可されたディレクトリのみアクセスできる。`wasmtime` で実行する場合、`--dir=.` フラグでカレントディレクトリのアクセスを許可する。

### Debug モジュール

`stdlib/Debug.ls` はデバッグ用ユーティリティを提供する:

```lisp
;; 値をそのまま出力して返す (デバッグ用)
(defn debug-print [x]
  (do
    (print x)
    x))

;; 条件が真でなければ 0 を返す
(defn assert [cond]
  (if cond 0 0))

;; 二値が等しいか検証
(defn assert-eq [a b]
  (assert (== a b)))
```

`debug-print` は値を出力した後、同じ値を返す。`do` ブロックの最後の式が戻り値になるため、パイプラインの途中に挿入してデバッグ出力を追加できる。

`assert` の実装は現時点では簡素で、失敗時も `0` を返すだけだ。コメントにもあるとおり、将来的に `panic` ビルトインが追加されれば、異常終了に置き換えられる予定だ。

## 標準ライブラリのテスト

各モジュールには `main` 関数がエントリポイントとして定義されている。これはライブラリテスト用で、個別にコンパイル・実行して動作を確認できる:

```bash
# Core.ls のテスト
cargo run -- compile stdlib/Core.ls -o core.wasm
wasmtime core.wasm
```

E2E テストとしては、`crates/lsharp-wasm/tests/e2e.rs` にパイプライン全体を通したテストが含まれている。標準ライブラリの機能は E2E テストの中で間接的に検証される:

```rust
#[test]
fn test_e2e_option_type() {
    let output = compile_and_run(
        "(type (Option a) (Some a) None)
         (defn unwrap [opt default]
           (match opt
             [(Some x) x]
             [None default]))
         (defn main [] (print (unwrap (Some 42) 0)))"
    );
    assert_eq!(output.trim(), "42");
}
```

標準ライブラリの各モジュールは互いに独立しているが、一部は暗黙の依存関係を持つ。`Set` は `Map` のビルトインに依存し、`String` の検索関数は `Char` の文字判定に概念的に対応する。将来的には `(import Core)` のようなモジュールインポートで依存関係を明示的に管理する計画がある。

## まとめ

L# の標準ライブラリは、言語自身の表現力を証明する実験場であると同時に、セルフホスティングへの道筋を示す重要な構成要素だ。9 モジュール合計で約 400 行のコードだが、ADT、パターンマッチ、高階関数、再帰、Wasm リニアメモリ操作といった L# の主要機能をすべて活用している。

次章では、この標準ライブラリの上に構築されるセルフホスティングコンパイラを解説する。
