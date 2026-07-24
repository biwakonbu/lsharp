# ADR: merged SCC inference の重複 import を除去する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::merge_scc_declarations` / `infer_scc_type_surfaces`
- Related: `decisions-legacy-module-scc-merged-surface-fast-path.md`

## Context

cyclic SCC の merged inference は、SCC 内の各 module の宣言を一つの `Program` へ連結する。
従来は複数 module が同じ import を宣言している場合も、その import 宣言をすべて連結していた。
Formatter のように共通 module と相互参照先を各 module が import する SCC では、同じ型環境注入を
重ねるため、初回 inference の負荷を増やす。

一方で `:only`、alias、`open` が異なる import は可視性や名前解決の意味が異なるため、module 名だけで
まとめると import contract を変えてしまう。

## Decision

merged SCC 用の宣言連結を `merge_scc_declarations` に分離し、次の完全一致 key だけを重複除去する。

`(module, alias, only, open)`

最初に現れた宣言の順序を維持し、`defn` と nested module の所属情報も従来どおり保持する。parse 済み
module が欠落している場合は暗黙にスキップせず、明示的なエラーを返す。

## Evidence

- RED: `test_merged_scc_declarations_deduplicate_identical_imports` を helper 実装前に追加し、未定義
  helper の compile error を確認した。
- GREEN: 同テストと `test_merged_scc_declarations_keep_distinct_import_visibility` が成功し、同一 import
  は 1 件、異なる `:only` は 2 件を保持することを固定した。
- Regression: 既知の長時間 Formatter test を除く `lsharp-ir` lib 250 passed / 0 failed、clippy
  (`-D warnings`)、rustfmt、`git diff --check` が成功した。
- Bounded canonical probe: `perl -e 'alarm 45; exec @ARGV' -- cargo test -p lsharp-ir
  test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds -- --nocapture` は
  45 秒で exit 142。重複 import 除去後も初回 Formatter compile/runtime parity は未確定であり、成功扱いにしない。

## Consequences

同一の import 宣言が多い cyclic SCC では merged inference の入力が小さくなり、import contract を変えずに
重複した型環境注入を避けられる。canonical Formatter の初回 inference の残る failure boundary は別途切り分けが
必要であり、CLI driver cache 接続、process 間 persistence、selfhost 移植、native 2 target evidence とともに
`LEGACY-MODULE-01` の未完了条件として残す。
