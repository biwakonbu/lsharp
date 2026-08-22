# block 形式の module body を compile 経路で reject する

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: `(module M (defn f ...) ...)` という **body を括弧内に持つ形**を
  compile 経路 (`lsharp-tooling` / `lsharp-ir` の parse → infer → lower) がどう扱うか。
  parser の受理そのもの、metadata / validation 検査の走査、
  入れ子 module (`(module A (module B ...))`) の可視性設計は範囲外。
- **Related**: `ISSUES.md` `I-39`、`TODO.md` `MODULE-BODY-FORM-01` (本 ADR で削除)、
  `I-37` / `I-38`、[worktree 取り込み判定](decisions-worktree-absorption-2026-08-20.md)

## 何が問題だったか

parser は `(module Name ...)` の 2 形を無条件に受理する
(`crates/lsharp-syntax/src/parser/decl.rs:410-428`)。

| 形 | 例 |
|---|---|
| flat (marker) | `(module Main)` + top-level に宣言を並べる |
| block (body) | `(module Main (defn main [] (print 42)))` |

しかし lowering は `Decl::ModuleDecl` を**一度も見ない**。
`crates/lsharp-ir/src/lower/program.rs` は `Decl::Defn` だけを拾い、
multi-file 経路の `crates/lsharp-ir/src/compile_entrypoints.rs:79-80` は
`Decl::ModuleDecl { .. } | Decl::ImportDecl { .. } => {}` で丸ごと捨てる。
body の中身は IR に到達しない。

一方で **型推論と検査系は body を走査する**。
`compile_pipeline.rs:229` / `:237` の `collect_private_surface_names` と
`:282` の `collect_expr_scope_owners` は非空 body へ qualified prefix 付きで再帰する。
metadata contract 検査 (`crates/lsharp-types/src/canonical_contract_check/`) も同様である。

つまり block 形式は **「検査はされるが lowering されない」中途半端な surface** になっている。

## 台帳の訂正 2 件

本 ADR は `ISSUES.md` `I-39` と `TODO.md` `MODULE-BODY-FORM-01` の記述を 2 点訂正する。
どちらも黙って直さず、訂正として残す。

### 訂正 1: 「この形を使っている `.ls` は repo 内に 0 件」は誤り

正しくは **2 件**ある。追跡下の `.ls` 133 件を走査した実測:

| ファイル | 用途 |
|---|---|
| `crates/lsharp-types/tests/fixtures/metadata/nested_contract_forms.ls` | 入れ子 module 下の `:example` / `:invariant` 検査を pin する fixture |
| `tests/fixtures/validation/ec-m2-project-duplicate-source.ls` | 同一 intent の重複宣言検知の fixture |

さらに Rust test 内の inline source としても **58 箇所**ある。大半は
`selfhost_lexer_parser` / `selfhost_typeinfer_assertion_scanners` などが
**selfhost の parser / assertion checker へ文字列として渡す**もので、
`(module Outer (module Inner (defn ...)))` を含む。

いずれも compile 経路に入らないので **「live な regression ではない」という結論は変わらない**。
しかし「0 件だから何を壊してもよい」という前提は成り立たない。
block 形式は **metadata / validation 検査系が意図的に受理している surface** である。

### 訂正 2: 「誤診断で落ちる」は過小評価

`I-39` は症状を「`未定義の変数` / `型の不一致` という誤診断で落ちる」と書いていた。
実際にはもう一段悪い**沈黙する誤答**がある。

| fixture | 内容 | compile | 実行 |
|---|---|---|---|
| `a.ls` | `(module Main (defn helper ...) (defn main [] (print (helper))))` | `[LS1001] 未定義の変数: helper` | -- |
| `b.ls` | `(module Main (defn main [] (print 42)))` | **成功** 6472 bytes | **何も出力せず exit 0** |
| `g.ls` | `(module Main)` + `(defn main [] (print 42))` | 成功 6498 bytes | `42` を出力、exit 0 |
| `d.ls` | `(module App (module Sub (defn succ ...)))` + top-level `main` | **成功** | -- |

`b` と `g` は意味的に同じ program である。block 形式は 2 function 少ない module を出し、
**何も起きないバイナリが exit 0 で完成する**。

sibling 参照がある場合 (`a`) は誤診断で止まるので気付ける。
sibling 参照が無い場合 (`b`) は**何も失敗を知らせない**。後者の方が重い。
これを理由に `I-39` の影響度を 中 → 高 へ上げる。

## 決定

**compile 経路の parse 直後・infer 直前で「未対応の構文」として reject する。**

診断は body を持つ `ModuleDecl` の span を指し、flat 形への書き換えを示す。
挿入点は 2 つ:

- `crates/lsharp-tooling/src/compile.rs` -- import を持たない単一 file 経路
- `crates/lsharp-ir/src/compile_entrypoints.rs` -- module graph 経路 (単一 file 短絡と SCC loop の両方)

### なぜ error にしても誰も壊れないか

body の宣言は lowering に到達しない。
したがって **block 形式の body に依存して正しく動いていた program は原理的に存在しない**。
reject が新たに error にするのは次の 2 種類だけである。

1. 既に沈黙して誤答していた program (`b` / `f`)
2. body が使われていないので偶然通っていた program (`d`)

どちらも「今まで通っていたものが通らなくなる」ではなく
「今まで黙っていた誤りに名前が付く」である。

## 却下した選択肢

### A. block 形式を実装する

`codex/legacy-maint-native-stage-chain-split` に参照実装がある
(`a5e5929c` / `fa7b4c51` の `incremental/root_module_body.rs`、`68849d55` の nested alias target scope)。
合計 **約 1,200 行**。

却下理由は規模ではなく**順序**である。実装は body 内宣言の可視性 --
sibling は見えるか、親 module から見えるか、`import` 越しに見えるか、
`private` との組み合わせはどうなるか -- を決めてからでないと書けない。
それは `TODO.md` が明示的に含めない範囲としたもの
(「入れ子 module の可視性設計。まず 1 段の body を決めてからにする」) である。

沈黙する誤答は今日消せる。可視性設計は今日決まらない。**先に沈黙を消す。**

**再開条件**: 入れ子 module の可視性が `I-37` / `I-38` と併せて決まったとき。
そのとき本 ADR の reject を撤回し、参照実装の 3 commit を当てなおす。

### B. parser で reject する

`I-39` と `TODO.md` の受入条件が文言として要求していたのはこれである。**却下した。**

parser を落とすと訂正 1 の 2 fixture が parse できなくなる。
`nested_contract_forms.ls` は入れ子 module 下の contract 検査を pin するために**存在している**
fixture であり、これを消すのは compile のバグを直すために
metadata 検査の能力を削ることになる。selfhost 側の assertion scanner 群 (58 箇所) も同様に
block 形式の source を parse できることに依存している。

**parse できることは正しい。lowering できないことが問題である。** 落とす層を間違えない。

### C. lowering で reject する

`Decl::ModuleDecl` が非空 body を持っていたら `LowerError` を返す形。
一見これが最も自然だが、**`a.ls` に対して発火しない**。
pipeline は parse → infer → lower であり、`a.ls` は infer 段階で
`未定義の変数: helper` を出して止まる。lowering まで到達しないので、
`I-39` が名指しした誤診断はそのまま残る。

infer より前に置く必要がある。よって compile driver の parse 直後とする。

## 満たしていない受入条件

`TODO.md` の受入条件は 「reject するなら **parse 時点で**『未対応の構文』と分かる診断」 と書いていた。
本 ADR は **parse 時点ではなく compile driver の parse 直後 (infer より前)** に置く。

文言どおりではない。理由は却下案 B のとおりで、parser 自体を落とすと
既存の metadata / validation 検査経路を壊すためである。
利用者から見た性質 --「compile しようとした瞬間に、自分のコードの誤りではなく
未対応の構文だと分かる」-- は満たしている。

## Evidence

### 実装

検査は `crates/lsharp-ir/src/module_body_form.rs` の
`reject_block_form_module_body` 1 本で、top-level の `Decl::ModuleDecl` を走査し
body が非空なら診断 `LS0105` を返す。入れ子は最外の body が非空になるので
top-level を 1 度見れば足りる。

呼び出し口は 3 つだが、**incremental 経路は 1 箇所に寄せた**。

| 経路 | 挿入点 |
|---|---|
| import 無しの単一 file | `crates/lsharp-tooling/src/compile.rs` の parse 直後 |
| module graph (非 incremental) | `crates/lsharp-ir/src/compile_entrypoints.rs` の 2 箇所 |
| module graph (incremental) | `crates/lsharp-ir/src/compile_support.rs` の `cached_program_or_parse` |

`cached_program_or_parse` は `compile_incremental.rs` の **6 箇所**から呼ばれる parse の
choke point である。呼び出し側 6 箇所へ検査を配ると、将来どれか 1 つで落としても
誰も気付けない。そこで返り値の型を
`Result<Arc<Program>, ParseAllError>` → `Result<Arc<Program>, String>` へ変え、
cache hit / fresh parse の**どちらでも返す前に**検査する形にした。
呼び出し側 6 箇所はいずれも直後に `format!` で `String` へ落としており、
`ParseAllError` を保持していなかったので、型変更で失われる情報は無い。

診断 code `LS0105` は `crates/lsharp-driver/src/error_codes.rs` の `ERROR_CODES` へ
`unsupported-module-body` として登録し、`docs/guides/error-reference.md` の
parser range (`LS0101` - `LS0104` → `LS0105`) と表へ追記した。
**doc への追記は任意ではない。** `mcp_server::tests::test_error_reference_doc_mentions_all_mcp_error_codes`
が `ERROR_CODES` の全 code について error-reference.md への出現を要求するので、
table へ足すだけでは driver の test が落ちる。この gate があるおかげで
「code はあるが利用者向け説明が無い」状態にはならない。

### test

| test | 場所 |
|---|---|
| `flat_marker_module_is_accepted` | `crates/lsharp-ir/src/module_body_form_tests.rs` |
| `program_without_module_decl_is_accepted` | 同上 |
| `block_form_module_body_is_rejected_with_code_and_span` | 同上 |
| `block_form_without_sibling_reference_is_rejected` | 同上 |
| `block_form_with_private_wrapper_is_rejected` | 同上 |
| `nested_block_form_is_rejected_at_outermost_module` | 同上 |
| `compile_rejects_block_form_module_body_in_single_file_path` | `crates/lsharp-tooling/src/compile_tests_diagnostics.rs` |
| `compile_rejects_block_form_module_body_in_module_graph_path` | 同上 |

compile 経路の 2 本は **RED を先に確認した**。どちらも
`expect_err` が `CompileArtifacts { output_path: ..., from_cache: false }` を掴んで panic した。
つまり実装前は 2 経路とも**成功していた**。

`compile_rejects_block_form_module_body_in_module_graph_path` は最初の実装 (`compile_entrypoints.rs`
への挿入のみ) では **GREEN にならなかった**。`compile_file` は
`compile_multi_file_with_cache` → `compile_multi_file_incremental` を通り、
`compile_entrypoints.rs` の loop を通らないためである。
上記の choke point 化はこの失敗から決めた。

### 拒否後の実測

`target/debug/lsharp compile` での probe。実装前の表 (「訂正 2」) と同じ fixture を使う。

| fixture | 実装前 | 実装後 |
|---|---|---|
| `(module Main (defn helper [] 1) (defn main [] (print (helper))))` | `[LS1001] 未定義の変数: helper` | `[LS0105] 未対応の構文 (0..74)` |
| `(module Main (defn main [] (print 42)))` | 成功 / **無出力 exit 0** | `[LS0105] 未対応の構文 (0..41)` |
| `(module App (module Sub (defn succ [x] (+ x 1))))` + top-level `main` | 成功 | `[LS0105] 未対応の構文 (0..61)` |
| `(module Main)` + top-level 宣言 (flat 形) | 成功 | **成功 / `42` を出力** |

誤診断の側 (`LS1001`) も沈黙の側も、同じ 1 つの診断へ寄った。

### 回帰

| 検査 | 結果 |
|---|---|
| `cargo test -p lsharp-ir` | 307 passed / 0 failed |
| `cargo test -p lsharp-types` | 501 passed / 0 failed |
| `cargo test -p lsharp-wasm --lib` | 138 passed / 0 failed |
| `cargo test -p lsharp-driver --bin lsharp` | 244 passed / 0 failed |
| `cargo test -p lsharp-tooling` | 145 passed / 1 failed (下記の既知 baseline) |
| `cargo clippy -p lsharp-ir -p lsharp-tooling --lib -- -D warnings` | exit 0 |
| `cargo fmt --check -p lsharp-ir` | exit 0 |

**block 形式を使う既存 fixture は 2 件とも通ったままである。**
`nested_contract_forms.ls` を使う `nested_module_contract_inventory_uses_qualified_owner` と、
`ec-m2-project-duplicate-source.ls` を使う `validation_source` の 62 件がいずれも pass した。
どちらも compile 経路に入らないので、reject の影響を受けない。
これは推論ではなく実行で確かめた。

`lsharp-tooling` の `api_doc::tests::test_build_api_doc_for_file_preserves_parse_error_code` は
**本変更の前から落ちている** (`workspace-expected-failures.txt:139` に登録済み。
`[LS0103]` を期待して `[LS0102]` が返る)。本変更は `api_doc.rs` に触れていない。

`cargo clippy -p lsharp-ir` は本変更とは無関係に
`module_graph/resolve.rs:261` の `redundant_closure` で落ちていた。
自分の変更を lint で確認できないので、その 1 行だけ併せて直した
(`|| Span::dummy()` → `Span::dummy`)。判断を含まない機械的な修正である。
