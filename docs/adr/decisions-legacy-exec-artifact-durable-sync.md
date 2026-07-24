# ADR: Wasm / native artifact の durability sync 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp_wasm::component_adapter` artifact writer / Mac native linker output

## Context

atomic `rename` は途中の artifact を公開しないが、temporary file の内容や rename を含む親
directory metadata が storage へ flush されたことまでは保証しない。クラッシュ直後の再起動で
古い、または空の artifact を選ぶ可能性を減らすには、publish 境界に file sync と parent sync
を加える必要がある。

## Decision

- Wasm artifact は temporary file に `write_all` した後 `File::sync_all` を行い、`rename` 後に
  Unix parent directory の `sync_all` を行う。
- Mac native executable は linker が生成した temporary output を同じ file sync API で flush し、
  `rename` 後に parent directory を flush する。
- sync failure は明示的な診断として返す。rename 後の parent sync failure では destination が
  置換済みになり得るため、rollback 成功とは扱わない。
- 非 Unix target では parent directory sync は no-op とし、対応 product target は Mac Apple
  Silicon / Linux x86_64 の Unix に限定する。

## Evidence

- RED: `test_artifact_sync_helpers_flush_file_and_parent_directory` は sync API 不在で compile
  failure になった。
- GREEN: 同テスト、Component artifact round-trip 8 tests、Mac native tests 2 tests が通過した。
- WasmGC compile 及び Component probe の既存 runtime gate は writer 接続後も再実行する。

## Residual risk

source fingerprint/manifest、Linux x86_64 native backend の actual artifact/runtime、selfhost stage0
release/rollback、external release bundle は未完了である。file/parent sync を release provenance
や Rust-free 全機能完了の証拠へ拡大解釈しない。
