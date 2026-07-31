# ADR: current-source Mac Apple Silicon stage0 producer

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-04-N1` / `M3-05-N9` / `EC-M3-04` / `EC-M3-05`

## Context

N9 の runtime gate は current checkout と一致する `lsharp-native-selfhost-stage0` を必要とする。
Mac には current-source の `App.Cli` native artifact を作る
`native-macos-aarch64-selfhost-release.sh`、Mac 用 transport driver、materializer、stage0 package
builder が個別に存在していたが、それらを一つの再現可能な producer として接続する入口がなかった。
古い `ci-artifacts` の App.Cli や stage0 は source commit が異なるため current evidence には使えない。

## Decision

`scripts/ci/native-macos-aarch64-stage0-release.sh` を Mac Apple Silicon 用の stage0 producer とする。

1. current checkout の source commit と macOS arm64 host を確認する。
2. 既存の `native-macos-aarch64-selfhost-release.sh` で current `App.Cli` native program/manifest を
   task-owned temporary work directoryへ生成する。
3. その `program.native` を compiler とし、`native-stage0-transport-macos-aarch64.sh` と
   `materialize-native-macos-aarch64-bundle.py` を `package-native-stage0.sh` へ渡す。
4. package manifest の target、source commit、payload path、executable bit を再検証し、指定 output
   が既に存在する場合は上書きせず fail-closed にする。

出力 stage0 directory は worktree の `ci-artifacts/native-stage0/aarch64-apple-darwin/` 配下、または
明示した `/tmp/lsharp-*` に限定する。producer が作る一時 App.Cli/Cargo directory は成功・失敗を問わず
回収する。provider snapshot、release archive、Linux VM runtime はこの sliceの scope 外である。

## Evidence

- RED: `bash scripts/ci/test-native-macos-aarch64-stage0-release.sh` は producer wrapper が存在せず
  失敗した。
- GREEN: 同じ fake producer/package harness が current source commit、Mac transport/materializer、
  stage0 manifest、既存 output の fail-closed を検証して通過した。
- GREEN: `TMPDIR=/tmp` を明示した current `f6a6da30` の Mac Apple Silicon producer/package が
  actual App.Cli E2E（542.31秒）を通過し、生成 stage0を
  `native-selfhost-dev-source-file-smoke.sh` へ渡して `aarch64-apple-darwin native selfhost source-file smoke passed`
  を確認した。
- RED: 既定 macOS `TMPDIR`（`/var/folders/...`）で同じ producerを起動すると、内側の
  `native-macos-aarch64-selfhost-release.sh` が task-owned temporary artifact pathを安全でないと拒否した。
- GREEN: safe cleanup boundaryを正規化した `TMPDIR_ROOT` の直下 `lsharp-*` に限定して許可し、root/
  traversal/任意 path は拒否する focused contract test と shell syntax を通過した。
- RED: 末尾 `/` を含む既定 `TMPDIR` では、外側 stage0 wrapperの未正規化連結によって
  `T//lsharp-native-macos-aarch64-stage0.../app-cli` が生成され、内側の安全検査に到達した。
- GREEN: stage0 wrapperでも `TMPDIR_ROOT="${TMPDIR_ROOT%/}"` を共通化し、既定
  `TMPDIR=/var/folders/.../T/` の actual producer/package を 484.89秒で完走した。
  `2abe8196c1cff06ae68265325b114e3c636e646` の source commit、Mac stage0 manifest、
  `aarch64-apple-darwin` target、および source-file smoke の一致を確認した。
- `bash -n scripts/ci/native-macos-aarch64-stage0-release.sh scripts/ci/test-native-macos-aarch64-stage0-release.sh`
  と `git diff --check` を通過した。

Mac Apple Silicon の current-source producer/package/source-file smoke は、既定 TMPDIR を含めて実行済みで、
task-owned path boundaryも fail-closed に固定した。ただし fetch後の公式 archive経路、Linux x86_64
runtime、provider snapshot digest、packaged App.Cli/rollback/Wasm byte parity は N9 の残件である。
`TODO.md` の `[~]` は維持する。

## Consequences

Mac target の current-source stage0 を同じコマンドで再生成でき、N9 の実 runtime gateへ渡す入力が
明確になる。producer は clean worktree と長時間 Cargo gateを要求するため、Linux VM replayと同時に
同じ target jobを重複起動せず、生成後は `native-official-release-local.sh` の fetch/runtime smokeへ
接続する。
