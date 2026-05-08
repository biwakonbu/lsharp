# V2-10: Native-only RC distribution

## 概要

native-only RC distribution は、Component Model pivot 後も公式配布の置き換えではなく experimental channel としてのみ提供する。
host launcher + embedded guest component が引き続き公式 distribution の正本であり、このページは actual native self-regeneration 成果物を調査用 RC として固める導線を定義する。

## 前提条件

- V2-08 native self-regeneration 完了
- V2-09 Wasm/native differential zero 完了
- signing / checksum / release workflow の正本 (`OPS-06`) が実運用できる

## 設計

### 配布チャネル

- experimental / opt-in の release channel を分離する
- 公式 component distribution と同じ tag を共有しても、artifact 名と release notes で明確に区別する
- stable 既定導線へは載せない
- asset 名は `experimental-native-rc-{version}-{target}.tar.gz` を使い、公式 `lsharp-{version}-{target}.{ext}` とは別物として扱う

### artifact 契約

- 初期 target は actual self-regeneration が green になっている Darwin arm64 (`aarch64-apple-darwin`) のみ
- `scripts/ci/build-native.sh` が生成する `ci-artifacts/native-proxy/{id}/` を source layout とし、top-level `manifest.json` / `actual-stage23-gap.json` と `stage1-native` / `stage2-native` / `stage3-native` を含める
- 各 stage directory は `program.o`, `runtime.o`, `linker-response.txt`, `program.native`, `stdout.txt`, `stderr.txt`, `summary.json` を必須にする
- `stage2-native` と `stage3-native` は actual self-regeneration 後の `summary.json` と transport payload が一致していることを smoke の前提にする
- checksum と signing を official archive と同水準で付与する。ただし official host launcher + embedded guest component 配布を置き換えないことを release notes に明記する
- `program.native` 系 artifact は host launcher + embedded guest component distribution の調査用 side artifact であり、stable 既定導線へ載せない

### release workflow

- official release workflow 内の `experimental-native-rc` job として、公式 `build` matrix とは別 artifact 名で扱う
- job は `macos-14` 上で Darwin arm64 actual native artifact を生成する
- package 前に `bash scripts/ci/native-only-rc-smoke.sh ci-artifacts/native-proxy/{id}` を実行する
- `experimental-native-rc-{version}-aarch64-apple-darwin.tar.gz` と `checksums.txt` を release asset として添付する
- smoke / verify の結果を experimental release notes に添付する
- ローカル再現は `NATIVE_PROXY_ARTIFACT_ID=<id> bash scripts/ci/build-native.sh` の後、`bash scripts/ci/native-only-rc-smoke.sh ci-artifacts/native-proxy/<id>` を実行する

## 正本参照

- plan 正本: [`../phase11-implementation-plan.md#v2-10-native-only-rc-distribution`](../phase11-implementation-plan.md#v2-10-native-only-rc-distribution)
- completion gate 境界: [`../completion-criteria.md`](../completion-criteria.md)
- 配布 / signing 正本: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)

## ステータス

完了。artifact layout / smoke test / 配布手順 / release workflow は actual native self-regeneration 成果物に合わせて固定済み。公式配布は引き続き host launcher + embedded guest component を正本とし、native-only RC は experimental channel としてのみ扱う。
