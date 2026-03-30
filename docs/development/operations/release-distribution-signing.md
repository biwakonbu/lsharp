# 配布 / 署名 / クロスビルド運用

native 配布物の生成・署名・公開チャネル・cross-build 方針を一箇所に集約した正本。ローカルでの実行手順は [`release-playbook.md`](./release-playbook.md)、artifact 名と保持期間は [`artifact-policy.md`](./artifact-policy.md)、CI の blocking graph は [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md) を参照する。

## 対象範囲

- stable / nightly の公開チャネル
- tier1 / tier2 の配布対象
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

## last-known-good (LKG) rollback anchor

- stable release は毎回 1 つの **rollback anchor** を持つ。
- rollback anchor の正本は **GitHub Release 上の stable tag + asset set** とし、package manager package は二次配布なので anchor にはしない。
- anchor に最低限含める情報は以下の 3 点:
  1. `last-known-good release tag` (`vX.Y.Z`)
  2. 同じ tag に紐づく host launcher archive / guest component package の asset 名
  3. 同じ release に添付した checksum file 名
- stable release は GitHub Release notes に `Rollback anchor` セクションを追記し、上記 3 点を明記してから完了扱いにする。
- nightly は継続検証チャネルであり、LKG anchor は更新しない。

## 公開チャネル

| チャネル | 目的 | 期待する成果物 | ブロッキング条件 |
|---|---|---|---|
| stable | エンドユーザー向け正式配布 | tier1 公式アーカイブ、checksum、release notes | `ci-gate-v2` 成功、release smoke 成功 |
| nightly | 継続検証と先行配布 | nightly アーカイブ、checksum | nightly workflow 成功 |

- stable は tag / GitHub Release を起点に扱う。
- nightly は `main` 系の継続検証チャネルとして扱う。

## プラットフォーム階層

| 階層 | 対象 | 配布方針 | CI での扱い |
|---|---|---|---|
| Tier1 | `macos-arm64`, `macos-x86_64`, `linux-x86_64`, `windows-x86_64` | 正式配布の必須対象 | release blocker |
| Tier2 | `linux-aarch64` | cross-build と smoke を維持する拡張対象 | non-blocking で段階導入 |

- Tier1 は stable / nightly の両チャネルで同一命名規則を使う。
- Tier2 は tier1 と同じ artifact 命名・checksum 規則を流用しつつ、release blocker にはしない。

## artifact と命名

- artifact 名・保持期間の正本は [`artifact-policy.md`](./artifact-policy.md)。
- 配布物のファイル名は `lsharp-{version}-{target}.{ext}` を基本形とし、target ごとの圧縮形式は release workflow で固定する。
- checksum は配布物と同時に生成し、release asset と同じ公開単位で扱う。

## 署名ポリシー

### 共通

- すべての公開配布物に checksum を付与する。
- 署名は checksum 生成後、release smoke 前に実行する。
- verify 手順は配布ジョブに含め、未検証 artifact を公開しない。

### Windows Authenticode

- 対象: `windows-x86_64` の `.exe`
- 手順:
  1. 証明書を release 用 secret / secure storage から取得
  2. `signtool.exe` で署名
  3. timestamp server を付けて verify
- verify 例:

```powershell
signtool verify /pa lsharp.exe
```

- 現状は設計段階であり、release-playbook の現行運用だけでは未接続。

## package manager 配布

- Homebrew / apt / scoop は **公式アーカイブから派生する二次配布** とする。
- version と checksum は GitHub Release 上の正本 artifact と一致させる。
- package manager 更新は release 後段に置き、24 時間以内の反映を目標にする。
- v1 では package manager 自体は best-effort で、正本は公式アーカイブ。
- rollback 時の復旧元も package manager package ではなく、LKG rollback anchor で指定した GitHub Release asset を使う。

## cross-build 方針

- cross-build は release workflow の packaging 段で扱う。
- tier2 `linux-aarch64` は cross-build + smoke を回すが、release blocker にはしない。
- target 追加時は artifact 命名・checksum・smoke 導線を tier1/tier2 表に追記してから workflow を増やす。

## 現状メモ

| 項目 | 現状 |
|---|---|
| `scripts/release-playbook.sh` | release binary を作り、bootstrap / default-path / README smoke まで実行可能 |
| tag push 起点の自動 release workflow | 未接続 |
| checksum 自動生成 | playbook にはあるが workflow 連携は未完 |
| Windows 署名 | 未実装 |
| package manager 配布 | 未実装 |
| `linux-aarch64` tier2 | 設計のみ |

## 関連ドキュメント

- [`release-playbook.md`](./release-playbook.md) -- 手元実行手順
- [`artifact-policy.md`](./artifact-policy.md) -- artifact 名と retention
- [`ci-gate-v2-job-graph.md`](./ci-gate-v2-job-graph.md) -- blocking CI graph
- [`../planning/v2-designs/v2-03-package-manager-distribution.md`](../planning/v2-designs/v2-03-package-manager-distribution.md)
- [`../planning/v2-designs/v2-04-linux-aarch64-tier2.md`](../planning/v2-designs/v2-04-linux-aarch64-tier2.md)
- [`../planning/v2-designs/v2-05-windows-authenticode-signing.md`](../planning/v2-designs/v2-05-windows-authenticode-signing.md)
