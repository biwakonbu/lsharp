---
name: doc-sync
description: L# の slice を開始・完了するときに、ISSUES.md / TODO.md / docs/adr / 運用記録のどれを更新すべきかを判定し、正本を同期する。実装の前 (doc-RED) と後 (doc-GREEN) の両方で使う。
---

# doc-sync — 正本ドキュメントの同期

このリポジトリの正本は 4 つあり、役割が重ならないよう分けられている。混ぜると二重管理になる。

| 正本 | 持つもの | 持たないもの |
|---|---|---|
| `ISSUES.md` | 何が問題か・根拠・状態 | チェックボックス、やること |
| `TODO.md` | **未完了**タスクだけ | `[x]`、完了項目、日付ログ、test 名、hash |
| `docs/adr/decisions-*.md` | 判断と却下理由 | 進捗 |
| `docs/development/operations/` | 実測値と運用手順 | 判断 |

`ISSUE.md` (単数形) は**作らない**。参照方向は ISSUES → TODO / ADR / 設計ドキュメントの一方向。

## 実装前 (doc-RED)

1. この変更は判断を含むか判定する。含まないなら (typo / rustfmt / test split / 挙動不変の
   リファクタ) 何も書かずに進んでよい。
2. **新しく認識した問題があるか。** あれば `ISSUES.md` に採番して追加する。
   - 採番: `grep -nE '^### (D|I|DOC)-[0-9]+' ISSUES.md | tail` で最大番号を確認し次番号。欠番は再利用しない
   - サマリー表の行と本文エントリの**両方**を足す。`<a id="i-10"></a>` のアンカーも忘れない
   - 状態は `open` / `in-design` / `deferred` / `documented-limitation` / `resolved` から選ぶ
3. **これからやることを `TODO.md` に置く。** `[ ]` で始める。
   受入条件と「この項目に含めない範囲」を書く。範囲を書かないと後で一括完了扱いされる。
4. **設計判断をしたなら ADR を先に書く。** 却下した選択肢とその理由を必ず含める。
   ファイル名は `docs/adr/decisions-<topic>.md`。Status / Date / Scope / Related のヘッダを付ける。
   Evidence 節は空でよい (実装後に埋める)。

## 実装後 (doc-GREEN)

1. ADR の Evidence 節に実測値・test 名・受入判定を埋める。
2. **受入条件を満たせなかったなら、その事実を書く。** 数字を静かに直さない。条件を後から緩めない。
   「文言どおりには満たしていないが意図は満たしている」と判断したなら、その判断と根拠を書く。
3. 計測をしたなら `docs/development/operations/` の該当ファイルへ、取得条件つきで記録する。
4. **完了した TODO 項目は削除する** — ADR / 運用記録へ移してから。`[x]` は付けない。
   partial parity / Rust-only / external boundary / 未検証 ABI は `[~]` のまま残す。
5. 日常の作業手順が変わった (新しいスクリプトを足した等) なら `AGENTS.md` に追記する。
6. 新しいドキュメントを作ったら、既存ドキュメントから**参照を張る**。孤立したファイルは無いのと同じ。

## 最終チェック

```bash
git status --porcelain -- ISSUES.md TODO.md AGENTS.md docs   # 何かしら差分があるか
grep -n '\[x\]' TODO.md                                      # 凡例の 1 行 (:12) だけであること
bash scripts/audit_docs.sh                                   # docs の整合
git diff --check                                             # 末尾空白 / conflict marker
```

`ISSUES.md` に新しい ID を足したなら、サマリー表とアンカーが本文と一致しているかを目視する。

## 落とし穴

- `TODO.md` は 2,800 行あり、v0.3 milestone 節が 2 箇所ある (`ISSUES.md` の `DOC-08`)。
  項目を足す前に既存項目を検索し、二重計上しない。
- 「今回の作業のスコープ外だが気づいたこと」は捨てずに `ISSUES.md` へ入れる。
  これを捨てるのが台帳未記載の既知問題を生む主因である。
- 一つの slice が閉じた範囲を越えて完了を宣言しない。focused test の GREEN、summary、
  stale artifact、Rust host fallback の成功だけでは完了としない。
