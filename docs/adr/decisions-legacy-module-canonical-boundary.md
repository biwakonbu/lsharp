# ADR: canonical `App.Cli` compile の failure boundary を分離する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: canonical `selfhost/src/App/Cli.ls` の Rust compile/incremental regression と CLI driver compile
- Related: `decisions-legacy-type-substitution-fast-path.md`

## Context

Formatter 3 module の explicit import と SCC fast path を進めた後も、
`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` は 45 秒の bounded probe で
完了しなかった。単一の timeout だけでは、Formatter SCC の意味論、型推論の初回コスト、Rust test harness の
stack 容量、Wasm artifact の生成を区別できない。

## Decision

canonical compile の証跡を次の境界に分けて記録する。

1. SCC 単位の phase timing で Formatter 自体と downstream module の時間を分離する。
2. 長時間 regression test は test thread の stack 容量を明示して実行し、default stack の overflow を compiler
   failure と解釈しない。
3. CLI driver の default 経路は別に実行し、artifact byte size と Wasm validation/runtime を独立に確認する。
4. 上記の証拠が揃うまで `LEGACY-MODULE-01` と Formatter canonical parity を完了扱いにしない。

## Evidence

- Formatter 単体 entry (`LSHARP_PROFILE_ENTRY=Formatter.ls`) は `927 ms`、cache entries `5` で成功した。
- canonical `Cli.ls` の SCC timing では Formatter merged SCC は `648 ms`。一方、
  `App.CompilerMode` は `22,927 ms`、`App.Cli` singleton は `10,969 ms` であり、Formatter SCC だけが
  45 秒 probe の原因ではない。
- default test thread の canonical regression は `App.Cli` singleton 後に 2 MiB stack の overflow で終了した。
  これは compiler の型エラーや Wasm invalid を示す証拠ではない。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir
  test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds -- --nocapture` は
  `67.05 s` で `1 passed; 0 failed` となった。Rust host 上の cold/warm incremental parity の証拠だが、
  native stage0 の証拠ではない。
- default stack の CLI driver 経路でも
  `cargo run -p lsharp-driver -- compile selfhost/src/App/Cli.ls -o <tmp>/Cli.wasm` は成功し、
  `1,132,259 bytes` の artifact を生成した。Wasm validation、runtime、Mac/Linux native stage0 は別 gate とする。
- `cargo test -p lsharp-wasm --test e2e test_e2e_bootstrap_cli_fixed_input_compile_gate -- --nocapture` は
  expanded-stack acceptance harness 上で `66.36 s`、`1 passed; 0 failed` となった。canonical `Cli.ls` の
  Rust host compile と `wasmparser` validation は確認できるが、selfhost/native stage0 や Wasm runtime 実行の証拠ではない。

## Consequences

今後の C-1n/C-2 の作業は、Formatter の再変更を推測で続けず、`App.CompilerMode` / `App.Cli` の初回 inference、
CLI cache 接続、Wasm validation/runtime、対応 2 target の順に狭い contract として追加する。既定 test stack の
制約を回避するための環境変数は evidence command に限定し、意味論や公開 CLI の stack contract を変更しない。
`LEGACY-MODULE-01` の TODO は削除せず、canonical parity と native evidence が揃うまで active のまま残す。
