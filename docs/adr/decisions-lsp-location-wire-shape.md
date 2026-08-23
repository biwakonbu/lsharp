# ADR: `definition` / `references` の wire 形式を LSP `Location` に一本化する

- Status: Accepted (2026-08-23)
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
snapshot 9 ファイル / 10 frame + `selfhost_cli_core.rs` のインライン 13 箇所**で、
これらは全て object 形式へ書き換わる。`I-57` の解決節に一覧がある。

**この file 数は doc-RED 時点では 8 と書いていた。** 実装時の実測で `references.json` (id 63) が
漏れていたと分かったので 9 へ直した。経緯は Evidence 節。

int の uri を送る test は、そのままでは 3 段目の `lsharp://document/<hash>` を
受け取ることになる。**期待値を機械的に置換するのではなく、
どの fallback 段に落ちるのが正しいかを test ごとに判断する**こと。

## Evidence

### 実装

`selfhost/src/Tools/Lsp/LspServerNav.ls`:

- `lsp-render-location-frame-with-state` / `lsp-render-locations-frame-with-state` から
  uri text の有無を見る guard を外し、常に `lsp-render-location-json-with-state` を通す。
  事実 3 の位置依存はこれで消える。
- 縮約 array のレンダラを削除した — Nav の `lsp-render-location-json` /
  `lsp-render-locations-json-loop` / `lsp-render-locations-frame` と、
  `LspServerCore.ls` の `lsp-render-location-frame`。呼び出し元は上記 guard だけだったので、
  残すと到達不能な形式定義が実装に残る。
  `render-rpc-int-vector-response-frame` (`JsonRpc.ls:200`) 自体は他の呼び出し元があるので残す。
- `lsp-virtual-uri-for-path` が `lsp-path-key` へ落ちるとき、新設の
  `lsp-register-file-uri-text` で `file://` + path を uri text map へ登録する (fallback 2 段目)。

### 2 段目を絶対 path 限定にした判断 (Decision の但し書きの実測根拠)

Decision は 2 段目を「path が絶対パスなら」と書いているが、**この条件は実装時に確かめた事実で決まった**。

既存 fixture は didOpen の path を**相対**で送る (`selfhost_cli_core.rs` の
`make_lsp_did_open_with_path(200, "src/Main.ls", ..)`)。import 先も `src/Support/Mid.ls` という
相対 path のまま `lsp-virtual-uri-for-path` へ届く。ここで `file://` を前置すると
`file://src/Support/Mid.ls` になり、これは host 部が `src` の**不正な URI** である。
かといって cwd で絶対化すると、応答が fixture の temp directory (実行のたびに変わる) に依存し
snapshot が非決定になる。

したがって **相対 path では合成せず 3 段目へ落とす**。判断は source のコメントにも残した。
この結果、既存 e2e はすべて 3 段目を受け取る。2 段目の被覆は新しい function レベル pin が持つ。

### 期待値をどう判断したか (受入条件「機械的な置換で済ませない」)

書き換えた 23 箇所は**すべて 3 段目 (`lsharp://document/<int>`) が正しい**。
根拠は frame ごとに次の 2 点を確認した結果であって、置換規則から出したものではない。

1. request が送る `"uri"` は int で uri text を伴わない → 1 段目に乗らない
2. filesystem import 先の uri (`8091858770804166904` = `Support/Mid.ls` の `lsp-path-key`) は
   相対 path の hash → 上記の判断により 2 段目にも乗らない

| 書き換え先 | 件数 | 落ちる段 |
|---|---|---|
| `tests/snapshots/lsp/stdio/*.json` | 9 file / 10 frame | 3 段目 |
| `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` インライン | 13 箇所 | 3 段目 |

**「影響範囲」節の見積もり (snapshot 8 file) は 1 file 少なかった。** 実測は 9 file で、
`references.json` (id 63) が漏れていた。frame 数 10 とインライン 13 箇所は見積もりどおり。

3 段すべてを 1 frame で踏む被覆は既存 fixture には作れないので、新しい pin
`test_e2e_selfhost_lsp_locations_frame_always_renders_location_objects`
(`selfhost_lsp_docs_ops.rs`) が開いた document / 絶対 path / 相対 path の 3 location を
**この順で** (先頭が uri text を持たない順で) 1 つの応答へ混ぜる。
空 list が `"result":[]` のままであることも同じ pin で押さえる。

### 測定

| 段階 | 実測 |
|---|---|
| RED (pin のみ) | `test result: FAILED. 0 passed; 1 failed` / 8.42s。`selfhost_lsp_docs_ops.rs:713` で panic。実際の出力は `"result":[[8091858770804166904,0,6],[5253187495188922191,1,2],[90369813871585817,2,4]]` で、**uri text を持つ 3 番目まで巻き込んで縮約 array へ落ちていた** — 事実 3 の位置依存が観測された |
| GREEN (pin のみ) | `test result: ok. 1 passed; 0 failed` / 7.66s |
| 回帰 lane (`--ignored`, 31 test) | `test result: ok. 31 passed; 0 failed` / 1655.18s |
| pin lane (2 test) | `test result: ok. 2 passed; 0 failed` / 8.44s |
| `cargo fmt -p lsharp-wasm` / `git diff --check` | 差分なし |
| `bash scripts/audit_docs.sh` | エラー 0 件, 警告 0 件 |

回帰 lane の filter に `lsp_stdio_rename` を入れたのは、**rename は本 ADR の対象外だが
snapshot file を definition / references と共有している**ため。形式変更で巻き込み事故が
起きていないことを同じ lane で示す必要がある。

### 受入条件の判定

| 受入条件 | 判定 |
|---|---|
| 縮約 array を廃止し常に `Location` object を返す | 満たした。レンダラ 4 本を削除したので形式が 2 つに戻る経路が実装に無い |
| 位置依存が消えたことを function レベルの pin で示す | 満たした。RED で位置依存が観測され、GREEN で消えた |
| 上記 lane が緑を維持する | 満たした。31 passed / 0 failed |
| 期待値の書き換えを機械的な置換で済ませない | 満たした。全 23 箇所を 2 つの事実から 3 段目と判定し、見積もりとのズレ (snapshot 8 → 9 file) も検出した |

### 残った問題

`rename` (`lsp-render-rename-frame-with-state`, `LspServerNav.ls:222`) には
**事実 3 とまったく同じ先頭要素依存が残っている**。本 ADR が scope を definition / references に
限ったためで、`WorkspaceEdit` は `Location` と別の型なので `changes` / `documentChanges` の
どちらへ寄せるかという別の判断が要る。`ISSUES.md` の `I-63` / `TODO.md` の
`RENAME-WIRE-SHAPE-01` として起票した。

なお 2 段目の追加によって uri text を持つ document が増えるため、**`I-63` の位置依存は
本 ADR の実装で踏みやすくなっている**。
