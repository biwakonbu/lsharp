# 取り込み済み branch ref の削除台帳 (2026-08-22)

`main` = `13a3786b` 時点。全 local branch **129 本** を `git cherry main <branch>` で分類し、
`+` (main に patch-id が無い commit) が **0 本** の branch を「取り込み済み」とした。
そのうち worktree が checkout していない **71 本** が削除対象。
**A の 25 本 / B の 46 本、計 71 本すべて 2026-08-22 に削除済み。**
その後 worktree を全部破棄して除外条件が消えたため再分類し、**さらに 10 本を削除した (D / E)**。
**local branch は 129 本 → 49 本**になり、取り込み済みなのに残っている ref は 0 本である。
名前と sha をここに残す。

## この台帳が保証するもの / しないもの

- **保証する**: 各 sha が指していた commit 列の差分は、hash は違っても patch-id として `main`
  に存在する。内容は失われていない。
- **保証しない**: sha からの復元。reflog は約 30 日で expire し、その後 local-only の ref は
  到達不能になる。**復元可能性ではなく、内容の等価性が根拠である。**
- 「origin」列が `origin` の行は remote 側に ref が残るので、この台帳が無くても復元できる。

## 除外したもの

| 分類 | 件数 | 理由 |
|---|---|---|
| worktree が checkout 中 | 9 | patch-id は commit 済みの内容しか見ない。未 commit の作業内容には何も言えないので、salvage が済むまで触らない。**→ D で salvage と worktree 破棄を済ませ、E で 9 本とも削除した** |
| `+` を 1 本以上持つ | 49 | 未取り込みの commit がある。判断は `docs/adr/decisions-worktree-absorption-2026-08-20.md`。**うち 1 本 (`codex/lsharp-wasmgc-atomic-artifact`) は 2026-08-22 に内容で取り込み済みと判定した (C)。E で再検証のうえ削除した**ので、残る 48 本が今の未取り込み集合である |

除外した 9 本:

- `codex/gc-soak-telemetry-lane`
- `codex/lsharp-typeinfer-record-next`
- `codex/release-input-bundle`
- `codex/v0.2-ec-m1-01`
- `codex/v0.2-ec-m1-02-frontend-snapshot`
- `codex/v2-16c-native-selfhost-doc`
- `codex/v2-16c-native-selfhost-install`
- `codex/v2-16c-native-selfhost-repl`
- `codex/worktree-absorb-2026-08-20`

## A. `main` の祖先 (25 本) — 削除済み

`git merge-base --is-ancestor <branch> main` が真。`git branch -d` が受理した。

| branch | sha | origin |
|---|---|---|
| `codex/lsharp-agents-batch-final-main` | `2a74d27c4ef8` | local |
| `codex/lsharp-main-typeinfer-assertions-integration` | `d3b654c5e59d` | local |
| `codex/lsharp-parser-collection-docs-latest` | `d0f1df3e1abf` | local |
| `codex/lsharp-parser-expression-spans-docs-latest` | `4d4370216878` | local |
| `codex/lsharp-parser-metadata-docs-latest` | `abd073a60ccd` | local |
| `codex/lsharp-parser-metadata-fields-final-main` | `2611a3608555` | local |
| `codex/lsharp-parser-metadata-init-final-main` | `db147470f9a6` | local |
| `codex/lsharp-parser-metadata-outer-integrate-latest` | `8eacb41b3c96` | local |
| `codex/lsharp-parser-metadata-sequence-integrate-latest` | `3e1b5674dbee` | local |
| `codex/lsharp-parser-primitive-docs-main-next` | `5249c4f329d7` | local |
| `codex/lsharp-parser-signature-integrate-latest` | `8803d50124df` | local |
| `codex/lsharp-parser-structural-docs-latest` | `fe566eaba43b` | local |
| `codex/lsharp-record-schema-pattern-next` | `4fddd5155187` | local |
| `codex/lsharp-type-record-ops-docs-main-next` | `7e4140d9e1c2` | local |
| `codex/lsharp-typeinfer-apply-64-final-main` | `66b22d469efd` | local |
| `codex/lsharp-typeinfer-block-recorddecl-docs-main-next` | `fd8bf7fcc396` | local |
| `codex/lsharp-typeinfer-pattern-next` | `39a23f6d3117` | local |
| `codex/lsharp-typeinfer-record-docs-main-next` | `4fddd5155187` | local |
| `codex/m3-04-n1-current-source` | `2d250cdcd5d4` | local |
| `codex/m3-04-n1-source-file-parity` | `62e9e7a4676c` | origin |
| `cp05-reachable-heap-hints-9f1fb07` | `0fef152a4b20` | origin |
| `cp06-branch-protection-sync` | `bc082e5c1b77` | origin |
| `fix-clippy-collapsible-match` | `901c10d8141c` | origin |
| `release/v0.1.0-native-rc1` | `e72c9e82b292` | origin |
| `worktree-agbullet-fire-todo-tasks` | `47e5d3e63266` | local |

## B. patch-id 一致のみ (46 本) — **削除済み (2026-08-22)**

祖先ではないが `git cherry main <branch>` が `+` を出さない。`-d` は拒否するので `-D` を使った。
**この 46 本がこの台帳の本体である。**

**削除直前に 3 つを再検証した** (`main` = `8da59d82` 時点)。

1. 46 本すべて `git cherry main <branch>` の `+` が **0 本**
2. `git worktree list --porcelain` の checkout 集合と 46 本の積が **空**
3. 削除後、`git branch -D` が報告した sha を下表の sha 列と機械的に突き合わせ、
   **46/46 が一致**した (不一致 0)

3 番目は台帳そのものの検算である。台帳が保証するのは「その sha の内容が main にある」ことだが、
**そもそも台帳の sha が実物とずれていたらその保証は空文になる**ので、消える直前の実測と
突き合わせておく。local branch は 105 本 → 59 本になった。

名前の抽出は次で行える (削除後は 0 本 hit するのが正しい)。

```bash
sed -n '/^## B\./,/^## C\./p' docs/development/operations/absorbed-branch-refs-2026-08-22.md \
  | grep -oE '^\| `[^`]+`' | sed -E 's/^\| `//; s/`$//'
```

なお **judging 対象だった非 batch 23 本と batch family 26 本は削除しない**。
前者は `ISSUES.md` の `I-35` / `I-40` / `I-42` / `I-43` / `I-44` などが
参照実装を commit hash で名指ししているため、ref を消すと参照先が到達不能になる。
後者は `BOUNDED-SCAN-01` の hand-port 元である。

| branch | sha | origin |
|---|---|---|
| `codex/lsharp-agents-batch-contract` | `06755d650bc8` | local |
| `codex/lsharp-analysis-failure-index` | `8f3e81f2cf73` | local |
| `codex/lsharp-compile-io-main` | `6faeb1e02a62` | local |
| `codex/lsharp-diagnostics-main` | `d909b7069f1b` | local |
| `codex/lsharp-direct-qualified` | `6b08748c9405` | local |
| `codex/lsharp-ec-m2-01-project-duplicate` | `a05a8c95e801` | local |
| `codex/lsharp-lsp-incremental-main` | `fb65710a5640` | local |
| `codex/lsharp-main-open-merge` | `fed455c535ad` | local |
| `codex/lsharp-main-qualified-adt-options-latest` | `846cc5d61a59` | local |
| `codex/lsharp-main-qualified-adt-options-merge` | `21ea83733890` | local |
| `codex/lsharp-only-qualified` | `25a39e758d9c` | local |
| `codex/lsharp-open-only` | `7fc3e9627183` | local |
| `codex/lsharp-open-parser` | `693c4651d0f3` | local |
| `codex/lsharp-open-record-accessor` | `09dabe30062e` | local |
| `codex/lsharp-open-typeinfer` | `c26edba23055` | local |
| `codex/lsharp-open-typeinfer-clean` | `7f78f13b61c6` | local |
| `codex/lsharp-parser-collection-main-next` | `1b84052631ec` | local |
| `codex/lsharp-parser-collection-next` | `9444f4dabb4d` | local |
| `codex/lsharp-parser-expression-spans-main-next` | `afef9488f84e` | local |
| `codex/lsharp-parser-expression-spans-next` | `261364e4caf5` | local |
| `codex/lsharp-parser-metadata-fields-integrate-latest` | `580e8084b498` | local |
| `codex/lsharp-parser-metadata-fields-main-next` | `d1b9f4ea30db` | local |
| `codex/lsharp-parser-metadata-init-integrate` | `407f89de6263` | local |
| `codex/lsharp-parser-metadata-loops-next` | `40df772789b7` | local |
| `codex/lsharp-parser-metadata-main-next` | `da3103f50604` | local |
| `codex/lsharp-parser-metadata-outer-main-next` | `8609b08413b7` | local |
| `codex/lsharp-parser-metadata-sequence-main-next` | `8546aa3492bc` | local |
| `codex/lsharp-parser-primitive-main-next` | `c581689f0813` | local |
| `codex/lsharp-parser-primitive-next` | `55109a92468c` | local |
| `codex/lsharp-parser-signature-main-next` | `ff6037f2eacc` | local |
| `codex/lsharp-parser-structural-main-next` | `2ba1dede1dd3` | local |
| `codex/lsharp-parser-structural-next` | `7b1aa6236f09` | local |
| `codex/lsharp-qualified-adt` | `6fe49eaaaeed` | local |
| `codex/lsharp-qualified-adt-options` | `bb299eb8d661` | local |
| `codex/lsharp-qualified-record` | `ac4986d2027d` | local |
| `codex/lsharp-qualified-record-constructor-options` | `43bf3da66a33` | local |
| `codex/lsharp-qualified-record-type-annotation` | `9d36476261e1` | local |
| `codex/lsharp-type-record-ops-next` | `0c5caed35c3e` | local |
| `codex/lsharp-typeinfer-apply-64-integrate` | `3c0fda35a956` | local |
| `codex/lsharp-typeinfer-block-recorddecl-main-next` | `a3d715fb125f` | local |
| `codex/lsharp-typeinfer-block-recorddecl-next` | `9e9cd0cd27ec` | local |
| `codex/lsharp-typeinfer-record-inference-next` | `358c594ebc0e` | local |
| `codex/lsharp-typeinfer-record-main-next` | `8c69d50957ae` | local |
| `codex/v0.2-ec-m2-02-evidence-field-contract` | `d6fe3f43d423` | local |
| `copilot/cp05-selfhost-user-call-rooting` | `c0c13af347ae` | origin |
| `v2-08-native-proxy-loop` | `552b6e8453b2` | origin |

## C. `git cherry` は `+` だが内容は取り込み済み (2026-08-22 追記)

**この分類は A / B と根拠が違う。** `git cherry` の patch-id 一致ではなく、
**関数名・file 内容・docs の突き合わせ**で等価と判定したものを置く。
main が後から file を分割すると hunk の当たり先が変わって patch-id が崩れるため、
`+` の件数だけでは「取り込むものが残っている」と言えない。

| branch | sha | origin | `+` | 判定根拠 |
|---|---|---|---|---|
| `codex/lsharp-wasmgc-atomic-artifact` | `35f59fe5` | local | 37 | 追加関数 208 個ミス 0、WasmGC ADR 79 本と `wit/` 2 本が main にある、probe は main の方が大きい (10769 行 vs 10300 行)。ADR [`decisions-worktree-absorption-2026-08-20.md`](../../adr/decisions-worktree-absorption-2026-08-20.md) |

**2026-08-22 に削除した。** 保留条件だった「`WORKTREE-ABSORB-02` の判定が全部終わるまで」が
解けたため、C の根拠 1 を現行 main で引き直したうえで実行している。手順と実測は E を見ること。

## D. worktree の破棄 (2026-08-22)

branch ref とは別に、**worktree 36 本を全部破棄した**。`git worktree list` は
main の 1 本だけになった。**branch ref は 1 本も消していない** (`git worktree remove` は
checkout を外すだけで ref に触らない)。除外 9 本を含め、削除後も 59 本が健在である。

破棄前に 3 種類に分けて、それぞれ別の根拠で安全性を確かめた。

| 種別 | 本数 | 破棄の根拠 |
|---|---|---|
| 未 commit なし (branch checkout) | 14 | 失うものが無い |
| 未 commit なし (detached HEAD) | 14 | **HEAD の commit が生き残る ref から到達可能**であることを `git for-each-ref --contains` で 1 本ずつ確認した (12 個の sha すべて hit) |
| 未 commit あり | 8 | 内容を salvage したうえで破棄 |

**detached HEAD だけは扱いが違う。** branch checkout の worktree は ref が残るので
worktree を消しても commit は残るが、detached HEAD の worktree は **HEAD 自身が
到達性の唯一の担保になっている可能性がある**。ここを確認せずに消すと、計測用に
切っただけの baseline のつもりが実は唯一の参照だった、という失い方をする。
実測では 12 個すべてが `main` / `codex/*` / `backup/*` のいずれかから到達可能だった。

未 commit ありの 8 本は
`/Users/biwakonbu/github/tmp/worktree-salvage-2026-08-20/removed-2026-08-22/<name>/` へ
`STATUS.txt` / `tracked.patch` / `tracked.stat` / `untracked/` の形で退避した (計 400 KB)。
7 本は上の「突き合わせの結果 (2026-08-22): salvage すべき内容は 0 件」で判定済みのもので、
残る `lsharp-baseline-a3ae4551` の 14 件は insta の `.snap.new` である
(`worktree-salvage-2026-08-20/untracked/lsharp-baseline-a3ae4551/` の既存 salvage と
**14/14 byte 一致**を確認した)。`.snap.new` を accept しない方針は変えていない。

### ディスク

`/Users/biwakonbu/github/tmp/` を **13 GB → 1.1 MB** にした。残したのは
`worktree-salvage-2026-08-20/` だけで、そこへ `worktree-inventory-2026-08-20.md` と
absorb 実行時の raw log 7 本 (`absorb-logs-2026-08-20/`) を移して集約した。

消したものは build 出力と probe の残骸である (`absorb-target` 7.3 GB /
`stale-pin-02` 3.1 GB / `lsharp-stage1-current-330-target` 1.7 GB /
`lsharp-validation-input-refs` 664 MB / `*-target` 12 本 / `*-reuse-*` 4 本 ほか)。
`stale-pin-02` と `absorb-target` は `ISSUES.md:1509` と worktree 取り込み ADR が
`CARGO_TARGET_DIR` として名前を引いているが、**引いているのは取得条件としての path であって
中身ではない**ので、再生成可能な build 出力として消してよい。

worktree 破棄分と合わせて **16 GB** を回収した (`544Gi used / 331Gi avail` →
`528Gi used / 346Gi avail`)。

## E. worktree 解放後の再分類と最終削除 (2026-08-22)

「除外したもの」の **worktree が checkout 中 9 本**は、D で worktree を全部破棄したことで
除外理由が消えた。全 58 本へ `git cherry` を回し直したところ、**`+` が 0 本なのは
ちょうどこの 9 本**だった (残り 49 本はすべて `+` を 1 本以上持つ)。9 本とも削除した。

| branch | sha | origin | 分類 |
|---|---|---|---|
| `codex/gc-soak-telemetry-lane` | `5f162a7013d5` | local | patch-id 一致のみ |
| `codex/lsharp-typeinfer-record-next` | `39a23f6d3117` | local | main の祖先 |
| `codex/release-input-bundle` | `185f953fd1e0` | local | main の祖先 |
| `codex/v0.2-ec-m1-01` | `13eac0b0cca4` | local | main の祖先 |
| `codex/v0.2-ec-m1-02-frontend-snapshot` | `e9f9442817c3` | local | patch-id 一致のみ |
| `codex/v2-16c-native-selfhost-doc` | `0cd019e40759` | local | main の祖先 |
| `codex/v2-16c-native-selfhost-install` | `0cd019e40759` | local | main の祖先 |
| `codex/v2-16c-native-selfhost-repl` | `0cd019e40759` | local | main の祖先 |
| `codex/worktree-absorb-2026-08-20` | `13a3786b3cf7` | local | main の祖先 |

7 本は main の祖先、2 本 (`gc-soak-telemetry-lane` / `v0.2-ec-m1-02-frontend-snapshot`) は
patch-id 一致のみ。未 commit の内容は D で salvage 済みである。

### `codex/lsharp-wasmgc-atomic-artifact` も削除した

C に置いていた 1 本である。C の保留条件は「`WORKTREE-ABSORB-02` の判定が全部終わるまで」
だったので、判定完了をもって条件が解けた。ただし **この 1 本だけは patch-id が根拠にならない**
(`+` が 37 件ある) ので、削除直前に C の根拠のうち最も機械的な 1 番を現行 main で引き直した。

```bash
git cherry main codex/lsharp-wasmgc-atomic-artifact | grep '^+' | awk '{print $2}' \
  | xargs -n1 -I{} git show {} -- '*.rs' \
  | grep -E '^\+' | grep -oE '\bfn [A-Za-z0-9_]+' | sed 's/^fn //' | sort -u
```

37 commit が追加した **一意な関数名 138 個**を `grep -rE "\bfn <name>\b" crates/` で突き合わせ、
`main` = `8da59d82` で **ミス 0 件**。判定は今も成立している。sha は `35f59fe5` (local のみ)。

**この 1 本の削除は他の 48 本へ一般化しない。** 48 本は「patch-id も内容も未取り込み」であって、
`ISSUES.md` / ADR が参照実装を commit hash で名指ししている。ref を消すと参照先が
到達不能になるので残す。

### 最終状態

local branch は **129 本 → 49 本**。残る 49 本は例外なく `+` を 1 本以上持つ
(24 本が batch family = `BOUNDED-SCAN-01` の hand-port 元、25 本が判定済みの EC / 各 lane で
`I-35` 以降が hash で名指しする参照実装)。**取り込み済みなのに残っている ref は 0 本**である。

## 再検証の手順

削除前に、この台帳の各行に対して以下が成立することを確認した (削除ループ内で 1 本ずつ再実行)。

```bash
git cherry main <branch> | grep -c '^+'   # => 0
```

inventory はスナップショットなので、ループ内の再確認を正とする。
