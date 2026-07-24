# ADR: syntax parser の bounded property panic-safety

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-syntax/Cargo.toml`, `crates/lsharp-syntax/src/lib.rs`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

L# の syntax crate には lexer/parser の unit test はあるが、任意入力に対する panic-safety の property test と property-based test dependency が
なかった。parser は不正入力を診断へ投影する boundary なので、まず AST の意味論を生成する前に、bounded arbitrary bytes を受けても panic せず
`Result` を返す契約を追加する。

## Decision

- `proptest` は `lsharp-syntax` の dev-dependency に限定して導入し、配布 crate の runtime dependency へ入れない。
- `parser_never_panics_for_bounded_arbitrary_bytes` は 0〜127 bytes の arbitrary input を生成し、proptest 64 cases の各回で `parse` を
  `catch_unwind` 内から呼ぶ。panic ではなく成功または lexer/parser error を許容する。
- 入力長を bounded に固定し、深さ無制限の AST generator や fuzz/nightly profile はこの slice へ混ぜない。失敗した入力は proptest の
  regression seed として後続で昇格できる。

## Evidence

- RED: property test を先に追加し、`proptest` 未導入時の unresolved crate/macro compile error を確認した。
- GREEN: `cargo test -p lsharp-syntax parser_never_panics_for_bounded_arbitrary_bytes -- --nocapture --test-threads=1` が成功した。
- `cargo test -p lsharp-syntax -- --nocapture --test-threads=1` は 161 unit/integration/doc tests、`cargo clippy -p lsharp-syntax --lib --tests -- -D warnings`、
  rustfmt、`git diff --check`、`scripts/audit_docs.sh` も成功した。

## Consequences

lexer/parser の arbitrary-input panic regression を通常の local test で検出できる。一方、AST roundtrip、型推論 property、fuzz/nightly 4096 cases、
GC/rooting/limit coverage は未完了であり、`LEGACY-TEST-01` を完了扱いにはしない。
