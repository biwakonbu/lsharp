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
| apply | `[5, func, arg-count, arg1..argN, start, end]` | `MacroExpand.ls:196` にレイアウト注記 |
| if | `[6, cond, then, else, start, end]` | `TypeInfer.ls:114-115` が読む |
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

(実装後に埋める)
