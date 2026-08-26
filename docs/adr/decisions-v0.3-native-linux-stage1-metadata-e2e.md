# ADR: Current-source Linux stage1 metadata boundary

- Status: Accepted (verified diagnostic slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`,
  `scripts/ci/native-linux-x86-hostgen-vm-exec.sh`
- Related: `V2-16e`, `LEGACY-BOOT-01`, `LEGACY-COMP-01`

## Context

The last full Linux x86_64 stage2 replay reached the VM memory boundary, so a fresh full replay
would repeat a known expensive failure without isolating whether current stage1 materialization and
the native execution entrypoint still work. The current checkout also needs a new stage1 artifact;
historical artifacts cannot be used as current-source provenance.

## Decision

Generate a current-source Linux stage1 on the host first. Then reuse that exact artifact in the Lima
VM and execute only metadata range `0..1`, stopping before the full stage2 chunk replay. Keep the
diagnostic summary separate from a fixed-point pass and stop the VM and remove all temporary
artifacts after evidence collection.

## Evidence

- `test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact` passed in
  `333.21s` for source commit `02201e1172ebb6dad8624186658f171fc9a88a3d`.

### 補足 (2026-08-24、`--ignored` lane 全量 sweep)

直上の Evidence は `LSHARP_NATIVE_LINUX_X86_ACTUAL_STAGE1_ARTIFACT_DIR` を設定し、
Lima VM `lsharp-linux-x86` を起動した状態での実測である。

`--ignored` lane 全量 sweep (2026-08-24) ではこの test は FAILED になったが、
**これは上の Evidence の反証ではない**。落ちているのは assertion ではなく
`std::env::var_os(`"LSHARP_NATIVE_LINUX_X86_ACTUAL_STAGE1_ARTIFACT_DIR"`).expect(..)` で、**前提の欠落による panic** である
(`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`)。sweep は
VM を起動すると 4 CPU を奪って計測を歪める ため、前提を揃えていない。

分類の根拠は [`decisions-native-root-pop-empty-guard.md`](decisions-native-root-pop-empty-guard.md)
の「分類規則」節にある。同節は本 test を (b) `LSHARP_NATIVE_*` env 依存に分類しており、
「回帰の候補になり得るのは (c) だけである」と述べている。

**したがって本 ADR の Evidence を再取得するには env と VM の前提を揃える必要がある。**
sweep のログだけでは真偽を判定できない。この点は `ISSUES.md` の `I-70` に記録した。
- Stage1 manifest: target `x86_64-unknown-linux-gnu`, code `4,393,425` bytes, data `2,757` bytes,
  entrypoint `4,390,965`, function-start length `3,409`, main function index `3,418`.
  Code SHA-256 was `625ec6a33f9f5722832eee6b9062994d680b48f6d1b6feba8e2db334629cbc9a`.
- `native-linux-x86-hostgen-vm-exec.sh` reused the stage1 in Lima `lsharp-linux-x86` with 16 GiB
  RAM and 12 GiB disk. VM free space was `7,688,683,520` bytes against the 4 GiB gate. Metadata
  range `0..1` returned summary `status=diagnostic`, stdout `8,353` bytes, stderr `0` bytes.

## Boundary

This proves current-source Linux stage1 generation, VM materialization, and a bounded native
metadata execution only. It does not prove full stage2/stage3 fixed-point, Linux native stage0
source-file smoke, App.Cli artifact/runtime, package acquisition/release/rollback, or Mac/Linux
parity. `V2-16e`, `LEGACY-BOOT-01`, and `LEGACY-COMP-01` remain `[~]` in `TODO.md`.
