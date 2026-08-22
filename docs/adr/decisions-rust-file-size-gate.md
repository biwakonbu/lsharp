# ADR: workspace 全域の Rust file-size gate

- Status: Accepted (verified slice)
- Date: 2026-08-23
- Scope: `crates/lsharp-wasm/tests/rust_file_size_contract.rs` (新規)、
  `tests/rust-file-size-allowlist.txt` (新規)、
  `tests/rust-test-file-size-allowlist.txt` (新規)、`AGENTS.md`
- Related: `I-01` (ファイルサイズ規約の大幅超過)、`RUST-FILE-SIZE-GATE-01`、
  `codex/legacy-maintenance-docs-active-only` (参照実装)

## Context

`CLAUDE.md` / `AGENTS.md` は 1 file 500〜800 行を規約としているが、main が持っていたのは
per-file の targeted guard 8 本 (`*_file_size.rs`) だけだった。これらは
**名指しした file の構成しか見ない**ので、新しく 800 行を超えた file が黙って増えるのを止められない。

実測 (2026-08-23) で超過は **39 件** — `crates/**/src/**` が 6、`crates/**/tests/**` が 33。
最大は `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の 62990 行 (78.7x)。

規約が文章としてしか存在せず、機械的な下限が無い状態が続いていた。

## Decision

`crates/**` を走査して 800 行超の `.rs` を集め、allowlist との**差集合が双方向で空**である
ことを要求する contract test を置く。

- **新規超過** (実測にあって allowlist に無い) → 落ちる
- **陳腐化 allowlist** (allowlist にあって実測に無い) → 落ちる

後者を入れるのが要点である。片方向だけだと、分割して 800 行以下になった file が
allowlist に残り続け、list が「かつて大きかった file の墓場」になる。
双方向にすることで **list は単調減少しかしない**。

### allowlist を 2 本立てにする

`tests/rust-file-size-allowlist.txt` (src 用) と `tests/rust-test-file-size-allowlist.txt`
(tests 用) を分ける。**1 本にまとめない。**

理由は超過の規模が桁で違うこと (src 6 / tests 33)。1 本だと「tests を 5 本分割したので
list が 5 行減った」という変化と「src の超過が 1 件増えた」という変化が同じ列に並び、
差分レビューで後者が埋もれる。gate の目的は数を減らすことではなく
**src の劣化を検知すること**なので、母集団を混ぜない。

### 参照実装から移植したのは workspace 走査の 2 test だけ

`codex/legacy-maintenance-docs-active-only` の `rust_file_size_contract.rs` は 800 行あり、
うち 16 test は同ブランチが行った分割の **fragment 構成 guard** である。
これらは main に存在しない file を参照するので移植できない。移植したのは
`rust_source_files_over_800_lines_match_allowlist` /
`rust_test_files_over_800_lines_match_allowlist` の 2 test と共有 helper (180 行) のみ。

**`codex/legacy-maintenance-stage-chain-integration` の allowlist 1 本の旧版は採らなかった。**
そちらは main の超過が src / tests で非対称であることを反映できない。

## Rejected

- **超過 file を先に分割してから gate を入れる** — 却下。62990 行の file の分割は
  それ自体が長期の作業 (`LEGACY-MAINT-01`) であり、その間ずっと新規超過が野放しになる。
  **gate と分割は別 slice にする。** allowlist は「今の負債の写像」であって免罪符ではない。
- **警告だけ出して落とさない** — 却下。`doc-guard.sh` を落とさない hook にしているのは
  「判断を含まない機械的な変更を止めない」ためだが、file-size は機械的に判定できる。
  警告は読まれない。
- **per-file guard 8 本を消して gate に一本化する** — 却下。既存の 8 本は
  fragment 構成 (`include!` マニフェストの順序と網羅) まで見ており、
  行数しか見ない gate では代替できない。役割が違うので併存させる。
- **allowlist への追加を test で禁止する** — 却下 (できない)。gate が見るのは
  「今の実測と list が一致するか」だけで、list に行を足す変更そのものは通る。
  **追加に ADR を要求するのは運用規約であって機械的な強制ではない。**
  この非対称性は明示しておく — 「gate があるから増えない」とは読めない。

## Evidence

`cargo test -p lsharp-wasm --test rust_file_size_contract` (2026-08-23):

| 段階 | 結果 |
|---|---|
| RED (allowlist 空) | `FAILED. 0 passed; 2 failed` — 新規超過に 39 件 (src 6 / tests 33) が列挙される |
| GREEN (実測どおり記入) | `ok. 2 passed; 0 failed` (`0.09s`) |
| 負の対照 1 (`validation.rs` を list から抜く) | `FAILED. 1 passed; 1 failed` — 新規超過に当該 file |
| 負の対照 2 (存在しない `nonexistent_probe.rs` を足す) | `FAILED. 1 passed; 1 failed` — 「解消済みまたは不正な allowlist」に当該 entry |

負の対照 2 本で**双方向の検知**が実測できている。gate が常に緑を返すだけの
置物になっていないことの証拠である。

### 満たせなかった受入条件

`TODO.md` の `RUST-FILE-SIZE-GATE-01` は受入条件を
「allowlist が単調減少すること (追加には ADR を要求する)」と書いていた。
**前半は機械的に強制できているが、後半はできていない。** 上記 Rejected の最終項のとおり、
list に行を足す変更自体は gate を通る。ADR 要求は運用規約として
allowlist file の先頭コメントと `AGENTS.md` に書くに留めた。
