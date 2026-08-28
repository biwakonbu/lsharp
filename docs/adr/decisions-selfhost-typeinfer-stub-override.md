# selfhost TypeInfer の stub / override 構造をどう扱うか

- **Status**: doc-RED (2026-08-28)
- **Date**: 2026-08-28 (doc-RED)
- **Scope**: `selfhost/src/Types/TypeInfer.ls` の stub 定義群と、それを上書きする
  `TypeInferApply` / `TypeInferBlock` / `TypeInferPattern` / `TypeInferRecord`。
  および `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の
  representative fixture の import 集合。
- **Related**: `I-101` (本 ADR の発端)、`I-102` (本 ADR が切り出す構造的ハザード)、
  `I-98` (parity test の発見経路)、`I-45` (0 引数 defn を `Unit -> body` にした契約)、
  `MODULE-DUP-FN-01` (同名 defn の重複定義)、`SELFHOST-PARSE-LENIENT-01` (落ちずに緩む形の先例)

## 問題

`I-101` は `(defn p [] (not true))` の推論結果が `Unit -> Bool` ではなく
`Unit -> t1001` になる、と記録している。`diagnostic-count` は 0 で、型エラーとしては
報告されない。**落ちない形の緩み**である。

## 判別測定 (doc-RED 時点で実施済み)

`I-101` が「この判別が最初にやるべきことである」と定めた Rust 側との対照を先に行った。
予測は `/Users/biwakonbu/github/tmp/i101/prediction.md` に**測定前**に書いた。

`crates/lsharp-types/src/infer_tests.rs` に
`infer_resolves_builtin_application_return_type` を新設し、4 形を測った。

| 入力 | Rust `infer` の戻り |
|---|---|
| `(defn p [] (not true))` | `() -> Bool` |
| `(defn q [] (and true false))` | `() -> Bool` |
| `(defn r [b] (not b))` | `(Bool) -> Bool` |
| `(defn s [] 42)` | `() -> Int` |

`cargo test -p lsharp-types --lib infer_resolves_builtin_application_return_type`
→ `ok. 1 passed; 0 failed; 255 filtered out`。

**Rust 側は型変数を残さない。したがって仕様ではなく selfhost 側の緩みである。**
prediction.md の分岐表で「仕様の疑い」側だった場合は selfhost を触らない取り決めだったが、
そちらには倒れなかった。

## 機構の特定

候補 3 つのうち、`I-101` が挙げた 1 と 2 は棄却され、実際の機構は 3 の変種だった。

- **候補 1 (`apply-subst` が var 連鎖を追い切らない) は棄却。**
  `Types/Type.ls:532-539` の Var 枝は `(apply-subst subst looked)` と再帰している。
- **候補 2 (subst が別物) も棄却。**
  `TypeInfer.ls:1473-1481` の `infer-program-analysis-type` は返す前に解析 subst を適用している。
- **実際の機構**: `Types/TypeInfer.ls:219-220` の

  ```lisp
  (defn infer-apply [node env subst counter]
    (make-result subst (fresh-type-var counter)))
  ```

  が **stub** であり、`fresh-type-var` を無診断で返す。本物は
  `Types/TypeInferApply.ls:731` にあり、`TypeInferApply` が `TypeInfer` を import する
  向きで上書きする。**`TypeInfer.ls` は `TypeInferApply` を import しない** (循環になるため)。

  `selfhost_native_stage_chain.rs:14643-14646` の fixture は

  ```lisp
  (import Syntax.Parser)
  (import Types.Type)
  (import Types.TypeInfer)
  ```

  だけを import しており、`Types.TypeInferApply` が無い。**module linker は import 到達性で
  link 対象を決める** — `try_compile_file_only` (`support.rs:202-230`) は import 宣言を持つ
  program を `lsharp_ir::compile_multi_file` へ渡し、その入口
  (`compile_entrypoints.rs:1-13`) が `ModuleGraph::build_from_entry_with_scc(entry_file)` で
  entry から辿れる module だけを `sorted_files` に集める。fixture dir にファイルが
  置いてあっても、import されない module は `sorted_files` に入らない。
  したがって**この fixture では stub がそのまま生き残る**。
  `[3, 1, 500, 2, 1001, 0, 0]` は stub の挙動そのものであり、`diagnostic-count = 0` も含めて説明できる。

- **parity test が見逃した理由**: `assert_representative_override_main_matches_selfhost`
  (`:48765-48814`) は同一の fixture source を native と selfhost の両方に流す。
  **両側とも stub を link するので、両方間違ったまま一致していた。** `I-98` で足した
  非空検査 (値そのものの pin) が無ければ表に出なかった。

- **concat bundle 経路は影響を受けない**: `selfhost_typeinfer_runtime_bundle()`
  (`support.rs:1558-1582`) は `TypeInfer.ls` の後に `TypeInferApply.ls` を並べるテキスト連結で、
  後勝ちの再定義として override が効く。
- **実 CLI も影響を受けない**: `selfhost/src/App/Cli.ls:18-22` が override 群 4 本を
  canonical な順で import している。**override 機構自体は在野で機能している。**
- **同型の under-import は e2e 全体で本 fixture 1 本のみ**
  (`grep -rn 'import Types.TypeInfer)' crates/lsharp-wasm/tests/e2e/*.rs` の結果、
  他の hit は `support.rs` の bundle 正規化とその検査だけ)。

## 決定

### 決定 1: fixture の import を canonical 集合へ揃える (`I-101` を閉じる)

`selfhost_native_stage_chain.rs:14643-14646` に override 群 4 本の import を足し、
slot 3/4 の期待値を `[1, 200]` (= `Con` / `Bool`) に締める。順序は `App/Cli.ls:18-22` を写す。

`(not true)` に厳密に要るのは `TypeInferApply` だけだが、**群ごと 4 本入れる**。
1 本だけ足すと、この fixture が将来 block / pattern / record を含む形へ育ったときに
別の stub を無言で踏む。既に一度踏んだ轍である。

`TypeInferApply` の transitive import (`Syntax.AST` / `Types.Type` / `Types.TypeScheme` /
`Types.TypeInferCore` / `Types.TypeInferFunctions`) はすべて
`SELFHOST_APP_MAIN_REPRESENTATIVE_MODULES` (`support.rs:41-74`) に既に入っている。
**ファイルは fixture dir に書かれており、欠けているのは import 行だけである。**

期待値を締めることが「実装に合わせて期待値を変える」に当たらない根拠:

1. test 内コメントが既に「`Bool` = `[1, 200]` になるはず」「実装を追認して `[2, 1001]` を
   焼き込むことはしない」と、**締める方向を先に予約している**
2. Rust 側の対照測定が `() -> Bool` を返した (実装の出力とは独立な canonical 側の根拠)
3. `TypeInferBuiltins.ls:129,182` が `not` を `Bool -> Bool` として型環境へ入れている

### 決定 2: stub 構造そのものは `I-101` に畳まず、`I-102` として別に切る

fixture の import を直しても、**`Types.TypeInfer` だけを import した任意のプログラムが
無診断で緩んだ推論を得る**構造は残る。これは fixture の bug ではなく selfhost の設計上の
ハザードなので、`I-101` (実測事例) とは別 ID で持つ。

### 却下した案

- **`TypeInfer.ls` が override 群を import する** — **循環になるので不可能。**
  override 群は 4 本とも `(import Types.TypeInfer)` している。この方向は取れない。
- **slot 3/4 を `[2, 1001]` に焼き込む** — 実装追認。test コメントが明示的に禁じている。
  型変数 id は selfhost の source が変われば動くので、pin として脆くもある。
- **`TypeInferApply` 1 本だけ import する** — 最小だが、同じ轍を残す。決定 1 の理由を参照。
- **stub を即座に trap / diagnostic 化する** — 本 slice ではやらない。
  `TypeInfer.ls` 単独 link を成立させている経路が他に無いことを確認しておらず、
  落とすと影響範囲が読めない。`I-102` の設計判断として deferred にする。
- **`not` に固有かを selfhost 側で別途測る** — **不要**。stub は argc も builtin の種類も
  問わず全 `infer-apply` に効くので、機構から導出できる。`I-101` の「`not` に固有かは
  分からない」は本 ADR で解消する (固有ではない)。

## Evidence

(実装後に埋める)
