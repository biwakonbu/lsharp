# Bootstrap Diff Artifact 仕様

## 概要

ブートストラップ検証テスト (`test_e2e_bootstrap_four_layer_comparison`,
`test_e2e_bootstrap_fixed_point_stage2_stage3`,
`test_e2e_bootstrap_stage2_self_feed_fixed_input_set`) が出力する
アーティファクトと保存先の仕様。比較結果は成功時も更新され、
失敗時のローカル/CI 調査でそのまま参照できる。

## アーティファクト保存先

```
ci-artifacts/bootstrap-diff/{commit_sha}/
├── stage1_a.wasm          # 1 回目コンパイルの stage1 Wasm
├── stage1_b.wasm          # 2 回目コンパイルの stage1 Wasm
├── diff-report.txt        # 不一致レイヤーの要約
├── sections_a.json        # 比較左辺のセクション構造
├── sections_b.json        # 比較右辺のセクション構造
├── export_a.bin           # stage1_a の Export セクション raw bytes
├── export_b.bin           # stage1_b の Export セクション raw bytes
├── metadata.json          # コミット SHA, タイムスタンプ, テスト名
├── fixed-input-set-self-feed-report.txt  # 54 target self-feed 要約 (optional)
└── fixed-input-set-self-feed.json         # target 別 self-feed metadata (optional)
```

## 4 層比較 (Four-Layer Comparison)

不一致が検出された場合、以下のレイヤーごとに診断情報を記録する:

| レイヤー | 対象 | 保存内容 |
|----------|------|----------|
| 1 | ハッシュフィンガープリント | 両方の Wasm バイナリ全体 |
| 2 | Export セクション | Export セクションの raw bytes |
| 3 | Data セクション | Data セクションの raw bytes (存在する場合) |
| 4 | 診断カウント | コンパイルエラーの有無 |

## diff-report.txt フォーマット

```
Bootstrap Diff Report
=====================
commit: {commit_sha}
timestamp: {ISO 8601}
test: test_e2e_bootstrap_four_layer_comparison

Layer 1 (hash):    MATCH | MISMATCH (0x... vs 0x...)
Layer 2 (export):  MATCH | MISMATCH (N bytes vs M bytes)
Layer 3 (data):    MATCH | MISMATCH | ABSENT
Layer 4 (diag):    MATCH | MISMATCH (0 vs N)

stage1_a.wasm: {size} bytes
stage1_b.wasm: {size} bytes
```

## metadata.json フォーマット

```json
{
  "commit_sha": "abc123...",
  "timestamp": "2025-01-01T00:00:00Z",
  "test_name": "test_e2e_bootstrap_four_layer_comparison",
  "stage1_a_size": 12345,
  "stage1_b_size": 12345,
  "layers": {
    "hash": "match",
    "export": "match",
    "data": "match",
    "diagnostics": "match"
  }
}
```

## fixed-input-set-self-feed-report.txt / .json

`test_e2e_bootstrap_stage2_self_feed_fixed_input_set` は同じ artifact
ディレクトリに、fixed input set 54 件（selfhost 40 / stdlib 11 /
examples 3）の stage2 self-feed 結果を追加保存する。

- `fixed-input-set-self-feed-report.txt`
  - `target_count`, `compiled_count`, `failed_count`
  - 各 target の `PASS [root] path -> wasm_bytes`
  - 失敗時は `FAIL [root] path -> first_error_line`
- `fixed-input-set-self-feed.json`
  - `stage2_self_compiler_bytes`
  - `compiled_targets[]` (`path`, `root`, `output_wasm_bytes`, `fingerprint`)
  - `failed_targets[]` (`path`, `root`, `error`)

```json
{
  "commit_sha": "abc123...",
  "test_name": "test_e2e_bootstrap_stage2_self_feed_fixed_input_set",
  "target_count": 54,
  "compiled_count": 54,
  "failed_count": 0
}
```

## CI での利用

### GitHub Actions でのアーティファクト保存

```yaml
- name: Bootstrap diff アーティファクト保存
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: bootstrap-diff-${{ github.sha }}
    path: ci-artifacts/bootstrap-diff/${{ github.sha }}/
    retention-days: 30
```

`always()` を使うのは、fixed-point テスト自体が成功した場合でも
直近の比較結果を後続調査や PR レビューで参照できるようにするため。

### ローカルでの調査

```bash
# 不一致時のバイナリ比較
diff <(xxd ci-artifacts/bootstrap-diff/{sha}/stage1_a.wasm) \
     <(xxd ci-artifacts/bootstrap-diff/{sha}/stage1_b.wasm)

# Export セクションの比較
diff <(xxd ci-artifacts/bootstrap-diff/{sha}/export_a.bin) \
     <(xxd ci-artifacts/bootstrap-diff/{sha}/export_b.bin)
```

## 関連テスト

- `test_e2e_bootstrap_four_layer_comparison` — 4 層比較
- `test_e2e_bootstrap_fixed_point_stage2_stage3` — stage2/stage3 fixed-point
- `test_e2e_bootstrap_stage2_self_feed_fixed_input_set` — fixed input set self-feed
- `test_e2e_bootstrap_stage_chain_verification` — ステージチェーン検証
- `test_e2e_bootstrap_stage0_oracle_chain_four_way_identity` — 4 連 oracle チェーン
- `test_e2e_bootstrap_stage1_deterministic` — stage1 決定性
- `test_e2e_bootstrap_selfhost_modules_deterministic` — モジュール決定性
