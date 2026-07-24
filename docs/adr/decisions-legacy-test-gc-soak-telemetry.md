# ADR: GC soak telemetry profiles

- Status: Accepted (verified slice; stateful soak blocker recorded)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / `LEGACY-RUNTIME-01` / imp-07 D-3

## Context

GC runtime bootstrap の collect/reuse/sentinel fixture と、ignored の長寿命 REPL / actual
`lsp --stdio` telemetry fixture が別々に存在していた。通常の `cargo test` は ignored fixture を
実行しないため、対象 command と failure boundary を明示した local lane が必要だった。

## Decision

[`scripts/ci/test-gc-soak-telemetry.sh`](../../scripts/ci/test-gc-soak-telemetry.sh) に profile を設ける。

- `--profile verified`: collect/reclaim、rooted reuse、legacy zero-root sentinel の 3 exact E2E を直列実行する。
- `--profile soak`: `--ignored` を明示して、in-session REPL telemetry と actual `lsp --stdio` telemetry の 2 exact E2E を実行する。
- `--profile all`: verified と soak を連続実行する。

ignored soak は未実行を成功とみなさない。現行 fixture は REPL 200-step と LSP 12-iteration の
stateful sequence で test thread stack overflow になるため、soak profile は blocker を再現する
diagnostic lane として扱い、verified profile の成功に吸収しない。

## Evidence

- RED: contract を先に追加し、未作成の lane script で実行失敗を確認した。
- Contract / syntax: `bash scripts/ci/test-gc-soak-telemetry-contract.sh` と `bash -n` → passed。
- Dry-run: verified 3、soak 2、all 5 の exact command 集合を profile 別に出力。
- Verified lane: `scripts/ci/test-gc-soak-telemetry.sh --profile verified` → 3 tests passed。
- Soak REPL: ignored exact test → `thread ... has overflowed its stack` / SIGABRT。
- Soak LSP: ignored exact test → `thread ... has overflowed its stack` / SIGABRT。
- REPL の `RUST_MIN_STACK=64M` 再実行でも同じ stack overflow となり、期待値を弱めず runner 環境だけで隠せないことを確認した。

## Consequences

通常 gate は実行可能な 3 件を安定して再検証でき、ignored stateful telemetry は明示的な RED として再現できる。
`LEGACY-TEST-01` / `GC-05` の aggregate 完了、再帰 stack boundary の解消、Mac/Linux native stage0、
Component parity、全 GC stress/static lint はこの ADR では完了扱いにしない。次の task は stateful
session fixture の stack-safe な実行境界を定義し、REPL/LSP telemetry を再度 GREEN にすることとする。
