# ADR: source override analysis の cache scope isolation

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::analyze_multi_file_incremental_with_overrides`
- Related: `decisions-legacy-module-cache-scope.md`

## Context

LSP の未保存バッファ解析は `CompilationCache` を常駐保持するが、override 入口だけは entry scope を
初期化していなかった。そのため同じ process で別 workspace を開くと、module 名が同じ stale entry と
linked IR が cache に残ったままになっていた。

## Decision

- `analyze_multi_file_incremental_with_overrides` の開始時に `cache.prepare_for_entry(entry_file)` を呼ぶ。
- source override の有無にかかわらず entry directory を cache scope として扱う。
- scope が変わった場合は module entries と linked IR を破棄し、現在 workspace の解析結果だけを保持する。
- SCC-aware override inference、依存 SCC key の統合、disk persistence、selfhost/native stage0 parity は後続 C-2 とする。

## Evidence

- RED: `test_analyze_multi_file_incremental_with_overrides_isolated_by_entry_root` は first workspace の
  2 module cache を second workspace へ持ち越し、stale entry が 2 件残ることを確認した。
- GREEN: 同テストは scope 切替後に second workspace の cache が 1 entry となることを確認する。
- lsharp-ir focused test、clippy、rustfmt、docs audit を通過した。

## Residual risk

これは LSP/override 入口の process 内 scope を閉じた verified partial slice である。既存の strict graph
経路は循環 module をまだ受理せず、Formatter SCC の special-case 除去と `LEGACY-MODULE-01` aggregate
完了条件には未到達である。
