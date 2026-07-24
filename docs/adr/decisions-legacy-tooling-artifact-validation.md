# ADR: compile artifact cache の target-aware Wasm validation

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-wasm/src/validation.rs`, `crates/lsharp-tooling/src/compile.rs`
- Related: `decisions-legacy-tooling-compile-session-artifact.md`, `decisions-legacy-tooling-compile-artifact-cache.md`

## Context

artifact envelope の key と payload checksum は stale input と破損 bytes を検出するが、checksum が一致する不正な Wasm を
実行可能 artifact として扱うことは防げない。特に cache を別 process から受け取る場合、target/backend に対応する parser/runtime
validation を hit と store の両方で通す必要がある。

## Decision

- `WasmValidationMode` を `Core`、`CoreWasmGc`、`Component` に分け、`wasmtime::Module` / `Component` を使って bytes を検証する。
- `CompileSession::with_artifact_cache` の Wasm target は key の target/backend に応じて validation mode を選ぶ。Native と不正な
  WasmGC target 組み合わせは cache hit しない。
- envelope が一致しても validation failure は cache miss とし、fresh compile へ戻る。生成後 bytes も validation failure なら
  cache/output を成功扱いせず error とする。
- `CompileSession::new` と default compile、Native executable は変更しない。この slice は structural validation であり、実行結果・
  WASI imports・component world contract の runtime validation は後続で閉じる。

## Evidence

- RED: checksum 付き `not-a-wasm` payload を cache に入れた session test が、未実装時は bytes を output へ返した。
- GREEN: `test_compile_session_artifact_cache_rejects_invalid_wasm_payload` (`1 passed; 0 failed`)。
- `cargo test -p lsharp-wasm validation::tests -- --nocapture` (`3 passed; 0 failed`)。
- `cargo test -p lsharp-tooling test_compile -- --nocapture --test-threads=1` (`51 passed; 0 failed`)。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings` と `cargo clippy -p lsharp-tooling --lib --tests -- -D warnings` が成功した。

## Consequences

cache hit が target/backend の構造検証を通るため、checksum だけでは見つからない不正 Wasm を fresh compile に戻せる。
runtime execution、CLI config、Native/selfhost stage0、Linux x86_64 evidence は未完了で、`LEGACY-MODULE-01` は active のまま残る。
