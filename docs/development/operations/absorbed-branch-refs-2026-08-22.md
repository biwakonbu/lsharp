# 取り込み済み branch ref の削除台帳 (2026-08-22)

`main` = `13a3786b` 時点。全 local branch **129 本** を `git cherry main <branch>` で分類し、
`+` (main に patch-id が無い commit) が **0 本** の branch を「取り込み済み」とした。
そのうち worktree が checkout していない **71 本** が削除対象。**A の 25 本は削除済み、
B の 46 本は未削除** (`git branch -D` が auto mode classifier に拒否された)。
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
| worktree が checkout 中 | 9 | patch-id は commit 済みの内容しか見ない。未 commit の作業内容には何も言えないので、salvage が済むまで触らない |
| `+` を 1 本以上持つ | 49 | 未取り込みの commit がある。判断は `docs/adr/decisions-worktree-absorption-2026-08-20.md`。**うち 1 本 (`codex/lsharp-wasmgc-atomic-artifact`) は 2026-08-22 に内容で取り込み済みと判定した。下記 C** |

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

## B. patch-id 一致のみ (46 本) — **未削除**

祖先ではないが `git cherry main <branch>` が `+` を出さない。`-d` は拒否するので `-D` が要る。
**この 46 本がこの台帳の本体である。**

**2026-08-22 に再検証済み** (`main` = `8f9cd510` 時点)。46 本すべてについて
`git cherry main <branch>` の `+` が 0 本であること、および `git worktree list` の
どれもこの 46 本を checkout していないことを確認した。削除の前提は今も成り立っている。

削除は下表の branch 名を強制削除するだけだが、**強制削除は auto mode classifier に
拒否されるため、ユーザーが実行する必要がある**。名前の抽出は次で行える。

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

削除は **`WORKTREE-ABSORB-02` の判定が全部終わるまでしない** (TODO の「含めない範囲」)。
この行は「削除してよい根拠が揃った」という記録であって、削除の実行記録ではない。

## 再検証の手順

削除前に、この台帳の各行に対して以下が成立することを確認した (削除ループ内で 1 本ずつ再実行)。

```bash
git cherry main <branch> | grep -c '^+'   # => 0
```

inventory はスナップショットなので、ループ内の再確認を正とする。
