# language

`docs/language/` は、L# の v1 で契約として扱う仕様文書をまとめるディレクトリである。
ここでは実装メモや Phase 単位の作業計画ではなく、compiler・semantic system・backend・runtime の境界を
長期的に参照できる形で整理する。

## このディレクトリが扱う範囲

`docs/language/` が扱う主な領域:

- source、型、contract、semantic snapshot、change acceptance の言語契約
- frontend / lowering / codegen の境界契約
- backend 共通の runtime API、値表現、メモリ管理
- native backend の ABI、成果物、決定性要件

一方で、以下はこのディレクトリの主対象ではない。

- 構文や型システム全体の利用者向け tutorial
- Phase ごとの実装計画や移行手順
- 実装タスク単位の要件定義書 / 設計書
- ベンチマーク結果や検証レポート

これらは `book/` や `docs/development/` の文書で扱う。

## `development/specs` との関係

`docs/development/specs/` は、個別 subsystem の requirements、architecture、受入条件、実装順序を保持する。
日付、実装メモ、review 指摘、段階的な検証計画のような開発工程に結び付く情報を含めてよい。

一方で `docs/language/` は、実装後も参照し続ける規範契約を残す。`specs` の内容をそのまま複製せず、
次の条件を満たすものだけを反映する。

- language / compiler / backend / runtime の長期的な observable contract である
- 特定 Phase の進捗や一時的な実装都合に依存しない
- 後続実装者と tool が仕様として参照すべき内容である

`Semantic Contract System` では、言語として不変の source semantics と acceptance rule を
`docs/language/` に置き、module layout、work package、test ID は
`docs/development/specs/semantic-contract-system/` に置く。

## 文書一覧

| 文書 | 役割 | 主な読者 |
|------|------|----------|
| [`semantic-contract-system.md`](./semantic-contract-system.md) | static fact、checked claim、intent、fingerprint、delta、obligation、evidence、acceptance の規範契約 | compiler 実装者、tooling 実装者、AI agent |
| [`semantic-contract-language.md`](./semantic-contract-language.md) | `:case`、`:assert`、owner-bound `:property`、constrained type、typestate、legacy metadata の source semantics | language / type-system 実装者 |
| [`backend-boundary.md`](./backend-boundary.md) | frontend / lowering / codegen が受け渡す成果物と責務の境界 | compiler 実装者、IR 設計者 |
| [`runtime-spec.md`](./runtime-spec.md) | runtime API、値表現、GC root 管理、診断モデル | backend 実装者、runtime 実装者 |
| [`native-backend-spec.md`](./native-backend-spec.md) | native backend の対象 target、ABI、artifact、決定性要件 | native backend 実装者、配布基盤担当者 |

## 読み進め方

Semantic Contract System を実装・利用する場合:

1. `semantic-contract-system.md` で authority、trust、acceptance boundary を理解する
2. `semantic-contract-language.md` で source form の正確な意味を確認する
3. `../development/specs/semantic-contract-system/README.md` で module と algorithm を確認する
4. 同 directory の `operation-example.md` で end-to-end の運用を確認する
5. `implementation-plan.md` と `test-matrix.md` に従って TDD を進める
6. coding agent は `agent-execution-guide.md` の入出力・停止条件に従う

backend / runtime を実装する場合:

1. `backend-boundary.md` で compiler pipeline の責務分割を把握する
2. `runtime-spec.md` で backend 共通の実行時契約を確認する
3. `native-backend-spec.md` で native backend の ABI / linker 契約を確認する

## 文書間の関係

```text
Source (.ls)
  -> Frontend / Type System
  -> Canonical Semantic Snapshot
  -> Delta / Obligation / Evidence
  -> Accepted Semantic Result
  -> Lowering
  -> Codegen (Wasm / Native)
  -> CodegenArtifact
  -> Runtime

Canonical Semantic Snapshot
  -> Specification / API / Ontology / LSP / MCP projection
```

- `semantic-contract-system.md` は semantic snapshot と accepted change の意味を定義する
- `semantic-contract-language.md` は snapshot へ入る source-level contract を定義する
- `backend-boundary.md` は `FrontendResult`、`LoweredModule`、`CodegenArtifact` を定義する
- `runtime-spec.md` は backend が依存する共通 runtime 契約を定義する
- `native-backend-spec.md` は native `CodegenArtifact` の具体契約を定義する

## 記述方針

- **契約を先に書く**: 実装詳細より、何が保証されるかを先に定義する
- **authority を一つにする**: generated document や graph を第二の正本にしない
- **責務境界を明確にする**: frontend、semantic model、lowering、codegen、runtime を混ぜない
- **assurance を区別する**: static、checked、authored、attested を同じ「仕様」に潰さない
- **将来拡張を version 化する**: v1 requirement と将来 schema を混同しない
- **経緯と契約を分離する**: task history は `development/specs` / ADR に置く

現在の実装状況への言及は参考情報であり、規範本文より優先しない。
