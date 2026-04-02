# V2-10: Native-only RC distribution

## 概要

native-only RC distribution は、Component Model pivot 後は公式配布の置き換えではなく experimental channel としてのみ検討する。  
host launcher + embedded guest component が引き続き公式 distribution の正本であり、このページは将来 native backend が十分成熟した後の RC 導線だけを定義する。

## 前提条件

- V2-08 native self-regeneration 完了
- V2-09 Wasm/native differential zero 完了
- signing / checksum / release workflow の正本 (`OPS-06`) が実運用できる

## 設計

### 配布チャネル

- experimental / opt-in の release channel を分離する
- 公式 component distribution と同じ tag を共有しても、artifact 名と release notes で明確に区別する
- stable 既定導線へは載せない

### artifact 契約

- tier1 target のみ対象とする
- checksum と signing を official archive と同水準で付与する
- `program.native` 系 artifact が host launcher 配布を置き換えないことを明記する

### release workflow

- official release workflow とは別 job / 別 environment を使う
- smoke / verify / signing の結果を experimental release notes に添付する

## 正本参照

- plan 正本: [`../phase11-implementation-plan.md#v2-10-native-only-rc-distribution`](../phase11-implementation-plan.md#v2-10-native-only-rc-distribution)
- completion gate 境界: [`../completion-criteria.md`](../completion-criteria.md)
- 配布 / signing 正本: [`../../operations/release-distribution-signing.md`](../../operations/release-distribution-signing.md)
- 手元リリース手順: [`../../operations/release-playbook.md`](../../operations/release-playbook.md)

## ステータス

Deferred。未着手。公式配布は引き続き host launcher + embedded guest component を正本とし、native-only RC は future experimental channel としてのみ扱う。
