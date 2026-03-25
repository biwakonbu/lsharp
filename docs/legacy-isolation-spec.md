# Rust 実装の段階的隔離と削除 仕様 (P11-6b)

## 概要

Rust 実装 (legacy reference) の段階的隔離と最終削除の方針を定める。
L# セルフホストが feature parity を達成した後、Rust 実装を安全に撤去するための
隔離ディレクトリ戦略、L# 正本化、tag 固定、順次削除手順の 4 軸で構成する。

本仕様は以下に依存する:

- **P11-3** コンパイラ中核の Rust parity (docs/rust-parity-spec.md)
- **P11-5** 差分判定規則 (docs/gap-classification.md)

---

## P11-6b: Rust 実装の段階的隔離と削除 (トップレベル方針)

### 隔離の原則

- Rust 実装は一括削除せず、feature parity 単位で段階的に隔離・削除する
- 隔離中は shadow mode 比較 (P11-3e-2) で動作の同値性を継続検証する
- 削除は全 golden test / differential test の差分ゼロ確認後に実施する

### 削除の前提条件

- 対象モジュールの L# 実装が shadow mode で 1 週間以上差分ゼロを維持
- compatibility-matrix.md の対応行が「L# parity test 全 PASS」を達成
- ADR に削除判断の根拠と承認者を記録済み

### 正本の定義

- 正本 (canonical implementation) = mainline でデフォルト実行される実装
- Phase 11 完了後、正本は L# 実装に移行し、Rust 実装は legacy 参照のみとなる

---

## P11-6b-1: 隔離方針

### 隔離ディレクトリ戦略

Rust 実装を `legacy-rust-bootstrap/` ディレクトリに集約し、mainline から分離する。

**ディレクトリ構成**:
```
legacy-rust-bootstrap/
  crates/
    lsharp-syntax/     -- Rust lexer/parser
    lsharp-types/      -- Rust 型推論
    lsharp-ir/         -- Rust IR 変換
    lsharp-wasm/       -- Rust Wasm backend
    lsharp-driver/     -- Rust CLI
    lsharp-lsp/        -- Rust LSP
    lsharp-docs/       -- Rust docs
  Cargo.toml           -- 隔離用ワークスペース定義
  README.md            -- legacy 参照の説明と注意事項
```

### 隔離の段階

| 段階 | 状態 | 条件 |
|------|------|------|
| active | mainline で既定実行 | parity 未達成 |
| shadow | L# が既定、Rust が shadow 検証用 | parity 達成、安定期間中 |
| isolated | `legacy-rust-bootstrap/` に移動 | 安定期間 (1 週間) 完了 |
| archived | tag 付きで参照のみ | 全 parity 完了、ADR 承認済み |
| deleted | ソースツリーから除去 | archive 後 1 リリースサイクル経過 |

### ブランチ戦略

- 隔離作業は `legacy/isolate-{crate-name}` ブランチで実施する
- mainline への merge 前に以下を確認:
  - L# 実装の CI が全 green
  - shadow mode 差分レポートが空
  - `legacy-rust-bootstrap/` 内の Rust コードが独立してビルド可能

### Cargo workspace の分離

- 隔離された Rust crate は mainline の `Cargo.toml` workspace members から除外する
- `legacy-rust-bootstrap/Cargo.toml` に独立した workspace を定義する
- 隔離後も `cargo build` / `cargo test` は mainline の L# ツールチェインのみで動作する

---

## P11-6b-2: L# 正本化

### mainline の既定コマンド

L# 実装が parity を達成した機能から順次、mainline のデフォルト実行パスを切替える。

**切替え対象と優先順位**:

| 優先度 | 対象 | 切替え条件 |
|--------|------|-----------|
| 1 | `lsharp compile` | Wasm backend parity 達成 |
| 2 | `lsharp check` | 型推論 parity 達成 |
| 3 | `lsharp parse` | parser parity 達成 |
| 4 | `lsharp test` | test runner parity 達成 |
| 5 | `lsharp build` | 全パイプライン parity 達成 |
| 6 | `lsharp fmt` | formatter parity 達成 |
| 7 | `lsharp lsp` | LSP parity 達成 |
| 8 | `lsharp doc` / `lsharp review` 等 | docs 系 parity 達成 |

### README 更新方針

- L# が正本になった時点で README の Quick Start を L# ネイティブバイナリ前提に書き換える
- Rust ビルド手順は `legacy-rust-bootstrap/README.md` に移動する
- mainline README から `cargo build` / `cargo run` の手順を除去する

### CI 切替え

- L# 正本化された機能は CI で L# パスのみを実行する
- Rust パスは `legacy-rust-bootstrap/` 内の独立 CI job に分離する
- 独立 CI job は `test-legacy-rust` として非 blocking (warning) で実行する
- 全 Rust crate が isolated になった時点で `test-legacy-rust` を廃止する

### ドキュメント正本化

- `book/` 内のドキュメントは L# 実装を前提とした記述に更新する
- Rust 固有の情報 (crate 構成、Cargo 依存等) は `book/appendix/rust-legacy.md` に集約する
- API ドキュメントは L# の `lsharp doc` 出力を正本とする

---

## P11-6b-3: tag 確定

### 最終 commit の tag 命名規則

Rust 実装の最終参照点を以下の tag で固定する:

**命名パターン**: `legacy-rust-{crate}-v{version}`

| tag 例 | 対象 | 意味 |
|--------|------|------|
| `legacy-rust-syntax-v1.0.0` | lsharp-syntax | Rust syntax crate の最終版 |
| `legacy-rust-types-v1.0.0` | lsharp-types | Rust types crate の最終版 |
| `legacy-rust-ir-v1.0.0` | lsharp-ir | Rust IR crate の最終版 |
| `legacy-rust-wasm-v1.0.0` | lsharp-wasm | Rust Wasm backend の最終版 |
| `legacy-rust-driver-v1.0.0` | lsharp-driver | Rust CLI の最終版 |
| `legacy-rust-lsp-v1.0.0` | lsharp-lsp | Rust LSP の最終版 |
| `legacy-rust-all-v1.0.0` | 全 crate | 全 Rust 実装の最終集約版 |

### tag の固定ルール

- tag は軽量 tag ではなく annotated tag を使用する
- tag メッセージに以下を含める:
  - 隔離理由と ADR 番号
  - parity 達成日
  - shadow mode 差分ゼロ確認日
  - 承認者
- tag 後の Rust コードへの変更は禁止 (セキュリティ修正を除く)
- セキュリティ修正が必要な場合は `legacy-rust-{crate}-v{version}.{patch}` で新 tag を発行

### 参照点の利用方法

- golden test の期待値生成に使用する (Rust 実装の出力を正とする)
- デバッグ時に Rust 実装の挙動を確認するための参照実装として利用する
- 性能比較のベースラインとして利用する

### tag の保持期間

- tag は永続的に保持する (削除しない)
- `legacy-rust-bootstrap/` ディレクトリが削除された後も tag は git 履歴に残る
- tag から checkout して Rust 実装を再ビルド可能な状態を維持する

---

## P11-6b-4: 順次削除手順

### 削除単位

feature parity 完了単位で削除する。削除単位は compatibility-matrix.md の行に対応する。

**削除順序** (依存関係の逆順):
```
1. lsharp-docs   (最も依存が少ない)
2. lsharp-lsp    (driver に依存)
3. lsharp-driver (全 crate に依存)
4. lsharp-wasm   (IR に依存)
5. lsharp-ir     (types, syntax に依存)
6. lsharp-types  (syntax に依存)
7. lsharp-syntax (最も多く依存される)
```

### 削除前チェックリスト

各 crate を削除する前に以下を全て確認する:

| チェック項目 | 検証方法 |
|------------|---------|
| L# parity test 全 PASS | CI の `test-parity-{crate}` job |
| golden test 差分ゼロ | `scripts/compare-golden.sh {crate}` |
| shadow mode 差分ゼロ (1 週間以上) | CI history の `test-shadow-{crate}` |
| 他 crate からの依存なし | `cargo tree -i {crate}` で確認 |
| compatibility-matrix.md 更新済み | 手動確認 |
| ADR 記録済み | `docs/adr/decisions-003.jsonl` |
| tag 発行済み | `git tag -l 'legacy-rust-{crate}-*'` |

### 削除の実施手順

```
1. 削除対象 crate を `legacy-rust-bootstrap/` から除去
2. mainline の Cargo.toml から workspace member を削除 (既に除外済みのはず)
3. CI から `test-legacy-rust-{crate}` job を削除
4. compatibility-matrix.md の Deletion gate を「削除済み」に更新
5. TODO.md に削除完了を記録
6. ADR に削除実施日と最終確認結果を追記
```

### dead code 回避策

Rust 実装の削除によって dead code が発生しないよう、以下の対策を講じる:

**テスト側の対策**:
- E2E テスト (`crates/lsharp-wasm/tests/e2e.rs`) で Rust パスを呼び出すテストは
  L# パスに移行済みであることを確認する
- `#[cfg(test)]` 内の Rust 固有ヘルパーは削除対象 crate と同時に削除する
- テストが削除対象 crate を `use` している場合、L# 実装への参照に切替える

**CI 側の対策**:
- `cargo clippy -- -D dead_code` を CI gate に含め、dead code をマージ前に検出する
- `scripts/audit_dead_code.sh` で Rust 実装への参照が残っていないことを確認する

**ドキュメント側の対策**:
- 削除済み crate への言及を grep で検出し、ドキュメントを更新する
  (`scripts/audit_docs.sh` に検出ルールを追加)
- `CLAUDE.md` のワークスペース構成セクションを更新する

### ロールバック手順

削除後に重大な問題が発見された場合のロールバック手順:

1. legacy tag から対象 crate のソースを checkout する
2. mainline の Cargo.toml に workspace member を再追加する
3. CI job を復元する
4. 原因調査と修正を行い、再度削除手順を実施する

ロールバックが 2 回以上発生した場合、削除判断の再審議を ADR で行う。

---

## 依存関係

| 依存先 | 前提条件 | 理由 |
|--------|----------|------|
| **P11-3** compiler parity | L# 実装が Rust 実装と同一挙動 | 削除の前提として parity が必要 |
| **P11-3e** parity 移行順 | shadow mode 比較が完了 | 安全な切替えの保証 |
| **P11-3f** 完了条件 | golden test 全通過 | 削除判断の根拠 |
| **P11-5** gap classification | 差分分類が確定 | 残存差分の評価に使用 |

---

## リスクと制約

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 削除後に未検出の parity 差分が発覚 | ユーザーへの影響 | tag からのロールバック手順を整備 |
| 他 crate への暗黙的依存が残存 | ビルド失敗 | `cargo tree` による依存解析を必須化 |
| CI 設定の不整合 | テスト漏れ | CI 設定の diff レビューを必須化 |
| ドキュメントの Rust 参照が残存 | 混乱 | `scripts/audit_docs.sh` で自動検出 |

---

## 用語定義

| 用語 | 定義 |
|------|------|
| **legacy reference** | Rust で実装された旧コンパイラ群。L# セルフホスト完了後は参照実装としてのみ利用 |
| **隔離 (isolation)** | Rust 実装を `legacy-rust-bootstrap/` に移動し、mainline の実行パスから切り離すこと |
| **正本 (canonical)** | mainline でデフォルト実行される実装。Phase 11 完了後は L# 実装を指す |
| **shadow mode** | L# と Rust の両方を実行し、出力を比較する検証モード |
| **annotated tag** | git のメタデータ (メッセージ、作成者、日時) 付きの tag |
| **dead code** | 実行パスから到達不能になったコード。clippy で検出可能 |
| **deletion gate** | Rust 実装を削除可能になる条件。compatibility-matrix.md で管理 |
| **feature parity** | L# 実装が Rust 実装と同一の機能・挙動を持つ状態 |
