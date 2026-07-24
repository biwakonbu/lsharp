# ADR: クレート別 test distribution の deterministic inventory

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `scripts/test-distribution.py`, `scripts/ci/test-test-distribution.sh`
- Related: `imp-07-test-verification-infrastructure.md`, `TODO.md` の `LEGACY-TEST-01`

## Context

L# のテストは `lsharp-wasm` の大規模 E2E に偏っているが、クレート間の偏りを継続的に確認する機械的な inventory がなかった。
目標値を導入すると既存テストの削除や形式的な数合わせを誘発するため、まず current distribution を再現可能な数値として出力する。

## Decision

- `scripts/test-distribution.py` は `crates/*/Cargo.toml` を deterministic に列挙し、Rust source の test attribute/function、`proptest!` macro、`#[ignore]` 数を
  クレート単位で集計する。
- `--json` は `schema_version`、crate entries、totals を stable key/order で出力し、既定の TSV は人間が差分・表計算へ渡せる形式とする。timestamp や
  absolute path は出力しない。
- `scripts/ci/test-test-distribution.sh` は JSON の schema、現行 8 crate の集合、非負値、合計整合、同一入力での byte-level 安定性、TSV header/total を検査する。
- 分布の目標値や pass/fail threshold はこの slice へ導入しない。inventory は観測用であり、テストの完了判定とは分離する。

## Evidence

- RED: contract test を先に追加し、未作成の `scripts/test-distribution.py` による実行失敗を確認した。
- GREEN: `scripts/ci/test-test-distribution.sh` が schema、8 crate、合計整合、JSON/TSV の安定性を検査して成功した。
- Current inventory: 164 Rust files、4,237 test attributes/functions、4 proptest macros、1,210 ignored attributes（2026-07-24 checkout）。
- Python syntax、shell contract、`git diff --check`、`scripts/audit_docs.sh` が成功した。

## Consequences

クレートごとのテスト分布を CI/local で再計測でき、E2E 偏重の推移を evidence として記録できる。一方、coverage percentage、mutation score、test quality、
GC leak/limit、rooting stress、`LEGACY-TEST-01` aggregate の完了判定はこの ADR の対象外である。
