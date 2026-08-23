# ADR: `:invariant` に書かれた quote の扱い

- Status: Accepted
- Date: 2026-08-23
- Scope: `CONTRACT-INVARIANT-QUOTE-01` / `I-59` /
  `crates/lsharp-types/src/metadata_check/legacy.rs` の `check_legacy_invariant_types`
  (対象は `:invariant` に quote が現れたときの診断のみ。quote/unquote のマクロ展開そのもの、
  および `:example` 側の同種の穴は含まない)
- Related: [`ISSUES.md` I-59](../../ISSUES.md#i-59)、[`ISSUES.md` I-43](../../ISSUES.md#i-43)

## Context

`I-43` で `:invariant` の**識別子スコープ検査**から quote されたシンボルを外した。
`'sym` は変数参照ではないので「未定義の識別子 'sym'」は誤診断であり、この是正は正しい。

しかし後段の型推論 (`check_legacy_invariant_types`) が残っており、
`(defn caller [x] :invariant (= 'sym 'sym) x)` は今も 1 件の診断を出す。

```
[E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません
```

これは `infer/expr.rs:394` が `Expr::Quote` / `Unquote` / `UnquoteSplice` を
`TypeError::UndefinedVar` に**メッセージだけ差し替えて**載せているためで、
「未定義の変数」という見出しは事実と食い違っている。

判断に必要な事実は 3 つある。

1. **`:invariant` は検査されるだけでなく実行される。** `crates/lsharp-wasm/src/test_runner.rs:85`
   が `format!("{}", test.expr)` で invariant 式をそのまま生成ソースへ差し込み、
   サンプル引数を束縛して `lsharp test` で走らせる。
2. **quote は実行時表現を持たない。** `macro_expand.rs` はマクロ**呼び出し**を展開するだけで、
   マクロ本体の外にある裸の `'sym` は `Expr::Quote` のまま残る。
   「quote はマクロ展開後には残らない」という契約は
   `infer/expr.rs:394` と `ir/lower/expr/quote_expr.rs:9` の **2 箇所が独立に実装**している。
3. したがって `:invariant` の型推論を通したとしても、`lsharp test` は同じ理由で
   **lowering の段階で落ちる**。診断が消えるのではなく、出る場所が後ろへ移るだけである。

## Decision

**(a) 型推論を quote 対応させる / (b) `:invariant` を型推論の対象外にする、のどちらも採らない。**

採るのは **(c)**: `:invariant` の型推論は残したうえで、invariant 式が `Expr::Quote` /
`Unquote` / `UnquoteSplice` を含む場合は probe を組み立てずに **metadata 固有の Error を出す**。

> `:invariant` に quote/unquote は書けません (実行可能な contract であり、
> quote はマクロ展開後に残らないため)

実装は `check_legacy_invariant_types` の `has_unknown_reference` による skip と同じ位置に置く。
違いは、skip するだけでなく診断を 1 件返すことである。

こうすると、

- 見出しが「未定義の変数」ではなくなり、事実と一致する
- span が生成ソースではなく元の `:invariant` を指す
- `lsharp test` まで待たずに `lsharp check` の段階で分かる

## 却下した選択肢

### 案 (a): `:invariant` の型推論を quote 対応させる

**却下。型を付けても実行できない。** 上記事実 3 のとおり lowering が拒否するので、
型検査を通すと `lsharp test` の生成ソースで落ちる。ユーザーから見ると span が
自分の書いた `:invariant` を指さない位置へ移動し、**診断は改善ではなく劣化する**。

通すには quote の実行時表現 (Symbol 型、heap 表現、codegen、比較) が要る。
これは言語機能の追加であって contract 検査の範囲ではない。
さらに「quote はマクロ展開後には残らない」は現在 2 箇所が実装する言語の契約であり、
型推論だけ例外にすると契約が片側で破れる。

### 案 (b): `:invariant` を `:example` と同じく型推論の対象外にする

**却下。エラーが消えるのではなく後ろへ移るだけである。** これも事実 1 と 3 の帰結で、
`:invariant` は実行されるので検査を外しても `lsharp test` で落ちる。
案 (a) と同じく span が生成ソースへ移る。

「`:example` は 0 件で通るのだから `:invariant` も揃えるべき」という対称性の議論は
**前提が誤っている**。`:example` も `test_runner.rs:78` で実行されるので、
`:example [(caller 'sym)]` が 0 件で通るのは**正しさの証拠ではなく同種の穴**である。
揃えるなら `:invariant` を緩める方向ではなく `:example` を締める方向であり、
そちらは本 ADR の Scope 外なので別項目で追跡する。

### 案 (d): 現状維持 (`infer/expr.rs` のメッセージをそのまま出す)

**却下。見出しが事実と違う。** `TypeError::UndefinedVar` の
「未定義の変数 (undefined)」は quote には当てはまらない。
`I-43` が誤診断を 1 つ潰した直後に、別の誤った見出しを残す理由は無い。

## 受入条件

`contract_scope_quoted_symbol_in_invariant_is_accepted` の
「識別子スコープ由来のエラーが残らないこと」だけを見る緩い assert を、次の厳密な assert へ差し替える。

- 診断は**ちょうど 1 件**
- 「未定義の識別子」「未定義の変数」を**含まない**
- `:invariant` と quote に言及する metadata 固有のメッセージである

## Evidence

実装は 2 箇所。

- `crates/lsharp-types/src/metadata_check/references.rs` に `find_quote_span` を追加した。
  既存の `collect_unquoted_references` と同じく **wildcard arm を持たない網羅 match** で書き、
  `Expr` に variant が増えたときにコンパイルエラーで気付けるようにしている。
- `crates/lsharp-types/src/metadata_check/legacy.rs` の `check_legacy_invariant_types` で、
  `has_unknown_reference` の skip と同じ位置に quote 検出を挿入した。
  probe を組み立てず `continue` し、収集した診断を推論結果の**前**に連結する。

test は `contract_scope_quoted_symbol_in_invariant_is_accepted` を
`contract_scope_quoted_symbol_in_invariant_reports_metadata_error` へ改名し、
緩い assert を受入条件の 3 つの厳密な assert へ差し替えた
(`crates/lsharp-types/src/metadata_check/diagnostics_tests.rs`)。

| 段階 | 実測 |
|---|---|
| RED | `test result: FAILED. 0 passed; 1 failed` — `diagnostics_tests.rs:117` で `[":invariant の型推論に失敗しました: [E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません (32..36)"]` |
| GREEN | `test result: ok. 1 passed; 0 failed` (0.01s) |
| 回帰 | `cargo test -p lsharp-types` 全 binary 緑 (最大 binary は `255 passed; 0 failed`)。FAILED 0 |
| lint | `cargo clippy -p lsharp-types --all-targets` 警告 0 |

**受入条件は 3 つとも文言どおり満たした。** 緩和も読み替えもしていない。

### 残した穴 (Scope 外、別項目で追跡)

`:example [(caller 'sym)]` が 0 件で通ることは本 ADR で「正しさの証拠ではなく同種の穴」と
判定したが、本 slice では触っていない。`:example` は `test_runner.rs:78` で実行されるので
`:invariant` と同じ理由で lowering が拒否するはずである。追跡は `ISSUES.md` の `I-62`。
