# 配布 / 署名 / クロスビルド運用

native 配布物の生成・署名・公開チャネル・cross-build 方針を一箇所に集約した正本。ローカルでの実行手順は [`release-playbook.md`](./release-playbook.md)、artifact 名と保持期間は [`artifact-policy.md`](./artifact-policy.md)、CI の blocking graph は [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md) を参照する。

## 対象範囲

- stable / nightly の公開チャネル
- supported product/release targets の配布対象
- release artifact / checksum / 署名 / smoke の順序
- package manager 配布と公式アーカイブの関係

## 正式フロー

```text
version bump
  -> ci-gate-v2
  -> package / archive
  -> checksum
  -> signing
  -> release smoke
  -> tag / GitHub Release
  -> rollback anchor
  -> package manager update
```

- `ci-gate-v2` が release 前提の blocking gate。
- 署名と package manager 更新は **公式アーカイブを正本** にしてぶら下げる。
- 詳細な手元実行コマンドは `scripts/release-playbook.sh` と `release-playbook.md` に寄せる。

## Native-only official replacement track

native-only archive は stable / nightly の正本であり、host launcher + embedded guest component は rollback compatibility 用の互換成果物へ降格済みである。

Supported product/release targets は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) の 2 つに限る。macOS Intel (`x86_64-apple-darwin`)、Windows (`x86_64-pc-windows-msvc`)、Linux ARM (`aarch64-unknown-linux-gnu`) は out of support scope であり、native-only 公式置換や release readiness の blocker には含めない。

V2-13 target matrix status は `docs/language/native-backend-spec.md` に正本化済みで、V2-14/V2-15 では native-only official archive layout / native-only release smoke / rollback anchor を stable 既定導線にした。current contract は次のとおり。

1. `aarch64-apple-darwin` と Linux x86_64 server priority track の actual self-regeneration evidence を保持する。Linux x86_64 の actual replay は Mac + Lima VM の local operator gate `scripts/ci/native-linux-x86-selfregen.sh` で扱い、GitHub Actions の required CI job や release workflow の `needs` には含めない。`stage23-map-insert-staged-merge-full-compare-v1` は stage2/stage3 byte-for-byte 一致、同一 SHA-256、stderr 0 で pass 済み。
2. `scripts/release.sh` と `scripts/ci/release-smoke.sh` は `program.native` / `manifest.json` / `checksums.txt` を native-only official archive の必須 payload として扱う。
3. `.github/workflows/release.yml` は stable build path で `NATIVE_ONLY_RELEASE=1` を渡す。current automated path は representative evidence artifact を package しているため、stable input を実 `App.Cli` native bundle に置き換える必要がある。Linux x86_64 の archive / smoke / publish wiring と実在する rollback compatibility asset は supported-target gap として扱う。
4. out of support scope の target に internal diagnostic coverage や archived design が残っていても、release blocker や必須 artifact にはしない。

stable release は native-only official archive を既定導線にする。host launcher + embedded guest component は rollback compatibility asset として保持し、default payload の `lsharp.component.wasm` companion sidecar には戻さない。

### native-only official archive layout

V2-14 で定義する native-only official archive layout は、host launcher + embedded guest component 配布から独立した target-native payload を正本にする。

必須 payload:

- `program.native`: target native executable。stable 既定導線へ昇格後は `lsharp` CLI の実体として扱う
- `manifest.json`: target triple、archive schema version、source commit、native backend evidence、entry binary、rollback compatibility asset への参照を記録する
- `checksums.txt`: archive 内 payload の SHA-256 一覧
- `README.md` / `LICENSE`: 配布物の利用条件と最小実行手順
- target metadata: object format、execution gate、signing/notarization status、known blocker を `manifest.json` 内に保持する

互換 payload:

- host launcher + embedded guest component archive は native-only official archive の rollback compatibility asset へ降格する
- `lsharp.component.wasm` companion sidecar は stable 既定 payload から外し、rollback compatibility / investigation 用 asset として扱う

V2-15 では `scripts/release.sh` / `scripts/ci/release-smoke.sh` / `.github/workflows/release.yml` をこの layout へ切り替え、native-only release smoke、checksum/signing/rollback anchor を stable 既定 gate にした。

## last-known-good (LKG) rollback anchor

- stable release は毎回 1 つの **rollback anchor** を持つ。
- rollback anchor の正本は **GitHub Release 上の stable tag + asset set** とし、package manager package は二次配布なので anchor にはしない。
- anchor に最低限含める情報は以下の 3 点:
  1. `last-known-good release tag` (`vX.Y.Z`)
  2. 同じ tag に紐づく native-only archive (`program.native` を含む) と rollback compatibility asset 名
  3. 同じ release に添付した checksum file 名、および archive 内 `manifest.json` の `rollback_anchor`
- stable release は GitHub Release notes に `Rollback anchor` セクションを追記し、上記 3 点を明記してから完了扱いにする。
- nightly は継続検証チャネルであり、LKG anchor は更新しない。

## 公開チャネル

| チャネル | 目的 | 期待する成果物 | ブロッキング条件 |
|---|---|---|---|
| stable | エンドユーザー向け正式配布 | supported target 公式アーカイブ、checksum、release notes | `ci-gate-v2` 成功、release smoke 成功 |
| nightly | 継続検証と先行配布 | nightly アーカイブ、checksum | nightly workflow 成功 |

- stable は tag / GitHub Release を起点に扱う。
- nightly は `main` 系の継続検証チャネルとして扱う。

## プラットフォーム階層

| 階層 | 対象 | 配布方針 | CI での扱い |
|---|---|---|---|
| Supported product/release targets | Mac Apple Silicon (`aarch64-apple-darwin`), Linux x86_64 (`x86_64-unknown-linux-gnu`) | 正式配布の必須対象 | release blocker |
| Out of support scope | macOS x86_64 (`x86_64-apple-darwin`), Windows (`x86_64-pc-windows-msvc`), Linux ARM (`aarch64-unknown-linux-gnu`) | 公式配布対象外。内部診断や archived design は可 | release blocker にしない |

- Supported product/release targets は stable / nightly の両チャネルで同一命名規則を使う。
- out of support scope の target を再導入する場合は、support scope の変更を TODO / spec / workflow / smoke に先に反映してから扱う。

## artifact と命名

- artifact 名・保持期間の正本は [`artifact-policy.md`](./artifact-policy.md)。
- 配布物のファイル名は `lsharp-{version}-{target}.{ext}` を基本形とし、target ごとの圧縮形式は release workflow で固定する。
- checksum は配布物と同時に生成し、release asset と同じ公開単位で扱う。現行 workflow では `release` job が top-level `dist/checksums.txt` を release-level checksum asset として生成・添付し、build job は native-only archive を stable 公開対象にする。host launcher + guest component は rollback compatibility asset としてのみ扱う。

## 署名ポリシー

### 共通

- すべての公開配布物に checksum を付与する。
- 署名は checksum 生成後、release smoke 前に実行する。
- verify 手順は配布ジョブに含め、未検証 artifact を公開しない。

### macOS notarization

- 対象: Mac Apple Silicon (`aarch64-apple-darwin`) の native-only archive
- 前提:
  1. `Developer ID Application` 証明書が release 用 secret / secure storage にある
  2. notarization 用の Apple ID credential / app-specific password もしくは API key が使える
- 手順:
  1. archive 展開後の `program.native` / `lsharp` alias に `codesign --options runtime --timestamp` を適用
  2. notarization 提出用の zip / pkg / dmg を生成
  3. `xcrun notarytool submit --wait` で Apple notarization へ提出
  4. pkg / dmg を使う場合は `xcrun stapler staple` で ticket を添付
  5. `spctl --assess -vv` または `codesign --verify --deep --strict` で verify
- verify 例:

```bash
codesign --verify --deep --strict lsharp
spctl --assess -vv lsharp
```

- embedded guest component (`.component.wasm`) は stable native-only archive の既定 payload ではない。rollback compatibility asset や investigation 用 asset として添付する場合だけ checksum / release asset 管理の対象にする。
- release workflow は macOS runner 上で `APPLE_CODESIGN_IDENTITY` と `APPLE_NOTARY_KEYCHAIN_PROFILE` が両方ある場合にだけ signing / notarization hook を実行し、`codesign --verify --deep --strict` / `spctl --assess -vv` / `xcrun notarytool submit --wait` を通す。credential 未設定時は skip し、native-only archive / rollback compatibility / checksum 契約だけを維持する。

### Windows Authenticode (archived / out of support scope)

- 対象: Windows は out of support scope であり、現行 release workflow の対象外。
- 手順:
  1. 証明書を release 用 secret / secure storage から取得
  2. `signtool.exe` で署名
  3. timestamp server を付けて verify
- verify 例:

```powershell
signtool verify /pa lsharp.exe
```

上記は archived design の記録であり、現行の supported product/release targets には含めない。Windows 対応を再開する場合は support scope の変更として扱い、`WINDOWS_SIGN_CERT_PFX_BASE64` / `WINDOWS_SIGN_CERT_PASSWORD` / `WINDOWS_TIMESTAMP_URL` を使う Authenticode hook を release workflow へ戻す前に TODO / native backend spec / smoke を更新する。

## package manager 配布

- Homebrew / apt / scoop は **公式アーカイブから派生する二次配布** とする。
- version と checksum は GitHub Release 上の正本 artifact と一致させる。
- package manager 更新は release 後段に置き、24 時間以内の反映を目標にする。
- v1 では package manager 自体は best-effort で、正本は公式アーカイブ。
- rollback 時の復旧元も package manager package ではなく、LKG rollback anchor で指定した GitHub Release asset を使う。

## cross-build 方針

- cross-build は supported product/release targets の packaging 段で扱う。
- target 追加時は artifact 命名・checksum・smoke 導線を supported/out-of-scope 表に追記してから workflow を増やす。

## 現状メモ

| 項目 | 現状 |
|---|---|
| `scripts/release-playbook.sh` | release binary を作り、bootstrap / default-path / README smoke まで実行可能 |
| tag push 起点の自動 release workflow | `verify` / `build` / `release-smoke` / `release` まで接続済み |
| checksum / native-only archive 自動生成 | `scripts/ci/build-native.sh` が actual native `stage3-native/program.native` を生成し、`scripts/release.sh` が `NATIVE_ONLY_PROGRAM` から archive 内 `program.native` / `lsharp` alias / `manifest.json` / `checksums.txt` を生成、`release` job が `bash scripts/checksum.sh dist > dist/checksums.txt` で attached checksum asset を追加、`scripts/ci/release-smoke.sh` が native-only payload と rollback anchor を workflow build job で検証 |
| macOS notarization | Mac Apple Silicon の secret-gated workflow hook まで接続済み。credential 未設定時は skip |
| Windows 署名 | archived design。Windows は out of support scope のため現行 release workflow から外す |
| package manager 配布 | 未実装 |

## workflow secrets

- `APPLE_CODESIGN_IDENTITY`: `codesign --sign` に渡す Developer ID identity
- `APPLE_NOTARY_KEYCHAIN_PROFILE`: `xcrun notarytool submit --keychain-profile` に渡す profile 名

現行 workflow はこれらの secret が未設定なら signing step を fail させず skip する。
したがって **workflow hook-up は repo 内で完了** しても、**実際の signing 完了判定** は credential が投入されるまで blocked のまま残る。

## 関連ドキュメント

- [`release-playbook.md`](./release-playbook.md) -- 手元実行手順
- [`artifact-policy.md`](./artifact-policy.md) -- artifact 名と retention
- [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md) -- blocking CI graph
- [`../planning/v2-designs/v2-03-package-manager-distribution.md`](../planning/v2-designs/v2-03-package-manager-distribution.md)
- [`../planning/v2-designs/v2-04-linux-aarch64-tier2.md`](../planning/v2-designs/v2-04-linux-aarch64-tier2.md)
- [`../planning/v2-designs/v2-05-windows-authenticode-signing.md`](../planning/v2-designs/v2-05-windows-authenticode-signing.md)
