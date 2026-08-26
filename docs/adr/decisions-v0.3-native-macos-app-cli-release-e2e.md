# ADR: Current-source Mac App.Cli native release E2E

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`,
  `scripts/ci/native-macos-aarch64-selfhost-release.sh`
- Related: `V2-16c`, `V2-16e`, `LEGACY-COMP-01`, `LEGACY-BOOT-01`

## Context

Rust-host actual Wasm CLI evidence does not establish that the current selfhost source can
regenerate a native App.Cli program on a supported host. Existing historical artifacts were not
valid current-source evidence, so the current checkout needs a fresh stage2/stage3 fixed-point and
native release smoke.

## Decision

Use the existing Mac Apple Silicon native stage-chain test as the current-source release gate.
Require stage2/stage3 transport equality, a current source commit in the manifest, target
`aarch64-apple-darwin`, a fixed-point marker, a native program digest, and the `--version` smoke.
After generation, run no-arg help and `parse` as artifact-only postflight checks without keeping the
large temporary Cargo target or native artifact in the repository.

## Evidence

- `test_e2e_native_macos_aarch64_actual_app_cli_release_program` passed with `1 passed` in
  `945.94s`.

### 補足 (2026-08-24、`--ignored` lane 全量 sweep)

直上の Evidence は `LSHARP_NATIVE_MACOS_AARCH64_APP_CLI_ARTIFACT_DIR` を設定した状態での
実測である。

`--ignored` lane 全量 sweep (2026-08-24) ではこの test は FAILED になったが、
**これは上の Evidence の反証ではない**。落ちているのは assertion ではなく
`std::env::var_os(`"LSHARP_NATIVE_MACOS_AARCH64_APP_CLI_ARTIFACT_DIR"`).expect(..)` で、**前提の欠落による panic** である
(`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`)。sweep は
artifact dir を用意すると Cargo target と native artifact が数 GiB 残る ため、前提を揃えていない。

分類の根拠は [`decisions-native-root-pop-empty-guard.md`](decisions-native-root-pop-empty-guard.md)
の「分類規則」節にある。同節は本 test を (b) `LSHARP_NATIVE_*` env 依存に分類しており、
「回帰の候補になり得るのは (c) だけである」と述べている。

**したがって本 ADR の Evidence を再取得するには env と VM の前提を揃える必要がある。**
sweep のログだけでは真偽を判定できない。この点は `ISSUES.md` の `I-70` に記録した。
- Manifest: `target=aarch64-apple-darwin`, `source_commit=0dc6d67348195ad23575913841459cdf2e6a36b2`,
  `selfhost_fixed_point=true`, `program_sha256=a1dac9ff7146fbfd012c6e299df786c3c6c00680e3849cfb98abdeb1efcd76de`.
- The generated Mach-O arm64 program was 4,327,168 bytes; `--version` returned `lsharp 0.1.0`
  with empty stderr. Artifact-only no-arg help and `parse` of `(defn main [] 42)` both returned
  exit `0` with empty stderr; parse output was `decls:1`, `first-decl:defn`, `first-body:int`,
  `diagnostics:0`.

## Boundary

This is current-source Mac native App.Cli evidence. It does not prove Linux x86_64 native
regeneration, packaged stage0 acquisition/release/rollback, all public commands, component
sidecar generation, or cross-target artifact parity. `V2-16c`, `V2-16e`, `LEGACY-COMP-01`, and
`LEGACY-BOOT-01` remain `[~]` in `TODO.md`.
