# ADR: official multi-target gate の stage0 fetch 後 runtime smoke

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05`

## Context

`native-official-release-local.sh` は、native-only archive と stage0 archive を package し、
`fetch-stage0.sh` で manifest と payload checksum を確認していた。しかし fetch した stage0 の
compiler を実行していなかったため、package → fetch の成功だけでは current-source stage0 の
runtime evidence にならなかった。

## Decision

stage0 fetch の直後に、fetch 済み directory を target 固有の既存 source-file smoke へ渡す。

- `aarch64-apple-darwin`: `native-selfhost-dev-source-file-smoke.sh` を Mac host で実行する。
- `x86_64-unknown-linux-gnu`: `native-linux-x86-native-stage0-source-file-smoke.sh` を通して Lima
  VM 内で実行する。
- いずれも元の stage0 input ではなく `${SMOKE_ROOT}/stage0-${target}` を渡し、fetch 後の payload
  を実際に使用する。
- target dispatch にない値は明示的に失敗する。既存の source-commit、target、blocked host tool、
  Wasm magic、VM/temp cleanup の契約は各 smoke scriptへ委譲する。

provider snapshot の取得・認証や release archive の identity 検証はこの sliceで変更しない。
それらは N8 の offline propagation と release-smoke が担い、実 provider/runtime の証拠は別に閉じる。

## Evidence

- RED: `bash scripts/ci/test-native-official-release-snapshots.sh` は runtime invocation log がなく
  失敗した。
- GREEN: 同じ fake two-target harness が Mac/Lima wrapper の両方へ fetch 済み stage0 path が渡ること、
  既存の snapshot propagation と片側入力拒否を確認して通過した。
- RED: official gate の source-file runtime wrapperには source smoke evidence directoryの入力がなく、
  fetch後の stdout/stderr、Wasm digest/size、exit codeを release gateの外部証跡へ残せなかった。
- GREEN: `NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` を指定した場合、target別の fresh leafへ
  `NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR` または
  `LSHARP_NATIVE_LINUX_X86_SOURCE_SMOKE_EVIDENCE_DIR` を渡すようにした。Linux wrapperは evidence writerを
  VMへコピーし、成功・失敗の source smoke後に evidence directoryをhostへ再帰コピーする。rootの
  absolute/non-root、cleaned smoke root外、既存 leaf、symlinkを fail-closed に検査する。
- GREEN: `bash scripts/ci/test-native-official-release-snapshots.sh` の fake two-target harnessで両 targetの
  target別 path伝播と evidence保持を確認し、`bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  の Linux writer/copy contract、stage0 package/provider snapshot tests、shell syntax、diff checkを通過した。
- GREEN: `bash scripts/ci/test-native-linux-x86-source-smoke-evidence-copy.sh` の fake `limactl` で、Linux
  VM内の成功/失敗 source smokeが host evidenceへ再帰コピーされ、失敗時の元の exit code `23` が保持される
  ことを直接確認した。実 Lima VMは別の重い replayが所有しているため起動していない。
- Direct current-source Mac evidence: `e44ca72746e2c970588ac357979dc2b0bc8a67cc` の
  `native-macos-aarch64-stage0-release.sh` が actual App.Cli producer E2E（1 passed、491.75s）から
  `lsharp-native-selfhost-stage0` packageを生成し、manifestの target/source commit と
  `bin/compiler` / `bin/transport-driver` / `bin/materializer` を検証した。release archiveを作成し、
  local HTTPの `fetch-stage0.sh` で release/package checksum、target、source commitを再検証して
  fetched packageをインストールした後、producer/fetched双方の Mac source-file smokeを通過した。
  いずれも evidence manifestの `exit_code=0`、`compile.wasm` / `build.wasm` の同一 digest
  `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`（2,559 bytes）、stage0
  manifest digest `103566c49e1d16074e7cda46ff42ca59ea74c08210a41541d9b882adaa43586b` を確認した。
  current App.Cli producerの `--version`（`lsharp 0.1.0`）と `--help` も exit `0` / stderr空で通過した。
- GREEN: `native-official-release-local.sh` の開始時に
  `LSHARP_NATIVE_LINUX_X86_HOST_REPLAY_LOCK_DIR`（既定は hostgen VM lock）を検査し、別セッションが
  live hostgen replayを所有している場合は release/dist作成や `limactl` 呼び出しより前に exit `90` で
  fail-closed に停止する。fake lock contractと、実際に稼働中の parser replay lock（artifact/vm workdir/PID
  を含む）で preflight停止を確認した。stale/不正形状の lockも自動削除せず停止する。
- GREEN: 直接入口の `native-linux-x86-native-stage0-source-file-smoke.sh` も同じ lockを
  `ensure_vm_running` 前に検査し、fake lockで exit `90`、owner metadata、`limactl` 未呼び出しを確認した。
  既存の fake Linux smoke/evidence testsは absent lock pathを明示して実 lockとの競合を避ける。
- Direct Mac evidence: current `f6a6da30` の producer/package outputを Mac source-file smokeへ渡す経路は
  actual App.Cli E2E と `aarch64-apple-darwin native selfhost source-file smoke passed` まで通過した。
  これは `fetch-stage0.sh` を含む公式 orchestrator の証拠ではなく、Linux x86_64 runtimeも未検証である。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  と `git diff --check` を通過した。

この証拠は orchestrator の wiring、target別 evidence propagation、replay lock の所有境界、fresh current-source Mac stage0の
producer → package → fetch → source-file smoke境界を含む。provider snapshot digest bytes比較、
current checkoutと一致する Linux x86_64 stage0、Linux source-file smoke、両 targetの packaged App.Cli
`--version` / `--help` と rollback/Wasm parityは未取得であり、N9 と EC-M3-05 は `[~]` のまま残す。

## Consequences

公式 local gate は manifest の存在確認で止まらず、fetch した stage0 を target runtime smoke へ
接続できる。`NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` を指定すれば、smoke root cleanupとは別に
target別の stdout/stderr、Wasm digest/size、exit code evidenceを保持できる。実 target gate は重い処理なので、
既存の Lima lock/artifact を再利用して target ごとに一つだけ実行し、完了後に smoke script の cleanup 契約を適用する。
