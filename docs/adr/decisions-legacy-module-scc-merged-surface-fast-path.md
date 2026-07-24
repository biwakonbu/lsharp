# ADR: visibility 制約のない cyclic SCC で merged type surface を再利用する

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: lsharp-ir::infer_scc_type_surfaces

## Context

cyclic SCC は相互再帰を解くため merged inference を行った後、各 module を個別に再推論して import visibility と
expression type table を復元していた。FormatterExpr / FormatterDecl / Formatter のように全 import が公開で
private 宣言もない SCC では、この再検証が重複コストになる。

ただし :only、private symbol、同名 scope の expression type は module 境界を守るため個別再検証が必要である。

## Decision

merged inference 後、次の全条件を満たすときだけその結果を直接 ModuleTypeSurface へ分配する。

1. group 内の import に :only がない。
2. merged inference の private symbol がない。
3. 全 ExprTypeKey.scope が AST の defn / impl method scope から一つの module に決定的に割り当てられる。

scope が未知または複数 module に衝突する場合は fast path を使わず、既存の module 単位 visibility revalidation へ戻す。
これにより一般化した最適化を保ちつつ、公開 surface と expression type parity の安全境界を維持する。

## Evidence

- RED: test_compile_multi_file_unrestricted_scc_uses_merged_surface_fast_path は実装前、公開 A↔B SCC の fast path count が 0。
- GREEN: 同テストで merged surface fast path count 1。SCC/multi-file focused 11 tests、既知 Formatter probe を除く
  lsharp-ir regression 248 passed / 0 failed、clippy (-D warnings)、rustfmt、git diff --check、docs audit が成功。
- Bounded canonical probe: perl -e 'alarm 45; exec @ARGV' -- cargo test -p lsharp-ir test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds -- --nocapture
  は 45 秒で exit 142。これは初回 Formatter parity の成功証拠ではない。

## Consequences

visibility 制約がない cyclic SCC では merged inference の再検証を省略できる。制約がある group は既存の安全な経路を使う。
canonical Formatter の初回 inference/runtime parity、source override の同等 fast path、disk persistence、両 native target の
実行証跡は後続タスクとして残る。
