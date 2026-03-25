# language

`docs/language/` は、L# の v1 で契約として扱う仕様文書をまとめるディレクトリである。
ここでは実装メモや Phase 単位の作業計画ではなく、compiler・backend・runtime の境界を長期的に参照できる形で整理する。

## このディレクトリが扱う範囲

`docs/language/` が扱うのは、主に次の 3 領域である。

- frontend / lowering / codegen の境界契約
- backend 共通の runtime API、値表現、メモリ管理
- native backend の ABI、成果物、決定性要件

一方で、以下はこのディレクトリの主対象ではない。

- 構文や型システム全体の利用者向けリファレンス
- Phase ごとの実装計画や移行手順
- 実装タスク単位の要件定義書 / 設計書
- ベンチマーク結果や検証レポート

これらは `book/` や `docs/development/` の文書で扱う。

## `development/specs` との関係

`docs/development/specs/` は、個別タスクの `requirements.md` / `design.md` を保持する場所である。
そこには日付、受入条件、実装メモ、レビュー指摘、段階的な設計判断のような、開発工程にひも付く情報を含めてよい。

一方で `docs/language/` は、タスクの経緯ではなく、実装後も参照し続ける契約を残す場所である。
そのため、`specs` の内容を丸ごと統合するのではなく、次の条件を満たす要素だけを抽出して反映する。

- backend や runtime の長期的な境界契約である
- 特定 Phase の進捗や受入条件に依存しない
- 後続実装者が仕様として参照すべき内容である

例えば、次のような内容は `language/` へ昇格する対象になる。

- `FrontendResult` / `LoweredModule` / `CodegenArtifact` の境界契約
- runtime API の公開面
- native backend の ABI や artifact 形式

逆に、次のような内容は `specs` 側へ残す。

- タスク分割、完了条件、受入条件
- 実装順序や移行手順
- 実績サマリー、レビュー指摘、暫定メモ

## 文書一覧

| 文書 | 役割 | 主な読者 |
|------|------|----------|
| [`backend-boundary.md`](./backend-boundary.md) | frontend / lowering / codegen が受け渡す成果物と責務の境界を定義する | compiler 実装者、IR 設計者 |
| [`runtime-spec.md`](./runtime-spec.md) | runtime API、値表現、GC root 管理、診断モデルを定義する | backend 実装者、runtime 実装者 |
| [`native-backend-spec.md`](./native-backend-spec.md) | native backend の対象ターゲット、ABI、artifact 形式、決定性要件を定義する | native backend 実装者、配布基盤担当者 |

## 読み進め方

初めて読む場合は、次の順で参照すると全体像を追いやすい。

1. `backend-boundary.md` で compiler パイプラインの責務分割を把握する
2. `runtime-spec.md` で backend 共通の実行時契約を確認する
3. `native-backend-spec.md` で native backend が追加で満たすべき ABI / linker 契約を確認する

## 文書間の関係

```text
Source (.ls)
  -> Frontend
  -> FrontendResult
  -> Lowering
  -> LoweredModule
  -> Codegen (Wasm / Native)
  -> CodegenArtifact
  -> Runtime
```

- `backend-boundary.md` は `FrontendResult`、`LoweredModule`、`CodegenArtifact` を定義する
- `runtime-spec.md` は backend が依存する共通 runtime 契約を定義する
- `native-backend-spec.md` は `CodegenArtifact` のうち native backend 側の具体契約を定義する

## 記述方針

このディレクトリの文書では、次の観点を優先する。

- **契約を先に書く**: 実装詳細よりも、何が保証されるべきかを先に示す
- **責務境界を明確にする**: frontend、lowering、codegen、runtime の責務を混ぜない
- **将来拡張の余地を残す**: v1 の必須要件と、将来拡張可能な領域を区別する
- **経緯と契約を分離する**: タスクの履歴は `development/specs` に残し、恒久契約だけを本ディレクトリへ反映する

必要に応じて現在の実装状況に触れるが、それは参考情報であり、仕様本文より優先しない。
