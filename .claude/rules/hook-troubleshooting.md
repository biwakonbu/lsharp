---
description: hooks/スキルの問題検知時に自動適用されるトラブルシューティングルール
globs:
  - ".claude/hooks/**"
  - ".claude/settings.json"
  - ".claude/commands/**"
  - ".claude/rules/**"
---

# hooks/スキル トラブルシューティング

## 問題発生時の対応

hooks やスキルに問題が発生した場合、以下の手順でサブエージェントを使って調査・修正すること。
自分で直接修正を試みる前に、まずサブエージェントで問題を切り分ける。

### 1. 調査 (Explore サブエージェント)

問題の種類に応じてサブエージェントを起動:

```
Agent(subagent_type="Explore", prompt="hook エラーの調査: /tmp/lsharp-hook-errors.log の内容と .claude/hooks/ のスクリプトを確認し、問題の根本原因を特定")
```

調査対象:
- `/tmp/lsharp-hook-errors.log` — hook のエラーログ
- `.claude/hooks/*.sh` — hook スクリプト本体
- `.claude/settings.json` — hooks 設定 (matcher, timeout 等)
- `.claude/commands/*.md` — スラッシュコマンド定義
- `.claude/rules/*.md` — ルールファイル (globs, 内容)

### 2. 修正

調査結果に基づいて修正。修正後は必ず動作検証を行う:

```bash
# tdd-guard.sh の検証
echo '{"tool_name":"Edit","tool_input":{"file_path":"crates/lsharp-types/src/infer.rs","new_string":"let x = 1;"}}' | .claude/hooks/tdd-guard.sh 2>&1

# test-result-tracker.sh の検証
printf '{"tool_name":"Bash","tool_input":{"command":"cargo test"},"tool_output":{"stdout":"test result: ok. 10 passed; 0 failed"}}' | .claude/hooks/test-result-tracker.sh 2>&1
```

### 3. よくある問題と対処

| 症状 | 原因 | 対処 |
|------|------|------|
| hook が無視される | settings.json の matcher ミス | `"Edit\|Write"` → `"Edit\|Write"` 正規表現確認 |
| jq パースエラー | 入力 JSON に制御文字 | jq の `--raw-input` や `// empty` フォールバック |
| grep エラー | macOS BSD grep 非互換 | `grep -oP` → `grep -oE` に変更 |
| タイムアウト | スクリプトが 10 秒超過 | 処理を簡略化、または timeout 値を増加 |
| 新規ファイルで誤警告 | `-f` チェック漏れ | `[[ ! -f "$FILE_PATH" ]]` ガード追加 |
| ログが書けない | /tmp 権限 | `LOG_FILE` のパスを確認 |

### 4. エラーログのフォーマット

```
[2026-03-22 14:30:00] [tdd-guard] エラーメッセージ
[2026-03-22 14:30:01] [test-tracker] エラーメッセージ
```

### 5. 設定変更時の注意

- `.claude/settings.json` を変更した場合、次のセッションから有効
- hook スクリプトの変更は即時反映
- `.claude/rules/*.md` の globs 変更は即時反映
