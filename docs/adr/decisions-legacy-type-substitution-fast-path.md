# ADR: 空の型置換と単相 scheme の走査を省略する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-types::types::{Type, TypeScheme, TypeEnv, Substitution}`
- Related: `decisions-legacy-module-scc-import-dedup.md`

## Context

canonical Formatter の cyclic SCC 初回 inference を stack sampling したところ、主な実行時間は
`Infer::infer_program` 内の `Type::apply_subst` と `TypeEnv::apply_subst` にあり、空の置換でも型・環境を
再帰的に複製していた。また、束縛型変数を持たない単相 `TypeScheme` まで `Substitution::without` で
置換 map 全体を複製していた。

## Decision

`Substitution::is_empty` を公開し、次の意味保存 fast path を追加する。

1. 空の置換に対する `Type`、`TypeScheme`、`TypeEnv` の適用は再帰走査せず clone を返す。
2. `TypeScheme` が束縛型変数を持たない場合は `without` を通さず、元の置換を直接 `Type` へ適用する。

置換結果、generalization、visibility、diagnostic の契約は変更しない。非空かつ束縛変数を持つ scheme は
従来どおり restricted substitution を使う。

## Evidence

- RED: `substitution_reports_empty_without_allocating_a_map` を API 実装前に追加し、`Substitution::is_empty`
  未定義の compile error を確認した。
- GREEN: `lsharp-types` lib 206 tests、`lsharp-ir` lib（既知の長時間 Formatter test を除く）250 tests が成功した。
- Quality: `cargo clippy -p lsharp-types -p lsharp-ir --lib -- -D warnings`、rustfmt、`git diff --check` が成功した。
- Bounded canonical probe: `perl -e 'alarm 45; exec @ARGV' -- cargo test -p lsharp-ir
  test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds -- --nocapture` は変更後も
  45 秒で exit 142。初回 Formatter compile/runtime parity を成功扱いにはしない。

## Consequences

空置換と単相 binding が多い通常の inference では map 再構築と型走査を削減できる。これは意味論を変えない
局所最適化だが、Formatter の初回 full inference の残る failure boundary、CLI cache 接続、process 間 persistence、
selfhost 移植、native 2 target evidence は未完了のまま `LEGACY-MODULE-01` に残す。
