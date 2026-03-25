---
paths:
  - "docs/**"
---

# docs/ ディレクトリの整備方針

## 構造と役割

```
docs/
├── language/          # 言語・runtime・backend の v1 契約仕様
├── development/
│   ├── planning/      # ロードマップ・Phase 計画・完了条件
│   ├── operations/    # CI 設定・ロールバック等の運用手順
│   └── validation/    # 検証仕様・自動生成ベンチマーク
├── adr/               # 意思決定ログ (JSONL)
├── guides/            # ユーザー向けチュートリアル (将来)
└── schemas/           # JSON Schema (lsharp-docs 等から参照)
```

## 配置ルール

| 種類 | 配置先 | 例 |
|------|--------|-----|
| 言語仕様・backend 契約 | `language/` | runtime-spec.md, backend-boundary.md |
| ロードマップ・Phase 計画 | `development/planning/` | phase11-implementation-plan.md |
| 運用手順 (恒久的に参照) | `development/operations/` | CI.md, rollback-procedure.md |
| 検証方針・ベンチマーク | `development/validation/` | BENCHMARK.md (自動生成) |
| 意思決定記録 | `adr/` | decisions-*.jsonl |
| ユーザー向けガイド | `guides/` | (将来) |

## 禁止事項

- **タスク単位の設計書・要件書を docs/ に置かない** — fire コマンドの specs/ や一時的な作業文書はリポジトリ外か `.claude/` 配下で管理
- **作業報告・進捗ログを docs/ に置かない** — ADR (adr/) で意思決定のみ記録する
- **自動生成ファイルを手編集しない** — `BENCHMARK.md` は `scripts/bench-report.sh` で生成

## 昇格ルール

タスク中に確定した恒久契約（API 境界、runtime 仕様等）は `language/` へ**要約して昇格**する。
要件定義書や設計書をそのまま移動しない。

## ドキュメント追加時のチェックリスト

1. 読者は誰か? → 配置先を決める
2. タスク完了後も参照されるか? → No なら docs/ に置かない
3. 既存ファイルに追記できないか? → ファイル増殖を避ける
4. README のリンクを更新したか? → `docs/README.md`, `development/README.md`
