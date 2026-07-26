# ADR: WASI string-eq helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-string-concat-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__string_eq` は二つの String object の length header を比較し、同じ長さ
の場合だけ linear memory の payload bytes を走査して i64 の 0/1 を返す独立した
code-emission 責務だが、他の helper と同じ parent にあり、WASI 変更の衝突単位を増やしていた。

## Decision

- `emit_string_eq_func` を `crates/lsharp-wasm/src/wasi/string_eq.rs`（99 行）へ移動する。
  これにより WASI production parent は 3543 行から 3448 行へ縮小する。
- Preview1 / Preview2 の helper registration は `string_eq` module 経由にし、function ordering、
  String length-header、payload offset `8`、unsigned byte comparison、i64 `0` / `1` return
  contract は維持する。
- `string_eq_tests.rs` の module seam test で helper body の登録を固定し、長さ不一致・空文字・
  同一 payload・異なる payload の既存 semantics は変更しない。

## Evidence

- RED: 空の `string_eq` module に対する seam test が `emit_string_eq_func` の unresolved
  import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib string_eq_module_emits_string_eq_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_string_eq -- --nocapture`
  （4 passed: true、false、different length、empty）。
- `cargo test -p lsharp-wasm --lib`（97 tests のうち 96 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `string-eq` code-emission の責務分離だけを扱う。String equality の全 backend
実装、長大入力、standalone/streamed runtime、Rust/native selfhost parity、dynamic memory
layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の完了を意味しない。
既存の `vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime checker の既知
failure、package-wide test-only lint debt は今回の差分外として残る。
