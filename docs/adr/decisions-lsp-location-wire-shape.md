# ADR: `definition` / `references` の wire 形式を LSP `Location` に一本化する

- Status: Proposed (doc-RED)
- Date: 2026-08-23
- Scope: `LSP-LOCATION-SHAPE-01` / `I-61` /
  `selfhost/src/Tools/Lsp/LspServerNav.ls` の `lsp-render-location-frame-with-state` /
  `lsp-render-locations-frame-with-state` / `lsp-virtual-uri-for-path`
  (対象は `textDocument/definition` と `textDocument/references` の応答形式のみ。
  座標系は `I-57` で決着済みで本 ADR では触らない。`rename` / `hover` も対象外)
- Related: [`ISSUES.md` I-61](../../ISSUES.md#i-61)、[`ISSUES.md` I-57](../../ISSUES.md#i-57)

## Context

同じ method が **2 つの異なる wire 形式**を返す。

| 形式 | 例 | 出す条件 |
|---|---|---|
| LSP `Location` object | `{"uri":"file:///a.ls","range":{...}}` | 対象 uri に uri text が state へ登録済み |
| 縮約 array | `[8091858770804166904,0,49]` | 登録されていない |

分岐条件は `server-state-uri-text-for-uri` が非空かどうかだけで、method でも
capability でも client の宣言でもない。判定に使う事実は 4 つ。

### 1. 縮約 array は LSP 3.17 に無い

`textDocument/definition` の結果は `Location | Location[] | LocationLink[] | null`、
`textDocument/references` は `Location[] | null`。`[int, int, int]` を解釈できる
準拠 client は存在しない。**互換性のための形式ではなく、uri text を state へ
持つ前の時代の名残**である。

### 2. 実 client から到達する

`lsp-virtual-uri-for-path` (`LspServerNav.ls:509-511`) は、開かれていないファイルを
goto-definition の対象にしたとき `lsp-path-key path` (= path の hash) を uri として返す。
この uri には uri text が登録されないので縮約 array になる。
**client が開いていないファイルへ定義ジャンプする**という通常の操作で踏む。
snapshot `definition-filesystem-import.json` / `references-filesystem-import.json` /
`filesystem-document-sequence.json` がこの経路である。

**この時点で path は分かっている。** 分からないのは uri text だけで、
それは登録していないから分からないのであって、原理的に不明なわけではない。

### 3. 描画が要素の位置に依存する

`lsp-render-locations-frame-with-state` (`LspServerNav.ls:128-140`) の guard は
**先頭要素の uri text だけ**を見る。先頭が非空なら object 配列を選び、
その後は要素ごとに `lsp-render-wire-uri-text-for-state` が走る。
この関数は uri text が無ければ `lsharp://document/<int>` を**合成する**。

したがって uri text を持たない同一の location が、

- 先頭に来れば → 結果全体が縮約 array になる
- 2 番目以降に来れば → `{"uri":"lsharp://document/8091858770804166904", ...}` になる

**位置によって形式が変わる。** これは形式選択の問題ではなく単体の不具合である。
そして合成器がある事実は、「uri text が無くても object は出せる」ことを
実装が既に認めていることを意味する。

### 4. 縮約 array を残す動機が見つからない

`git log` に「client がこの形式を要求した」記録は無い。バイト数の節約という
実利はあるが、LSP は stdio 上の JSON-RPC でありここが律速になった計測も無い。

## Decision

**縮約 array を廃止し、常に LSP `Location` object を返す。**

uri 文字列は 3 段の fallback で決める。形式は 1 つ、変わるのは uri 文字列だけである。

1. client が送った uri text が state にあればそれ (現行の object 経路と同じ)
2. 無ければ、path が絶対パスなら `file://` + path を合成し、
   **解決時点で state の uri text map へ登録する** (`lsp-virtual-uri-for-path` の位置)
3. それも無ければ `lsharp://document/<hash>` (現行の合成器のまま)

3 段目は opaque な uri であり LSP 的には合法である (client が開けないだけで、
形式違反ではない)。2 段目を足すのは、事実 2 のとおり path が既知だからである。

これにより `lsp-render-location-frame-with-state` /
`lsp-render-locations-frame-with-state` の guard は不要になり、事実 3 の
位置依存も同時に消える。

## 却下した選択肢

### 案 B: 縮約 array を fallback として残したまま、guard を要素ごとにする

**却下。位置依存は直るが、準拠 client が読めない応答が残る。**
事実 1 のとおり縮約 array は spec に無いので、「読めない応答を一貫して返す」に
なるだけである。加えて snapshot と inline 期待値が非準拠形式を pin し続ける。

### 案 C: 常に縮約 array にする (object 経路を捨てる)

**却下。LSP 準拠を放棄することになる。** 形式は 1 つになるが、
`lsharp-lsp` を LSP client から使えなくなる。

### 案 D: capability / initializationOptions で形式を選ばせる

**却下。消費者がいない。** 縮約 array を要求する client は存在せず、
交渉の相手がいない。分岐を残したまま設定項目を 1 つ増やすだけになる。

### 案 E: uri text が無いときは `null` を返す (定義が見つからない扱い)

**却下。事実と違う。** 定義は見つかっている。見つかったものを
「見つからなかった」として返すのは、形式の都合で結果を捨てる操作である。

## 影響範囲 (実装時に必ず更新する)

`I-57` の実測で判明した範囲がそのまま当てはまる。**縮約 array を pin している箇所は
snapshot 8 ファイル / 10 frame + `selfhost_cli_core.rs` のインライン 13 箇所**で、
これらは全て object 形式へ書き換わる。`I-57` の解決節に一覧がある。

int の uri を送る test は、そのままでは 3 段目の `lsharp://document/<hash>` を
受け取ることになる。**期待値を機械的に置換するのではなく、
どの fallback 段に落ちるのが正しいかを test ごとに判断する**こと。

## Evidence

(実装時に埋める)
