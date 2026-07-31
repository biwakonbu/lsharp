# ADR: native official gate の current-source commit preflight

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `EC-M3-05-N9` / `scripts/ci/native-official-release-local.sh`
- Related: [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

`native-official-release-local.sh` は caller が渡した `SOURCE_COMMIT` を Mac/Linux の
App.Cli と stage0 package、release smoke へ伝播する。しかし preflight がなければ、current
checkout と異なる 40 桁 commit を受け取った場合でも、packaging や Lima smoke を開始してから
下流の provenance mismatch で失敗する。これは重い target gate を stale source で消費し、失敗
地点を release/runtime 境界まで遅らせる。

## Decision

- orchestrator の開始時に `git rev-parse --verify HEAD` で current checkout の commit を取得する。
- current checkout と `SOURCE_COMMIT` の双方を 40 桁小文字 hexadecimal として検査する。
- `SOURCE_COMMIT` が current checkout の HEAD と一致しない場合は、packaging、provider input、
  release smoke、Lima VM を開始せず、`SOURCE_COMMIT must match current checkout HEAD` の
  fail-closed 診断で終了する。
- source commit は caller が明示しても current checkout の provenance を上書きできない。既定値
  は従来どおり current HEAD とする。

## Evidence

- RED: `scripts/ci/test-native-official-release-snapshots.sh` に stale 40 桁 commit の two-target
  fixtureを追加し、実装前は release gate が成功して stale input を受理した。
- GREEN: 同テストが stale input を拒否し、release/package/smoke invocation log が増えないことを
  確認する。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  と `git diff --check` を通す。

## Boundary

これは current-source input を早期に固定する operator preflight であり、Mac Apple Silicon /
Linux x86_64 の actual stage0 regeneration、provider API/authentication、packaged runtime、
rollback、Wasm parity の evidence ではない。`EC-M3-05-N9` はこれらの証跡が揃うまで partial の
まま維持する。
