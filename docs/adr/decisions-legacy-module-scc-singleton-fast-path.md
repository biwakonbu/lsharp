# ADR: singleton SCC の型推論重複を除去する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::infer_scc_type_surfaces`

## Context

モジュール graph を SCC 単位で推論する経路では、サイズ 1 の SCC も cyclic SCC と同じ merged
prepass と module 単位の visibility revalidation を通っていた。相互再帰を含まない通常の module
では同じ宣言を二度処理するため、incremental compile の初回推論コストが不要に増えていた。

一方、サイズ 2 以上の cyclic SCC では merged 推論によって相互再帰の型を同時に解決する必要があり、
その visibility revalidation は import の `:only` と private symbol を守るために必要である。

## Decision

サイズ 1 の SCC は merged prepass を経由せず、依存 closure の既知 type surface を direct import の
visibility に従って注入して module-local `Infer::infer_program` を一度だけ実行する。結果から private
symbol と expression type snapshot を作り、既存の `ModuleTypeSurface` 契約を返す。

サイズ 2 以上の cyclic SCC は従来の merged 推論と visibility revalidation を維持する。singleton の
self-loop も module-local 推論で扱い、self-recursive definition の解決は通常の Infer に委ねる。

この変更は segment reuse、dirty SCC の局所再推論、disk persistence、native stage0 parity を同時に
完了扱いにしない。

## Evidence

- RED: `test_compile_multi_file_incremental_infers_mutual_recursive_scc` に singleton の推論回数 1 を追加し、
  実装前は 0 で失敗。
- GREEN: 同テスト、`test_compile_multi_file_scc_preserves_import_only_visibility`、
  `test_compile_multi_file_infers_mutual_recursive_scc` が成功。
- Regression: `cargo test -p lsharp-ir --lib -- --skip test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  は 244 passed / 0 failed、clippy (`-D warnings`)、rustfmt、`git diff --check`、`scripts/audit_docs.sh` が成功。

## Consequences

acyclic な通常 module の型推論では merged + revalidation の重複がなくなる。cyclic SCC の意味論と
import visibility は維持される。canonical `Cli.ls` の初回 full inference は依然として長時間であり、
Formatter の compile/runtime parity と両 native target の証跡は後続タスクとして残る。
