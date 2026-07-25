# ADR: metadata checker test-generation production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/metadata_check.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-metadata-check-reference-split.md`

## Context

`metadata_check.rs` は metadata 診断、legacy invariant の型検査、reference helper、
executable test generation を一つの production module に抱えていた。reference helper は
既に `metadata_check/references.rs` へ分離済みだが、test-generation の型と property smoke
profile が親 module に残り、責務境界と focused review の単位がまだ混ざっていた。

## Decision

- `GeneratedTest`、`PropertySmokeTestSpec`、`PropertyBinderType`、`TestKind`、
  `property_smoke_test_spec`、`generate_tests` を `metadata_check/test_generation.rs` に移す。
- `metadata_check.rs` はこれらを `pub use` で再公開し、`lsharp_types::metadata_check::*` の
  既存 API と `metadata_test` / Wasm test runner の import path を変更しない。
- 実行順、property profile の受理条件、生成 test 名は変更しない。module seam test と
  package 回帰で parity を確認する。

## Evidence

- `test_generation_module_exposes_property_smoke_profile` は新 production module の profile
  seam と binder/cases projection を固定する。
- `cargo test -p lsharp-types -- --nocapture`（210 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは test-generation production の責務分離だけを扱う。metadata diagnostics、legacy
contract migration、parser/lexer production split、native/selfhost parity、I-01 / I-08
aggregate の完了を意味しない。
