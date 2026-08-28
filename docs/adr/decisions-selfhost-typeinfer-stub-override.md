# selfhost TypeInfer の stub / override 構造をどう扱うか

- **Status**: doc-GREEN (`I-101` 側は focused 2 run まで / lane 未了) + doc-GREEN (`I-102` 側の決定 3 / poison は未着手 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED) / 2026-08-28 (RED) / 2026-08-28 (GREEN) / 2026-08-28 (訂正 + 決定 3 の doc-RED)
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
  向きで上書きする。**`TypeInfer.ls` は `TypeInferApply` を import しない。**
  (当初ここに「循環になるため」と書いていた。**誤りである。** 下記「訂正」を参照。)

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

- **`TypeInfer.ls` が override 群を import する** — **却下。ただし当初書いた理由は誤りだった。**
  当初は「循環になるので不可能」と書いたが、**循環は両 backend とも許容されている**
  (下記「訂正」)。正しい却下理由は**両 backend で上書きの向きが逆になること**である。

  循環させた場合、`Types.TypeInfer` と override 4 本は 1 つの SCC になる。
  この SCC 内の順序は backend ごとに別の規則で決まる。

  | backend | 循環時の順序 | 結果 |
  |---|---|---|
  | Rust | `scc_groups()` を flatten (`resolve.rs:276`)。群内は名前で安定化 | `Types.TypeInfer` < `Types.TypeInferApply` なので**現状と同じ向き**が偶然保たれる |
  | selfhost | post-order append (`CompilerMode.ls:697-701`)。deps を先に、自分を後に | `TypeInfer` が**最後**に append され、後勝ちで**stub が本物を上書きする** |

  **向きが反転する。** 後勝ち規則そのものは両 backend で共通だが、
  循環時に誰が最後に来るかが食い違うため、同じソースが backend によって
  別の推論器を link することになる。これは `I-98` の parity 破壊そのものであり、
  「両方間違ったまま一致していた」より悪い。

  なお現状 (非循環) では、override 4 本が `(import Types.TypeInfer)` している以上、
  Rust の topological sort でも selfhost の post-order でも `TypeInfer` が必ず先に来る。
  **循環させないことが、両 backend の一致を保証している唯一の理由である。**
- **slot 3/4 を `[2, 1001]` に焼き込む** — 実装追認。test コメントが明示的に禁じている。
  型変数 id は selfhost の source が変われば動くので、pin として脆くもある。
- **`TypeInferApply` 1 本だけ import する** — 最小だが、同じ轍を残す。決定 1 の理由を参照。
- **stub を即座に trap / diagnostic 化する** — **`I-101` slice ではやらなかった。**
  `I-102` 側での扱いは決定 3 を参照 (「poison」案として保留した)。
- **`not` に固有かを selfhost 側で別途測る** — **不要**。stub は argc も builtin の種類も
  問わず全 `infer-apply` に効くので、機構から導出できる。`I-101` の「`not` に固有かは
  分からない」は本 ADR で解消する (固有ではない)。

## 訂正 (2026-08-28)

本 ADR は当初、`TypeInfer.ls` が override 群を import する案を
**「循環になるので不可能」**として却下していた。**これは検証せずに書いた推定であり、誤りである。**

循環は**両 backend とも許容されている**。

- **Rust**: compile 経路の入口 `compile_multi_file_with_mode`
  (`crates/lsharp-ir/src/compile_entrypoints.rs:1-13`) が使うのは
  `ModuleGraph::build_from_entry_with_scc` (`module_graph/resolve.rs:202-209`) で、
  この関数の doc comment 自身が「通常の `build_from_entry` は既存互換のため循環を
  エラーにする。この経路は SCC 単位の一括推論を行う compile pipeline 専用で、
  **循環したグループを許容する**」と書いている。実装は `allow_cycles=true` を渡し、
  `topological_sort()` が `CyclicDependency` を返したら
  `graph.scc_groups().into_iter().flatten()` へ落とす (`:272-278`)。
- **selfhost**: `load-module-if-new-with-cache`
  (`selfhost/src/App/CompilerMode.ls:675-716`) は再帰の**前**に
  `seen-ref` へ印を付ける (`:681`)。したがって循環に入っても静かに停止し、
  エラーにはならない。

**正しい却下理由は順序である。** 上記「却下した案」に書き直した。

### この誤りが生まれた経路

「override 4 本が `TypeInfer` を import している」という事実から
「逆向きは循環」→「循環は不可能」と 2 段推論した。
1 段目は正しいが、2 段目は module graph の実装を読まずに書いた一般論である。
**ADR に「不可能」と書くときは source で裏を取る**、を運用の教訓として残す。

## 決定 3: entry module の co-import 契約を正本へ書き、静的 invariant test で守る (`I-102` / doc-RED)

### 何が本当の欠陥だったか

この契約は**既に一度確立されていた**。`2b0c54b1` (2026-07-20) の commit message:

> The base TypeInfer module keeps its standalone stubs for module-graph compilation,
> while **public entry modules explicitly load the full implementation slices**.

同 commit は `App/Cli.ls` / `App/EmbeddedCli.ls` / `App/PipelineSmoke.ls` /
`App/SmokeCli.ls` の 4 entry へ override 4 本の import を足している。
危険の認識も同じである — 「native check could silently use compile-safe stubs
while the Rust hosted bundle exercised the full implementation」。

**にもかかわらず `I-101` が起きた。** 理由は単純で、**この契約はどの正本にも
書かれず、commit message にしか残らなかった**。書かれていない契約は、
新しい entry が足されたときに守られない。

静的走査 (2026-08-28) で、`selfhost/src` の `(defn main` を持つ entry 40 本のうち
**3 本**が同じ漏れを起こしていることが分かった。

| entry | 状況 |
|---|---|
| `Tools.Doc.DocTools` | `infer-defn` / `init-builtin-env` を呼ぶ。override 4 本を import していない |
| `Tools.Doc.HtmlDoc` | `DocTools` 経由で同じ |
| `Types.TypeInferSmoke` | `infer-expr` を 8 箇所で呼ぶ。override 4 本を import していない |

**3 本とも現在の link site は bundle だけで、override は効いている。**
今こわれてはいない。潜在ハザードである。

### 決定

1. **契約を明文化する。** 「`Types.TypeInfer` を import 閉包に含む entry module は、
   override 4 本 (`TypeInferApply` / `TypeInferBlock` / `TypeInferPattern` /
   `TypeInferRecord`) も import 閉包に含めなければならない」を本 ADR と `AGENTS.md` に置く。
2. **静的 invariant test で守る。** `selfhost/src/**.ls` を読むだけの純ファイル走査とし、
   **`#[ignore]` を付けない**。compile も wasm 実行も不要なので通常の `cargo test` で毎回効く。
   `#[ignore]` の lane へ入れると保護価値がほぼ消える。
   **RED は今日の 3 件で落ちること**であり、`I-102` の「先に赤い test を書く」を満たす。
3. **3 entry を是正する。** `App/Cli.ls:18-22` の順序を写して import 4 行を足す。
4. **死コードを削除する。** `recordlit-field-node-loop` は override が存在せず、
   `selfhost/src` 全体で呼び出しが 0 件である。

### 検査範囲を `selfhost/src` に限る理由

`.rs` の inline fixture が組む entry source は**対象外**とする。
bundle 正規化 (`support.rs:1483-1485` の `normalize_selfhost_bundle_source`) が
`(import Types.TypeInfer)` を意図的に剥がしており、テキスト走査では
「import していない entry」と区別が付かない。ここを機械判定へ入れると
正当な bundle を誤検出する。**fixture 側の残余リスクは poison (下記) が拾う**、という分担にする。

### 却下・保留した案

- **stub を全部消す** — 却下。`TypeInfer.ls:209-214` のコメントどおり
  stub は `TypeInfer.ls` 単体を型検査可能にするために置かれている。
  消すと `infer-expr` から呼ばれる名前が未定義になる。
- **例外リスト方式** (3 entry を許可リストへ入れて invariant を通す) — 却下。
  例外リストは腐る。`Types.TypeInferSmoke` は自身の `:8` に「連結実行でのみ
  最後の main として使う」と書いてあり bundle 専用だが、**import を 4 行足せば
  単独 link でも正しくなる**ので、例外にする理由が無い。
- **poison (stub の戻り型を `Con "__typeinfer_stub__"` 等の観測可能な marker にする)** —
  **保留。本 slice には入れない。** 効果は大きい (踏んだ瞬間に unify が落ちる) が、
  検証には `selfhost_cli_core` / `selfhost_native_stage_chain` を含む共有 lane 1 本が要り、
  それは `SWEEP-LANE-RERUN-01` が既に 8 項目を抱えて待たせている。
  **`I-102` の受入条件「現状の『静かに `fresh-type-var` を返す』だけは残さない」は、
  決定 3 の invariant test では `selfhost/src` の範囲しか満たさない。**
  この不足は `TODO.md` の `SELFHOST-INFER-STUB-DIAG-01` に明記して残す。

## Evidence

RED / GREEN とも同じ 1 本を focused で回した
(`AGENTS.md` の「representative native bundle 系は focused run で 1 本ずつ」)。
予測はどちらも実行前に `/Users/biwakonbu/github/tmp/i101/prediction.md` へ書いた。

| run | 起動 | 出力 | 判定 |
|---|---|---|---|
| RED (fixture 未修正 / 期待値のみ締める) | pid 88674、`os.setsid()` で切り離し。`red.log` | `[3, 1, 500, 2, 1001, 0, 0]` | `FAILED` / `MODEXIT=101` / `ELAPSED=170.29` |
| GREEN (fixture に override 群 4 本 import) | pid 98904、同上。`green.log` | `[3, 1, 500, 1, 200, 0, 0]` | `ok. 1 passed` / `MODEXIT=0` / `ELAPSED=140.77` |

どちらも `3082 filtered out` + 1 = **3083** で workspace e2e の宣言数と一致する。
`MODEXIT=101` は libtest の通常 test 失敗であり SIGKILL (`-9`) ではない。

### 予測との突き合わせ

- **RED は予測どおり。** 落ちた assert も予測した `values[3]` (`戻り型が Con にならない`) で、
  値 7 個すべてが `I-101` の記録と一字一句一致した。
  **型変数 id `1001` は build を跨いで安定**という `I-101` の記録も再確認された。
- **GREEN も予測どおり。** `[3, 1, 500, 1, 200, 0, 0]`。
  予測で挙げた外れ方 4 通り (compile error / duplicate definition / `[2,1001]` のまま /
  診断が非 0) はいずれも起きなかった。
- **これで `I-102` の前提が実証された。** override は import さえ張れば効く。
  linker は同名 `defn` の上書きを拒否しない。

### RED を別 run で取った理由

値そのものは `I-101` に既に記録があり、RED run は情報として一部冗長である。
それでも 1 run 割いたのは、**締めた assert が意図した slot に配線されているか**を
確かめるためである。期待値だけ書き換えて GREEN が出た場合、
assert が実は評価されていない (vacuous) 可能性を排除できない。`I-98` で
まさにその vacuous green を踏んでいる。

### 決定 3 の RED / GREEN (2026-08-28)

`crates/lsharp-wasm/tests/selfhost_module_import_contract.rs` を新設した。
**e2e とは別 binary** にしたので、workspace e2e の宣言数 3083 は変わらない
(`SWEEP-LANE-RERUN-01` の完走判定の分母を動かさないため)。

| 段階 | コマンド | 結果 |
|---|---|---|
| RED | `cargo test -p lsharp-wasm --test selfhost_module_import_contract` | `1 passed; 1 failed`。落ちた 3 件は **`Tools.Doc.DocTools` / `Tools.Doc.HtmlDoc` / `Types.TypeInferSmoke` ちょうど**で、事前の静的走査と完全一致 |
| GREEN | 同上 (import 追加後) | `ok. 2 passed; 0 failed` |

**RED が予測した 3 件と過不足なく一致したことが重要である。** 走査ロジックが
実装の import 解決と同じ閉包を計算していることの確認になっている。

実体の編集は 2 ファイルである。`Tools.Doc.HtmlDoc` は `DocTools` を import しており、
契約は**閉包**に対して定義されているので `DocTools` の是正で満たされる。

- `selfhost/src/Tools/Doc/DocTools.ls` -- override 4 本の import を追加
- `selfhost/src/Types/TypeInferSmoke.ls` -- 同上。冒頭コメントの
  「連結実行で**のみ**」も実態に合わせて訂正
- `selfhost/src/Types/TypeInfer.ls` -- stub 前書きを訂正 (後勝ち / module-graph でも効く)
  + 死コード `recordlit-field-node-loop` を削除

### 回帰確認

| 対象 | 結果 |
|---|---|
| `cargo test -p lsharp-wasm --test doctools_parity` | `38 passed; 1 failed`。落ちた 1 件は `test_e2e_doctools_extracts_typed_defn_metadata` で、`workspace-expected-failures.txt:148` に載る既知赤 (`DOCTOOLS-META-SLOT-01`)。**import 追加による新規赤は 0** |
| `e2e::selfhost_bootstrap_contracts::test_e2e_selfhost_type_hm_core_golden` (TypeInferSmoke の 15 module 連結 bundle) | `ok. 1 passed` / `3082 filtered out` + 1 = **3083** |

bundle 側が壊れないことは事前に構造からも読めていた。`App/Cli.ls` は
`2b0c54b1` 以降 `(import Types.TypeInferApply)` を持ったまま連結 bundle に入っており、
`normalize_selfhost_bundle_source` が剥がすのは `(import Types.TypeInfer)` 1 行だけである。
**同じ形の import 行が bundle 本文に残ったまま通る実績が既にあった。**

## 満たせなかったこと

- **lane 再計測は未了。** `ignored-lane-expected-failures.txt:412` の台帳行はまだ落としていない。
  **focused GREEN は lane 1 本の完走ではない。** `SWEEP-LANE-RERUN-01` が回るまで
  `I-101` は `open` のままにする。
- **poison (stub 自体を観測可能にする) は入れていない。** したがって
  `I-102` の受入条件「現状の『静かに `fresh-type-var` を返す』だけは残さない」は
  **`selfhost/src` の entry の範囲でしか満たしていない**。
  `.rs` の inline fixture が組む entry source は依然として無診断で stub を踏みうる。
  決定 3 の「却下・保留した案」に理由を書いた (検証に共有 lane 1 本が要る)。
- **`(mk-int)` を返す 5 件の stub が実際に踏まれた形跡は調べていない。**
  `infer-apply` は `I-101` で確定したが、他 22 件は未確認。
- **決定 3 の invariant test は `#[ignore]` の e2e を見ていない。**
  ソース走査は `selfhost/src` だけが対象である。

### 解消したもの (当初「満たせなかったこと」に挙げていた項目)

- ~~`I-102` の本題に手を付けていない~~ -> 決定 3 で entry 側の構造を塞いだ。
  残るのは poison のみ (上記)。
- ~~stub / override 対の module 横断の列挙をしていない~~ -> 完了。23 件。`I-102` に記録。
- ~~linker の override 解決規則を仕様として読んでいない~~ -> **後勝ちで確定**。
  `ftable-lookup-loop` (`Backend/Wasm/CompilerBase.ls:430-434`) が末尾側から走査し、
  `register-defns-step` (`Backend/Wasm/Compiler.ls:3971-3976`) は重複を skip しない。
