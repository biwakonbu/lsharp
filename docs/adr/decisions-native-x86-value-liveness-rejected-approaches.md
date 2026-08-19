# ADR: Native x86 の値 liveness 調査 — 到達した原因と却下した案

- Status: Accepted (salvage record)
- Date: 2026-08-19
- Scope: `selfhost/src/Backend/Native/NativeCodegen.ls`,
  `selfhost/src/Backend/Wasm/Compiler.ls`, `selfhost/src/Syntax/Parser.ls`,
  `selfhost/src/App/CompilerMode.ls`
- Related: `DOC-09` (この ADR が救出先)、`I-27` (native 実行時の値破壊)、
  `V2-13a-5` / `V2-13a-5b` / `V2-13a-5h` (削除された旧 TODO 項目)、
  `NATIVE-INLINE-01`

## Context

2026-05〜07 の `V2-13a-5*` は、Linux x86_64 の stage2/stage3 replay が値破壊で進まない問題を
数十回の VM 診断で追ったものである。その過程で **原因の絞り込み 61 行**と
**却下した案 35 行**が `TODO.md` に書かれていたが、`0bd8bd47`
「docs: keep active-only TODO backlog」(2026-07-25) が同ファイルから 1,364 行を削除した際、
`docs/adr/` へは 1 ファイルも移されなかった (`DOC-09`)。

結果として `I-27` を起票した時点では、5 件の否定 assertion のうち 1 件しか原因が判っていない
ように見えていた。実際には**同じ欠陥クラスの調査記録が既にあった**。

本 ADR はその記録の移送先である。**新しい判断はしていない。** 当時の判断を主題ごとに
まとめ直したものであり、事実関係はすべて `git show 0bd8bd47 -- TODO.md` の削除行に遡れる。

## Decision

削除された 1,364 行のうち、**93 行** (原因 61 / 却下 35 / 重複 3) を救出対象とし、
以下の 3 主題へ集約する。残り 1,271 行は移送しない (理由は末尾)。

### 主題 1: 返却値の root gap

最頻出の原因は単一のパターンである。

> **helper が返却値を root せずに roots を unwind するため、返却値が GC で stale 化するか
> 0 に化ける。**

`root_pop` が値を返す前提に依存していた個所では、unwind 後の return がそのまま 0 になっていた。
修正の形は毎回同じで、返却値を下位の root slot へ `root_set` してから unwind し、
unwind 後に明示的に返す。

当時 RED→GREEN で追加された回帰テストが、そのままこのパターンの一覧になっている:

| 対象 | test |
|---|---|
| Wasm compiler の 64-step continuation | `test_wasm_compiler_source_defn_step64_continuations_root_recursive_result_before_unwinding` |
| Wasm compiler の single-step continuation | `test_wasm_compiler_source_defn_single_continuations_root_returned_state` |
| Wasm compiler の step state builder | `test_selfhost_step_state_builders_root_final_state_before_unwinding_parts` |
| Parser の `parse-let-v3` | `test_selfhost_parser_parse_let_roots_returned_result_before_unwinding_bindings` |
| let chain の init compile | `test_selfhost_compile_let_step_reloads_body_after_init_compile` |
| CompilerMode の functions | `test_selfhost_compile_file_functions_roots_record_prelude_result_before_unwinding_state` (当時の名は `test_selfhost_compile_file_functions_roots_result_before_unwinding_state`。`d320c9b4` で改名) |
| CompilerMode の payload | `test_selfhost_compile_file_payload_roots_result_before_unwinding_state` |
| CompilerMode の transport entry | `test_selfhost_compile_file_mode_roots_payload_and_wasm_before_printing` |
| x86 one-arg call core | `test_native_codegen_x86_one_arg_call_core_roots_parts_before_concat` |
| x86 call bundle の `function-metas` | `test_native_codegen_x86_call_bundle_roots_function_metas_before_ref_allocations` |

`I-27` の候補 (b) 「callee 側の ref 確保が caller の未 root ref を無効化する」は、
この主題の一部である。**ただし本主題の修正はすべて L# 側の書き方を変えるものであり、
lowering は直っていない。**

### 主題 2: x86 の call-rel byte 列が helper 呼び出し境界で失われる

`I-27` の直系にあたる。到達した最も狭い所在は次の 1 文である。

> `call-rel` は helper 内の `push rax` 後ではなく、`append-zero-arg-call-bundle-x86` への
> **関数呼び出し境界で既に崩れている**。

VM metadata では `idx=138 opcode=60(map-new) depth=1 bytes=[80,232,79,4,0,0,89,72]` /
target `2219` が、以下の修正案をすべて跨いで**不変**だった。

却下した 27 案は 5 つの形に分類できる。**どの形も static contract test は通り、
落ちるのは必ず selfhost 自身の実行か artifact gate である。**

| 案の形 | 例 | 落ち方 |
|---|---|---|
| 新規 helper を足す | `append-map-new-call-bundle-x86`、broad `build-callables-with-imports`、fallback 側の小 helper | `lsharp parse/check NativeCodegen.ls` が Wasm call stack exhausted / embedded component の `realloc_internal` failure / 深い再帰 backtrace |
| rooted-ref 化する | `call-rel` rooted-ref 案、`call-rel-bytes` の helper-local root 化、`call-rel-ref` 化 | host 側 actual stage1 artifact gate で layout harness が Wasm OOB / `unreachable` |
| control-loop 本体へ分岐を足す | `opcode53 depth<=2` direct append、depth=1 専用分岐、runtime bundle/fallback を本体へ戻す | selfhost compiler の Wasm 実行が `unreachable`、または parse/check 失敗 |
| rel32 の算出位置を動かす | append 後に rel32 再計算、`push rel` 後に算出、target-param-count=8 を zero-arg branch より前へ移動 | static gate は通るが VM metadata の bytes / target が不変 |
| root 操作を増減する | 八引数専用の early root-pop、root_push/drop の fixed byte-vector 化、plain two-to-one helper root | call stack exhausted、または metadata 不変 |

**唯一通った形**は、新しい分岐を作らず既存分岐へ合流させるものだった。
`opcode63` は `opcode50 (string-char-at)` の consume-two direct append 分岐へ合流させ、
runtime dispatch 側も同じ op50/op63 分岐へ統合して nested branch を増やさない形にした。
同様に `opcode51 (string-length)` は narrow direct append だけを足した。

ここから引ける一般則:

> **selfhost の control-loop 本体や helper の arity を増やす修正は、selfhost 自身の
> parse/check を壊す。** NativeCodegen.ls への修正は既存分岐への合流か、
> 単一 opcode の narrow direct append の形でしか通らない。

`I-25` が数えた「呼び出し元 0 の defn 64 件」のうち、`emit-call-bundle-x86-one-to-nine` や
`emit-map-new-bundle-x86` といった arity dispatcher が production から外れているのは、
この制約の結果である。

### 主題 3: IR サイズ別 fallback の廃止

`generate-native-function-x86-64-bundle-with-layout` は当初、IR 長で
chunked control loop と row-state loop を使い分けていた。blocker はこの分岐差分にあり、
`n > 1024` (巨大)、`n < 17` (極小)、`65..1024` (中間) を順に切り分けた結果、
**全 IR size を row-state loop へ統一**して size fallback を廃止した。

- 回帰テスト: `test_native_codegen_x86_function_bundle_avoids_initial_control_state_fallback`
- 却下: offsets/depths を単一 snapshot 化する案 — heavy host layout が wasm OOB になった

## Evidence

- 削除元: `git show --format='' -U0 0bd8bd47 -- TODO.md` の削除行 (1,364 行)。
  `git show --numstat --format='' 0bd8bd47 -- TODO.md` は `128 / 1364`。
- 救出対象の抽出条件: 削除行のうち `原因|判明|症状|下流|真因|突き止|に絞|絞られ|崩れている|化ける`
  にマッチする 61 行と、`却下|見送|採用しない|断念|不採用` にマッチする 35 行 (重複 3)。
- 主題 2 の 27 件は削除行 `V2-13a-5h` の範囲に、主題 1 の大半は `V2-13a-5b` の範囲にある。
- 上表の test 名は 11 件すべてを削除行と現在の `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`
  の双方で照合した (2026-08-19)。改名されていたのは 1 件だけである。
- 当時の artifact は `ci-artifacts/native-linux-x86-hostgen-vm/<slice>/` 配下に置かれていたが、
  多くは診断後に削除されている。**artifact パスは再現手段としては当てにできない。**

## 移送しなかったもの

残り 1,271 行は移送しない。内訳と理由:

| 種類 | 理由 |
|---|---|
| 日付つき進捗ログの本文 (「fresh VM diagnostic ... は stderr 0 で完了した」等) | ADR は判断と却下理由を持つ正本であり、進捗は持たない (`.claude/rules/docs-organization.md`) |
| VM の disk 使用率、workdir 削除、artifact 回収サイズの記録 | 運用の一時状態であり、`docs/development/operations/` の運用手順にも当たらない |
| 各 slice の commit hash 列 | `git log` で引ける。ADR に写すと二重管理になる |
| `EC-M2` / 責務分離 (2026-07-26) の 30 節 | いずれも「ファイルを分割した」だけで、却下理由も原因記述も含まない (該当行 0) |
| Phase 11 / 14 / 15 の計画節 | `docs/development/planning/phase11-implementation-plan.md` が正本として現存する |
| `V2-15` の Lexer fixed radix 由来 (1 行) | 原因 (「`NativeCodegen.ls` が 1,000,000 bytes を超え、fixed radix が `end-pos` を巻き戻す」) が修正箇所の**コメントとして現存する** — `selfhost/src/Syntax/Lexer.ls:386` 「未終端文字列の末尾 escape は source-len + 1 を返せるため、それより大きい radix を使う」。回帰 test `test_selfhost_lexer_lex_result_encoding_scales_with_source_length` / `test_e2e_selfhost_lexer_lex_result_round_trips_large_and_trailing_escape_positions` も現存する |
| `V2-16` の ADT field key 由来 (1 行) | 同様に `selfhost/src/Backend/Wasm/CompilerBase.ls:57` に「Map runtime の 0 は空スロット判定に使うため、ADT field key は 1 始まりにする」というコメントが残り、実装 `(defn adt-constructor-field-key [idx] (+ idx 1))` と対になっている。回帰 test は `test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs` |

`V2-13a-5` (親項目) の 2 行はどちらも移送済みで、上表の 2 行に対応する — 1 行目が
`test_wasm_compiler_source_defn_step64_continuations_root_recursive_result_before_unwinding`
(64-step continuation の root 漏れ)、2 行目が
`test_selfhost_parser_parse_let_roots_returned_result_before_unwinding_bindings`
(`parse-let-v3` の返却 AST が binding roots の unwind 中に stale 化する穴)。
2 行目は先頭が artifact の green 記録 (`code_len=3590589` ほか) なので進捗ログに見えるが、
本文の後半で「step64 fix は直接 blocker ではない」と否定したうえで原因を `parse-let-v3` へ
絞り込んでおり、移送対象の「原因の絞り込み」に当たる。

**`V2-15` / `V2-16` の 2 行を移送しないと決めたのは、「原因が消える」条件を満たさないためである。**
`DOC-09` が問題にしているのは *`git log -S` でしか到達できなくなること* であって、行が消えること自体ではない。
この 2 件は原因が修正箇所のコメントとして読める位置にあり、回帰 test も名前ごと現存するので、
ADR へ写しても到達性は上がらない。逆に主題 1〜3 は、修正が入らなかった却下案の記録なので、
コメントとして残る先が無い。

## Related

- `I-27` — 本 ADR の主題 1 / 主題 2 が、その候補 (b) / (c) の裏付けにあたる
- `NATIVE-INLINE-01` — 主題 2 の一般則は、inline 展開で回避されている 5 件の背景である
- `docs/adr/decisions-v0.3-native-linux-stage2-entrypoint-rel32-diagnostic.md` —
  同じ rel32 を扱うが、そちらは診断手順の ADR で、却下案は持たない
