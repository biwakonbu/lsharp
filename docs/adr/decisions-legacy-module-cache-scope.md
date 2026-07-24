# ADR: multi-file cache の entry scope isolation

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: Rust host の `lsharp-ir::CompilationCache`
- Related: `decisions-legacy-module-cache-api.md`

## Context

`CompilationCache` の module entry は module 名をキーにしているため、同じ process で別 project の
entry を compile すると、前の project の同名 module が cache に残り続ける。fingerprint が異なる
entry は再解析されるが、不要な stale entry と linked IR を保持し続けるため、CLI / host integration
で cache の scope が曖昧だった。

## Decision

- `CompilationCache::prepare_for_entry(entry_file)` を公開する。
- entry file の canonical directory を cache scope とし、scope が変わったら module entries と linked IR を破棄する。
- 同じ scope の warm compile は既存の fingerprint / IR cache を再利用する。
- module 名キーの置換、依存 SCC key、process 間永続化は後続 C-2 とする。

## Evidence

- RED: `test_compile_multi_file_with_cache_isolated_by_entry_root` は first project の cache を別 root
  へ持ち越し、stale module が 2 entry 残る状態を確認した。
- GREEN: 同テストは `prepare_for_entry` 後に second project の cache が 1 entry となり、`main` の IR が 42
  を保持することを確認する。
- `test_compile_multi_file_with_cache_matches_fresh_and_warm_compile`、lsharp-ir focused test、clippy、
  rustfmt、docs audit を通過した。

## Residual risk

これは process 内 cache の entry scope を明示した verified partial slice である。CLI driver の常駐
session への接続、依存 SCC の公開型 key、source override の SCC 統合、disk persistence、selfhost/native
stage0 parity は未完了で、`LEGACY-MODULE-01` / C-2 の aggregate 完了条件には到達していない。
