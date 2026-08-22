# 到達不能になった legacy free-list first-fit search を削除する

- **Status**: accepted
- **Date**: 2026-08-22
- **Scope**: `crates/lsharp-wasm/src/wasi/allocator.rs` の `emit_alloc_func` が出す
  `__alloc` function body。size-class allocator の設計そのものは範囲外。
- **Related**: `ISSUES.md` `I-35` / `I-04`、`TODO.md` `ALLOC-DEAD-BR-01` (本 ADR で削除)、
  [worktree 取り込み判定](decisions-worktree-absorption-2026-08-20.md)

## 何が問題だったか

size-class heads を導入したとき、旧来の free-list first-fit search を消さず、
block の直後に無条件 `Br(0)` を置いて**区間ごと到達不能にする**形で残した。
コメントは「旧 table は新しい class heads と併用しない。コードは ABI 差分を
小さく保つため残すが、常に bump/class path へ進む。」と書かれていた。

その到達不能区間の中に誤りがあった。内側の `if` の末尾が `Br(0)` で、
search loop の次 iteration へ進むには `Br(1)` でなければならない。現状の `Br(0)` は
内側の `if` を抜けるだけなので、**この path を再有効化すると無限 loop する**。

`codex/legacy-maintenance-stage-chain-integration` の `8be951e4`
(`fix(wasm): skip undersized free-list entries`) が同じ箇所を `Br(1)` へ直していたが、
main では dead path なので取り込みを却下していた (`I-35` の起票理由)。

## 決めたこと

**区間ごと削除する。** `Br(1)` へ直して残す案は採らない。

削除したのは `// free-list first-fit search` から、`Br(1)` の後に続く 3 つの `End` までの
91 行 (emitter コード)。同時に、この区間が唯一の参照だった
`AllocatorGlobals::free_list_base_global_idx` を struct ごと落とした。

## なぜ「直して残す」を却下したか

- **ABI 差分の議論が成り立たない。** ABI は function signature と heap layout であって、
  function body の instruction 列ではない。到達不能命令を消しても呼び出し側からは
  何も変わらない。「ABI 差分を小さく保つため」という当初の理由は根拠を欠いていた。
- **残すコストは毎回払う。** 到達不能命令は wasm validator を通るので、
  **出力するすべての module に 91 行分の死んだ byte が乗り続ける**。
- **残すと誤りに気付けない。** 実行されない以上、`Br(0)` / `Br(1)` の取り違えは
  どんな test でも検出できない。「直して残す」を選んでも、次の誤りは同じ理由で
  また検出できない。正しさを保証できない code を保持することになる。
- **再有効化する予定が無い。** `I-04` (free list 線形探索が O(n)) の解決策が
  size-class heads であり、それは既に入っている。legacy search は
  「まだ使うかもしれないもの」ではなく「置き換え済みのもの」である。

## 削除ではなく無効化を選ぶべき場合との違い

到達不能化して残すことに意味があるのは、**戻す条件が具体的に決まっている**ときである
(feature flag 待ち、段階移行の途中、外部 ABI の互換期間)。今回はどれにも当たらない。
戻す条件が無いまま到達不能化した code は、時間が経つほど「なぜ残っているか」を
説明できる人がいなくなる。実際この区間も、残した理由 (ABI) が誤っていたことに
1 年近く誰も気付いていなかった。

## Evidence

### guard test (RED → GREEN)

`crates/lsharp-wasm/src/wasi/allocator_tests.rs::allocator_body_has_no_unreachable_block_prologue`

`__alloc` の encode 済み body に `block (empty)` + `br 0` のバイト列
(`0x02 0x40 0x0C 0x00`) が現れないことを検査する。この形は「区間を丸ごと
到達不能にして残す」ときにだけ出るので、**同じ形の再発を型的に禁止できる**。

- RED (削除前): `__alloc の body に到達不能 block 区間が残っている` で fail
- GREEN (削除後): pass

### 削除の副作用が「参照が消えること」で裏取りできた

削除直後に `unused variable: free_list_base_global_idx` の warning が出た。
これは**この区間が legacy table の base pointer を使う唯一の場所だった**ことの
機械的な証拠である。続いて `field is never read` へ変わったので、
`AllocatorGlobals` の field と 4 箇所の構築サイトからも落とした。
同名 field を持つ `CollectorGlobals` は別 struct で、`gc_collect_core.rs` が今も使う。

### 回帰

| 検査 | 結果 |
|---|---|
| `cargo test -p lsharp-wasm --lib` | 138 passed / 0 failed / warning 0 |
| `cargo test -p lsharp-wasm --test e2e runtime_allocator` | 96 passed / 0 failed / 4 ignored |
| `cargo test -p lsharp-wasm --test e2e -- gc runtime_ strings_patterns` | 282 passed / 11 failed / 33 ignored / 914.67s。**11 件はすべて baseline 既知** |
| `cargo clippy -p lsharp-wasm --lib -- -D warnings` | exit 0 |
| `allocator.rs` の rustfmt | diff なし |

より広い e2e subset は **3 回失敗したのち 4 回目で採れた** (2026-08-22)。
1〜3 回目はログが途中で切れる / exit 0 で空になる、で終わった。この host には
`timeout` も `setsid` も無く、長時間 job を確実に切り離す手段が無いのが原因である。
4 回目は `nohup ... & disown` でファイルへ落として 914.67s で完走した。

failed 11 件の test 名を
`docs/development/validation/workspace-expected-failures.txt` と突き合わせ、
**baseline に無いものが 0 件**であることを確認した。新規 regression は無い。

削除の妥当性はこの subset に依存していない。削除した区間が唯一の参照元であることは
`unused variable` → `field is never read` の連鎖が機械的に示しており、
guard test と `--lib` 138 件 / `runtime_allocator` 96 件が実行経路を覆っている。


### 満たしていない受入条件

`TODO.md` の `ALLOC-DEAD-BR-01` は受入条件を
「`I-04` (free list 線形探索) の設計判断とセットで倒すこと」としていた。
**`I-04` は in-design のまま閉じていない。** ただし `I-04` が問うているのは
「線形探索をどう速くするか」であり、その答えである size-class heads は既に入っている。
本 ADR が決めたのは「置き換え済みの旧実装を保持しない」という一点で、
`I-04` の残りの設計 (class 境界の選び方、oversize の扱い) には触れていない。
**セットで倒す、の文言どおりではないが、`I-35` が `I-04` の判断を待つ理由は無くなった**
と判断した。`I-04` は自身の範囲で open のまま残す。
