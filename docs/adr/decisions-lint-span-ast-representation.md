# ADR: lint 診断へ実 span を載せるための AST 表現

- Status: doc-RED (判断のみ確定。実装は本 slice に載せない)
- Date: 2026-08-19
- Scope: `LINT-SPAN-01` / `I-24`
- Related: [診断 dedup の rule identity](decisions-lint-diagnostic-dedup-identity.md)、
  `selfhost/src/Syntax/AST.ls` / `Syntax/Parser.ls` / `Tools/Doc/DocTools.ls`

## 問題

`L0001` (unused binding) と `L0002` (empty do block) は、実ソース
`(defn main [] (let [unused (do)] 0))` に対してどちらも range `0:0..0:0` で publish される。
`Tools/Doc/DocTools.ls:714` / `:729` が `make-review-diagnostic` の line / column 引数へ
定数 `1` `1` を渡しているためで、これは手抜きではなく**渡せる値が存在しない**ことの帰結である。

`TODO.md` の `LINT-SPAN-01` はこの前提を
「**selfhost の AST はそもそも位置情報を持っていない**」と書いていた。
**この記述は過度に一般化されており、本 ADR で訂正する。**

## 実測した現状 (2026-08-19)

### 1. 位置情報を持つノードは既にある

parser が構築するノードのうち、以下は byte offset の `start` / `end` を**既に持っている**。

| ノード | レイアウト | 構築点 |
|---|---|---|
| var | `[4, name-hash, start, end]` | `Parser.ls:389` `make-var-node-with-span` |
| qualified var | `[4, name-hash, start, end, prefix-hash, suffix-hash]` | `Parser.ls:393` |
| string | `[3, start, end, map-key-hash]` | `Parser.ls:400` |
| float | `[19, start, end]` | `Parser.ls:404` |
| apply | `[5, func, arg-count, arg1..argN, start, end]` | `Parser.ls:5124-5128` `parse-apply-v3` が `apply-start` / `previous-token-end-v3` を push |
| if | `[6, cond, then, else, start, end]` | `Parser.ls:3472-3473` `finish-parse-if-result-after-expect-v3` が `if-start` / `if-end` を push |

apply / if の 2 行は当初 reader 側 (`TypeInfer.ls:114-115` の条件付き読み) と
`MacroExpand.ls:196` のレイアウト注記だけを根拠にしていたので、**構築点まで遡って裏を取り直した**。
どちらも parser が実際に span を push しており、`make-lit-float` のような
「reader 側だけが対応している死んだ経路」ではない。一方 `AST.ls:132` `make-if` /
`AST.ls:133` `make-let` は span を取らないので、**同じ kind でも生成元によって長さが違う**。
これは決定 1 を補強する事実である (一律の末尾 span slot は、この非対称をさらに増やす)。
| module-decl / import-decl / type-alias | `name-start` / `name-end` を引数に取る | `AST.ls:262` / `:273` / `:238` |

**持っていないのは `let` (tag 7) と `do` (tag 9)** — つまり `L0001` / `L0002` が
対象とするまさにその 2 種である。`Parser.ls:3519` / `:3541` / `:3544` / `:3605` は
`(vector-push-quad-rooted-v3 (vector-new 8) 7 nh init body)` で長さ 4 のノードを作る
(容量 8 は確保済みで、余白はある)。

### 2. 「末尾 span pair」は既に確立した規約である

`preserve-apply-span` (`MacroExpand.ls:197-202`) が典型で、
**arity フィールドから span の index を算出し、長さで有無を判定する**。
新規設計を持ち込む必要はない。

### 3. ただし「無条件に末尾 slot を足す」は安全ではない

AST ノードを対象にした**長さ条件付き probe が 46 箇所**ある
(`(> (vector-length node|expr|decl) N)` 形)。うち 2 つは決定的:

- `TypeInfer.ls:60` `typeinfer-var-scheme` — **var ノードで長さ > 5 なら slot 4/5 を
  qualified name の prefix/suffix hash として読む**。span を無条件に足して長さ 6 に
  なったノードは、span を名前ハッシュとして解釈される。落ちずに誤った型解決をする
- `TypeInfer.ls:114-115` `infer-if` — **長さ > 5 を span の有無の判別子として使う**

したがって「全ノードに末尾 span slot」は却下する。**長さが判別子として使われている以上、
長さを一律に変えてはならない。**

### 4. span は byte offset で、診断は line / col — 変換が存在しない

`Syntax/Span.ls` の span は `[start, end]` の byte offset。一方 diagnostic の
shape は `[severity, rule-id, line, col, msg-hash, source]`
(`LspServerCore.ls:579`)。**selfhost に offset → line/col の変換関数は存在しない**
(`selfhost/src` 全体を走査して 0 件)。これが第 2 の欠落であり、
「AST に span を載せる」だけでは `LINT-SPAN-01` は閉じない。

### 5. lint 走査にソーステキストが届いていない

`review-collect-node [node results]` (`DocTools.ls:790`) は AST ノードと結果だけを取り、
ソースを受け取らない。offset → line/col 変換にはソース (ないし行頭 offset のテーブル) が
要るので、**走査の signature を変える必要がある**。ここが本項目の実質的な重さである。

## 決定

### 決定 1: 全ノード一律の末尾 span slot は採らない

理由は上記 3。既存の 46 probe のうち少なくとも 2 つは長さを意味の判別子に使っており、
一律の長さ変更は**落ちずに誤動作する**変更になる。

### 決定 2: kind ごとに span 付き constructor を足す既存規約へ寄せる

`let` / `do` に対して span 付きの構築形を追加し、**その kind に対する既存の長さ probe が
無いことを確認したうえで**長さを伸ばす。accessor は `import-decl-alias-hash`
(`AST.ls:341`) や `preserve-apply-span` と同じ「長さで有無を判定する」形にする。

### 決定 3: 側テーブル方式は採らない

ノードを key にした span の外部テーブルは、(a) parser から lint までテーブルを
引き回す必要があり、(b) `MacroExpand` がノードを再構築するため identity が保存されず、
(c) 既存規約と二重になる。**ノードに載せる方が既存コードと整合する。**

### 決定 4: ソース再走査による近似は採らない

lint 側で識別子名を検索して位置を当てる案は、同名の束縛が複数ある場合に誤った位置を返す。
`L0001` は「未使用の束縛」を報告する rule なので、同名の別束縛を指す誤りは
診断としての価値を損なう。

### 決定 5: offset → line/col 変換は診断投影の層に置く

AST 側は byte offset のまま持ち、line/col への変換は診断を組み立てる層
(`DocTools.ls` の `review-*`) で行う。理由は (a) AST に line/col を持たせると
ソース編集ごとに再計算が要る、(b) 既存の span 保持ノードが全て offset なので
表現を混ぜない、(c) 変換は 1 箇所に閉じる方が test しやすい。

**この決定に伴い `review-collect-node` 系の signature がソースを取る形へ変わる。**
これは本項目の受入条件に含める。

## この ADR に含めない範囲

- 重複判定の意味論。`I-24` で rule identity を含む形に裁定済みで、span が精密になっても
  判定規則は変わらない (同一 span に別 rule が正当に並ぶため)
- `sort-diagnostics` の順序規則 (AC-208 で固定済み)
- `let` / `do` 以外のノードへの span 追加。必要になった時点で同じ規約で足す

## 受入条件

1. `(defn main [] (let [unused (do)] 0))` に対し `L0001` と `L0002` が**別の range** を持つ
2. `I-24` の pin 2 本が引き続き pass する
3. 変更対象 kind (`let` / `do`) に対する既存の長さ probe が無いことを、
   実装前に grep で確認し ADR の Evidence へ記録する
4. offset → line/col 変換に単体 test を置く (行頭 / 行末 / 最終行 / 空行を含む)

## Evidence

### 受入条件 3 (実装前の長さ probe 確認) — 2026-08-19 に実施

cargo 非依存。`selfhost/src/**.ls` の 6,458 個の defn をトップレベル括弧走査で切り出し、
`(= tag 7)` / `(= tag 9)` / `(= ... (ast-let))` / `(= ... (ast-do))` の分岐直下から
呼ばれる defn 81 名を候補として集め、その本文でノード風仮引数に掛かる
`(vector-length <param>)` を全数走査した (走査窓は 600 文字の過大近似で、
安全側に振っている)。

**結論: `let` / `do` ノードの長さを条件に使う分岐は 1 箇所も無い。**

- 見つかった条件付き probe 4 箇所はいずれも**別 kind の専用ハンドラ**の中にある:
  `TypeInfer.ls:61` (var の qualified name)、`:83` (var の span 有無)、
  `:114-115` (`infer-if`)、`:177` (`infer-ann`)。`let` / `do` ノードはこれらに届かない
- `let` / `do` ノードに実際に掛かる `vector-length` は **2 箇所だけ**で、
  どちらも**条件ではなく print** である:

  | 位置 | 関数 | 形 |
  |---|---|---|
  | `Backend/Wasm/Compiler.ls:3108` | `compile-let-with-source-normal-setup-diagnostic` | `(print (vector-length node))` |
  | `Backend/Wasm/Compiler.ls:1434` | `compile-do-with-source-normal-setup-diagnostic` | `(print (vector-length node))` |

  この 2 つは `-normal-setup-diagnostic` という**閉じた複製系統**にあり
  (`Compiler.ls` 内の非 diagnostic 関数からの呼び出しは 0 件)、
  対応する test (`selfhost_native_stage_chain.rs:5052,5132,5257,5312`) は
  `(print 9000000237)` / `(print 9000000243)` という**ソース文字列の包含**しか見ていない。
  出力される数値を pin した test は無いので、span slot 追加で数値が変わっても test は壊れない。
  ただし診断ログを目視で読む側からは値が変わって見える

### `do` は可変長ノードである — 末尾 span が安全な理由

`do` の shape は `[9, expr-count, expr0, expr1, ...]` で (`Linter.ls:299` の構築が実例)、
**長さがソース依存で変わる**。したがって「末尾 pair を足す」が成立するのは
**全消費者が `vector-length` ではなく slot 1 の `expr-count` で走査を打ち切っている**
場合に限る。実測でこれは成り立っていた:
`FormatterExpr.ls:245,252` / `TypeInferBlock.ls:657` / `Compiler.ls:1352,1429` /
`Linter.ls:98` はいずれも `(vector-get node 1)` を上限に使い、`vector-length` は使わない。

`let` は `make-let` (`Syntax/AST.ls:133`) が固定長 4 (`[7, name-hash, init, body]`) を作るので
この論点は無い。

**この確認だけで受入条件 3 は満たした。** 残る受入条件 1 / 2 / 4 は実装を要する。

再現手順は `scripts/lint_span_probe_survey.py`。

### 診断経路の実測マップ — 2026-08-20 (cargo 非依存)

受入条件 1 / 2 / 4 の実装に入る前に、lint 診断が AST 走査から LSP wire へ出るまでを
ソース走査だけで辿った。**この過程で本 ADR と `TODO.md` / `I-24` の記述 2 件が誤りと判明した。**

#### 訂正 1: offset → line/col 変換は存在する

「selfhost に offset → line/col 変換が存在しない (`selfhost/src` 全体で 0 件)」は誤り。

| 関数 | 位置 | 形 |
|---|---|---|
| `lsp-position-from-offset` | `Tools/Lsp/LspServerNav.ls:285` | `src` を先頭から走査し char 10 で行を進める |
| `lsp-position-from-offset-loop` | `:278` | 実体。seed は `0 1 1` = **1-based** |
| `lsp-range-from-offsets` | `:288` | start/end の 2 offset を range へ |

呼び出し元は 7 箇所: `LspServerNav.ls:536` / `:579` / `:948` / `:1083`、
`App/Cli.ls:1404-1405` (parse 診断) / `:1455-1456` (type 診断)。
逆向きの `lsp-offset-from-line-col` (`:276`) も seed `1 1` で、**両方向が 1-based で一貫**している。
改行判定は char 10 のみ。CR は列文字として数えるが、これは両方向で同じなので往復は保たれる。

#### 訂正 2: 走査の signature 変更は要らない

「`review-collect-node [node results]` (`DocTools.ls:790`) はソースを受け取らないので走査の
signature を変える必要がある。ここが本項目の実質的な重さである」も誤り。
review 診断が byte offset を持てば、走査に `src` は要らない。**変換は投影境界で行えばよく、
その境界は既に `src` を持っている**。

| 投影関数 | 位置 | `src` |
|---|---|---|
| `lsp-parse-diagnostic-to-lsp [diag src]` | `Cli.ls:1400` | あり (`lsp-position-from-offset` を使用) |
| `lsp-type-diagnostic-to-lsp [code src start end]` | `Cli.ls:1450` | あり (同上) |
| `lsp-review-diagnostic-to-lsp [diag]` | `Cli.ls:1660` | **無し** — 3 本のうちここだけ |
| 呼び出し元 `lsp-source-lint-diagnostics [src]` | `Cli.ls:1681` | **あり** |

つまり `src` を通すのは `lsp-source-lint-diagnostics-loop` と
`lsp-review-diagnostic-to-lsp` の 2 本だけで、兄弟 2 本に signature を揃える向きの変更になる。
本項目は当初見積もりより軽い。

#### `0:0..0:0` の機構

`DocTools.ls:713` (`review-unused-let-diagnostic`) と `:732` (`review-empty-do-diagnostic`) が
`make-review-diagnostic` へ line/column を **`1 1` で直書き**している。
selfhost 内部は 1-based、`render-standard-diagnostic-json` (`LspServerCore.ls:613-616`) が
JSON 境界で 4 座標それぞれから 1 を引いて 0-based の LSP range にする。1 − 1 = 0。
`I-24` が pin する `0:0..0:0` はこの引き算の結果であって、投影の失敗ではない。

#### 第 2 の消費者があるため slot 4/5 は line/col のまま残す

`docjson-render-review-diagnostic` (`Tools/Doc/DocJson.ls:111`) が同じ slot 4/5 を
`line` / `column` として JSON に出し、`tests/snapshots/doctools/review-payload.json` が
2 診断とも `line: 1, column: 1` を pin している
(`doctools_parity.rs:870-880` が読む)。
さらにこの経路の入口 `generate-review-schema-json [ast source-id]` (`DocJson.ls:244`) は
`src` を持たないので、**docjson 側では offset → line/col 変換ができない**。

したがって採る形は「slot 4/5 は line/col のまま据え置き、offset は**末尾 slot へ足す**」。
`lsp-review-diagnostic-to-lsp` だけが末尾 offset を読んで変換し、docjson は現状のまま通る。
却下したのは「slot 4/5 の意味を offset へ変える」案 — snapshot が壊れるだけでなく、
`line` / `column` というフィールド名が嘘になり、変換手段の無い経路に offset が漏れる。

**末尾追加が安全であること**も確認した。review 診断ベクタ (7 slot) に対する `vector-length`
probe は 0 件で、掛かるのは診断の**配列**側だけ (`DocTools.ls:851-871`、`DocJson.ls:138`)。
一方 **LSP 投影後の 10 slot ベクタは長さを判別子に使っている** —
`LspServerNav.ls:1198` / `:1200` が `>= 8` で end 座標の有無を、
`LspServerCore.ls:636` が `>= 10` で standard / legacy を分ける。
これは AST の一律末尾 slot を却下したのと同じ危険であり、**投影後ベクタの長さは変えない**。

#### 混同しやすい別系統

`Tools/Text/Linter.ls` の `make-diagnostic` (`:176`) も `0 0` を直書きしている
(`:203` / `:236`) が、**これは LSP の生きた経路ではない**。didopen は
`generate-review` → `DocTools` を通る。ここを直しても `I-24` の観測値は動かない。

なお `Syntax/Parser.ls:5214` の `make-diagnostic [severity code span message-hash]` は
既に byte offset span を持っており、review 診断へ offset を載せる形はこの既存規約に沿う。

#### 受入条件 4 の再定義

変換は既にあるので、受入条件 4 は「新規実装 + test」ではなく
**既存 `lsp-position-from-offset` への単体 test 追加**になる。境界は
行頭 / 行末 / 最終行 / 空行に加え、`offset == (string-length src)`
(呼び出し元は end ≤ len を保つが、現状 pin が無い) を含める。

### 受入条件 1 / 2

(実装後に埋める)
