---
description: hooks/スキルの問題検知時に適用されるトラブルシューティングガイド
globs:
  - ".claude/hooks/**"
  - ".claude/settings.json"
  - ".claude/commands/**"
  - ".claude/rules/**"
---

# hooks/スキル トラブルシューティング

## 重要: 正常な情報メッセージについて

hook の stderr 出力で `[TDD Guard]` や `[TDD Tracker]` 接頭辞のメッセージは**正常な情報通知**であり、エラーではない。
これらのメッセージが表示されても、トラブルシューティングを起動する必要はない。

## 問題発生時の対応

hooks やスキルに**実際の問題**（hook がクラッシュする、設定が壊れた等）が発生した場合、以下の手順でサブエージェントを使って調査・修正できる。

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

# doc-guard.sh の検証
# 注意: doc-guard は「対象 repo の working tree に docs 差分があるか」で判定するため、
# 差分がある worktree で叩くと必ず無音になる。警告経路を見たいときは差分の無い
# 一時 repo (git init + 実装ファイル 1 個) を作ってそのパスを渡すこと。
printf '{"tool_name":"Edit","tool_input":{"file_path":"/tmp/fixture/crates/foo/src/lib.rs","new_string":"fn b(){}"}}' | .claude/hooks/doc-guard.sh 2>&1
```

doc-guard の検証済み期待挙動 (2026-08-16):

| 入力 | 期待 |
|---|---|
| 実装ファイル + 正本 docs 未変更 | `[Doc Guard]` 警告 |
| 実装ファイル + `TODO.md` 変更済 | 無音 |
| 未追跡の新規 `docs/adr/*.md` あり | 無音 (untracked も差分として数える) |
| `*_tests.rs` / `tests/` 配下 / `scripts/ci/test-*` | 無音 |
| 対象外ファイル (README 等) | 無音 |

いずれも `exit 0`。doc-guard はブロックしない。

### 3. よくある問題と対処

| 症状 | 原因 | 対処 |
|------|------|------|
| hook が無視される | settings.json の matcher ミス | 正規表現確認 |
| jq パースエラー | 入力 JSON に制御文字 | jq の `// empty` フォールバック |
| grep エラー | macOS BSD grep 非互換 | `grep -oP` → `grep -oE` に変更 |
| タイムアウト | スクリプトが timeout 超過 | 処理を簡略化、または timeout 値を増加 |
| 新規ファイルで誤警告 | `-f` チェック漏れ | `[[ ! -f "$FILE_PATH" ]]` ガード追加 |

### 4. 設定変更時の注意

- `.claude/settings.json` を変更した場合、次のセッションから有効
- hook スクリプトの変更は即時反映
- `.claude/rules/*.md` の globs 変更は即時反映
