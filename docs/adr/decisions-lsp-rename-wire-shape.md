# ADR: `rename` の wire 形式を LSP `WorkspaceEdit.changes` に一本化する

- Status: Accepted (2026-08-23)
- Date: 2026-08-23
- Scope: `RENAME-WIRE-SHAPE-01` / `I-63` /
  `selfhost/src/Tools/Lsp/LspServerNav.ls` の `lsp-render-rename-frame-with-state` /
  `lsp-render-rename-frame` / `lsp-render-workspace-change-json` /
  `lsp-render-workspace-changes-json-loop`
  (対象は `textDocument/rename` の**応答形式**のみ。`handle-rename` の occurrence 収集ロジック、
  座標系 (`I-57` で決着済み)、`definition` / `references` (`I-61` で解決済み) は対象外)
- Related: [`ISSUES.md` I-63](../../ISSUES.md#i-63)、
  [ADR: definition / references の wire 形式](decisions-lsp-location-wire-shape.md)、
  [`ISSUES.md` I-61](../../ISSUES.md#i-61)

## Context

`I-61` が `definition` / `references` で潰したのと**同じ形の位置依存**が `rename` に残っている。

`lsp-render-rename-frame-with-state` (`LspServerNav.ls:222-234`) は

```lisp
(if (> (vector-length changes) 0)
  (let [first-uri (vector-get (vector-get changes 0) 0)
    first-uri-text (server-state-uri-text-for-uri state first-uri)]
    (if (> (string-length first-uri-text) 0)
      ...{"changes":{...}} を出す...
      (lsp-render-rename-frame request-id changes)))   ; 縮約形へ list ごと落ちる
  (lsp-render-rename-frame request-id changes))
```

と書かれており、**`changes` の先頭要素の uri text だけ**で応答全体の形式が決まる。

| 形式 | 例 | 出す条件 |
|---|---|---|
| `WorkspaceEdit` | `{"changes":{"file:///a.ls":[{"range":{...},"newText":"cube"}]}}` | 先頭要素の uri に uri text が登録済み |
| 縮約 array | `[[8091858770804166904,[{"range":{...},"newText":"cube"}]]]` | 登録されていない / `changes` が空 |

`I-61` と違う点が 1 つある。**要素ごとの uri 文字列化はすでに正しい** —
`lsp-render-rename-changes-json-loop-with-state` (`:210-220`) は要素ごとに
`lsp-render-wire-uri-text-for-state` (`:81-85`) を呼んでいる。これは `definition` /
`references` が使うのと同じ uri 文字列合成器で、state に uri text があればそれを、
無ければ `lsharp://document/<uri>` を返す (絶対 path の `file://` は
`lsp-virtual-uri-for-path` が state へ登録する段で入る)。
壊れているのは外側の guard だけで、**中身は既に一本化されている**。

判定に使う事実は 3 つ。

### 1. 縮約 array は LSP 3.17 に無い

`textDocument/rename` の結果は `WorkspaceEdit | null`。`[[int,[TextEdit..]],..]` を
解釈できる準拠 client は存在しない。`I-61` の縮約 `[uri,line,col]` と同じ、
uri text を state へ持つ前の時代の名残である。

### 2. 混在は `I-61` の後で起きやすくなった

`I-61` で `lsp-virtual-uri-for-path` が**絶対 path の uri text を state へ登録する**ように
なった。したがって「uri text を持つ document」と「持たない document」が同じ rename 結果へ
混ざる状況は、`I-61` 以前より到達しやすい。先頭がどちらに来るかで応答形式が変わる。

### 3. object 経路はすでに `changes` を出している

`:228` は `",\"result\":{\"changes\":{"` を literal で持つ。**動いている経路の形式は
すでに `changes`** であり、`documentChanges` を選ぶとこの経路まで書き換えることになる。

## Decision

### D1. `WorkspaceEdit.changes` に一本化する。`documentChanges` は採らない

`changes` は `{ [uri: DocumentUri]: TextEdit[] }`。現行 object 経路の形式そのままで、
本 ADR の変更は「縮約形への fallback を消す」ことに閉じる。

**`documentChanges` を却下した理由:**

1. **capability 交渉をしていない。** `documentChanges` は
   `workspace.workspaceEdit.documentChanges` を client が宣言した場合にのみ送ってよい形式で、
   宣言していない client へ送ると解釈されない。このサーバーは `initialize` で client capability を
   読んでおらず (`lsp-render-initialize-frame` は固定の server capability を返すだけ)、
   宣言の有無を知らない。**`changes` は宣言不要で常に合法**である。
   これは `I-61` で `LocationLink` を却下したのと同じ理由 (`definitionProvider.linkSupport` の宣言が要る)。
2. **`documentChanges` の利点を使わない。** versioned edit (`OptionalVersionedTextDocumentIdentifier`)
   と resource operation (create / rename / delete file) が `documentChanges` の存在理由だが、
   `handle-rename` はシンボル名の置換しか作らず、document version も追跡していない。
   得るものが無い形式へ寄せる理由が無い。
3. **動いている経路を壊す。** 上記の事実 3。

将来 version 追跡か resource operation が要るようになったら、そのときに
capability 交渉ごと入れる。**その時点で改めて ADR を書く**。

### D2. `changes` が空のときは `{"changes":{}}` を返す。`null` は採らない

現行は `(vector-length changes) == 0` を縮約 renderer へ落として `"result":[]` を出している。
一本化するなら空の場合も形式を揃える必要がある。

`WorkspaceEdit | null` なので `null` も合法だが、**`{"changes":{}}` を採る**。

- `I-61` は空 list を `"result":[]` にした (`Location[]` のまま空)。
  「空であることを、形式を変えずに表す」という同じ方針を踏襲する
- `null` にすると「結果が無い」と「rename できない」が同じ wire になる。
  後者はいずれ error response で表すべきもので、両者を潰したくない
- client 側は `changes` を空 object として素直に処理でき、分岐が減る

### D3. 縮約側の 3 関数を削除する

guard を外すと `lsp-render-rename-frame` の呼び出し元が消える。連鎖して
`lsp-render-workspace-changes-json-loop` / `lsp-render-workspace-change-json` も
呼び出し元を失う。**3 つとも削除する。**

`I-61` と同じ基準 — **形式が 2 つに戻る経路が実装に残らないこと**を、dead code を
残さないことで担保する。削除前に `grep -rn` で他の呼び出し元が無いことを確認する。

## 却下した代替案

| 案 | 内容 | 却下理由 |
|---|---|---|
| A | guard を「全要素が uri text を持つとき」に変える | 位置依存が「全体依存」に変わるだけで、形式が 2 つある事実は消えない。混在時に縮約形へ落ちるのも変わらない |
| B | 縮約形を残し、client 側で両方を読む | client は準拠実装 (VS Code 等) であり、こちらの都合で分岐を強いることはできない |
| C | `documentChanges` へ寄せる | D1 の 3 点 |
| D | 空を `null` にする | D2 |

## 影響範囲 (実装時に必ず更新する)

`grep` で列挙した実測 (2026-08-23)。**見積もりではない。**

- **snapshot 5 ファイル / 5 frame** — `rename.json` (id 65) /
  `rename-changed-document.json` (id 84) / `rename-latest-reopened.json` (id 85) /
  `rename-filesystem-import.json` (id 195) / `filesystem-document-sequence.json` (id 199)
- **`selfhost_cli_core.rs` のインライン 5 箇所** — `:16648` (id 65) / `:17014` (id 70) /
  `:17059` (id 70) / `:18017` (id 84) / `:18182` (id 85)
- **`selfhost_cli_core.rs:5293`** (`test_e2e_selfhost_cli_lsp_transport_rename_frame`, id 13) —
  縮約形かつ TextEdit まで縮約された期待値 (`[[99,[[1,7,1,13,"cube"],..]]]`) を持つ。
  現行 renderer は TextEdit を object で出すので、**この期待値は現時点で既に古い可能性がある**。
  実装前に単独実行して現状を確定させる (結果は Evidence 節)

`crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs:1840-1900` の
`lsp_real_shapes_rename` は `handle-rename` の**戻り値ベクタ**を検査しており wire を見ていない。
対象外。

## 受入条件

1. `rename` の応答が、`changes` の要素順・uri text の有無によらず常に `WorkspaceEdit` である
2. 空 `changes` が `{"changes":{}}` である
3. 縮約 renderer 3 関数が実装に残っていない
4. uri text を持つ document と持たない document を混ぜた rename 結果を、
   **先頭要素が uri text を持たない順序**で pin した function レベルの test がある
   (`I-61` の `test_e2e_selfhost_lsp_locations_frame_always_renders_location_objects` と同型)
5. `I-61` と同じ 31 test の lane が GREEN

## Evidence

### 実装

`selfhost/src/Tools/Lsp/LspServerNav.ls`:

- `lsp-render-rename-frame-with-state` から guard を丸ごと外し、
  **`changes` の長さも先頭要素の uri text も見ない**単一経路にした。
  空 `changes` は loop が空文字列を返すので、そのまま `{"changes":{}}` になる (D2)。
  要素ごとの uri 文字列化は既存の `lsp-render-rename-changes-json-loop-with-state` のままで、
  そこが呼ぶ `lsp-render-wire-uri-text-for-state` は `definition` / `references` と同じ合成器である
- 縮約側 3 関数 (`lsp-render-rename-frame` / `lsp-render-workspace-changes-json-loop` /
  `lsp-render-workspace-change-json`) を削除した (D3)。削除後に
  `grep -rn 'lsp-render-rename-frame\b\|lsp-render-workspace-change' selfhost/src crates` で
  live source に呼び出し元が無いことを確認した (hit するのは
  `crates/lsharp-wasm/ci-artifacts/native-linux-x86-hostgen-vm/.../actual-stage1/` の
  凍結コピーだけで、これは過去成果物の記録であり実行されない)

### `documentChanges` を却下した根拠の確認

`lsp-render-initialize-frame` は固定の server capability だけを返し、`initialize` params の
client capability を一切読まない。したがって `workspace.workspaceEdit.documentChanges` の
宣言有無をサーバーは知らない — ADR D1 の 1 点目は source で確認済み。

### 期待値をどう判断したか

書き換えた pin はすべて uri text を持たない document で、`lsharp://document/<uri>` に落ちる。
根拠は 2 つ。(1) fixture は uri を int で送るので state に uri text が無い。
(2) filesystem import の uri `8091858770804166904` は相対 path `Support/Mid.ls` の
`lsp-path-key` なので、`lsp-virtual-uri-for-path` の絶対 path 限定登録に掛からない
(`I-61` の ADR で決めた挙動)。`I-61` で書き換えた `references-filesystem-import.json` が
同じ uri を `lsharp://document/8091858770804166904` にしているのと一致する。

| 対象 | 件数 |
|---|---|
| snapshot ファイル | 5 (`rename` / `rename-changed-document` / `rename-latest-reopened` / `rename-filesystem-import` / `filesystem-document-sequence`) |
| snapshot frame | 5 (id 65 / 84 / 85 / 195 / 199) |
| `selfhost_cli_core.rs` インライン | 6 (`:5293` id 13 / `:16648` id 65 / `:17014` id 70 / `:17059` id 70 / `:18017` id 84 / `:18182` id 85) |

**影響範囲の見積もりは今回ズレなかった** — snapshot 5 / インライン 5 の grep 実測に、
`:5293` を「実装前に単独実行して確定させる」と書いた 1 件が加わって 6 になった。

### `:5293` は本 slice 以前から赤だった (`I-64` として登録)

`test_e2e_selfhost_cli_lsp_transport_rename_frame` は `#[ignore]` 付きで、
**本 ADR の変更を入れる前に単独実行したところ FAIL した**。

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3076 filtered out; finished in 259.16s
  left: ..."result":[[99,[{"range":{"start":{"line":0,"character":6},...},"newText":"cube"},...]]]
 right: ..."result":[[99,[[1,7,1,13,"cube"],[2,16,2,22,"cube"],[2,27,2,33,"cube"]]]]
```

期待値が TextEdit を縮約 array で pin しており、現行 renderer の object 出力と合っていない。
`git log -S` で辿ると期待値は 2026-03-27 `9deab1ce` 以来据え置きで、その後 renderer だけが変わった。
**本 ADR の変更とは無関係な、先行して存在した赤**である。本 slice では
`WorkspaceEdit` 形へ書き換えて GREEN にしたが、**`#[ignore]` の赤が台帳に載らない仕組みのほう**は
`I-64` / `IGNORED-STALE-PIN-01` として別に登録した。

### 測定

| 段階 | 結果 |
|---|---|
| RED (新 pin) | `FAILED. 0 passed; 1 failed` / 7.61s。`selfhost_lsp_docs_ops.rs:788` で panic。actual は `"result":[[8091858770804166904,[..]],[-2650468177460755683,[..]],..]` の縮約 array |
| GREEN (新 pin) | `ok. 1 passed; 0 failed` / 7.76s |
| 回帰 lane (`--ignored`, 32 test) | `test result: ok. 32 passed; 0 failed` / 1638.95s |
| pin lane (2 test) | `test result: ok. 2 passed; 0 failed` / 8.79s |
| `cargo fmt -p lsharp-wasm` | 差分なし |
| `git diff --check` | clean |
| `bash scripts/audit_docs.sh` | `エラー 0 件, 警告 0 件` |

回帰 lane は `I-61` の 31 test に `lsp_transport_rename_frame` を足した 32 test。
filter は `lsp_stdio_definition` / `lsp_stdio_references` / `lsp_stdio_rename` /
`lsp_stdio_filesystem_document_sequence` / `lsp_transport_goto_definition_frame` /
`lsp_transport_references_frame` / `lsp_transport_rename_frame`。
`definition` / `references` 側を残してあるのは、**`I-61` の結果が今回の削除で壊れていない**ことを
同じ lane で示すためである。

### 受入条件の判定

| 条件 | 判定 |
|---|---|
| 1. 要素順・uri text の有無によらず常に `WorkspaceEdit` | 満たした。guard が実装に無い |
| 2. 空 `changes` が `{"changes":{}}` | 満たした。新 pin の id 12 frame で pin |
| 3. 縮約 renderer 3 関数が残っていない | 満たした。削除済み、grep で live source に呼び出し元なし |
| 4. 混在 rename 結果を先頭 uri text 無しの順序で pin する test | 満たした。`test_e2e_selfhost_lsp_rename_frame_always_renders_workspace_edit` |
| 5. lane GREEN | 満たした。32 passed / 0 failed |

### 残った問題

- **`#[ignore]` の e2e に陳腐化した期待値が溜まっている** → `I-64` / `IGNORED-STALE-PIN-01`。
  本 slice で 1 本 (`:5293`) を直したが、全量は洗っていない
- `documentChanges` / version 追跡 / resource operation は入れていない。
  必要になったら client capability 交渉ごと別 ADR で決める (D1)
