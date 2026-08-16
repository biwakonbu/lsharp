#!/usr/bin/env bash
# scripts/ci/check-workspace-baseline.sh の契約テスト。
#
# 本体は既定では `cargo nextest` を回すが、それでは契約テストが数時間かかって
# 誰も実行しなくなる。差分ロジックだけを切り離して検証できるよう、本体は
# `--junit <path>` で「測定済みの JUnit XML を読む」モードを持つ。
# ここでは合成した JUnit XML と expected リストを食わせて、判定の 4 方向
# (一致 / 新規 FAIL / 期待 FAIL の消失 / 期待 FAIL の pass 転向) を観測する。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/ci/check-workspace-baseline.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

[[ -x "$RUNNER" ]] || fail "baseline checker is missing or not executable: $RUNNER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-baseline-check.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

OUT="$TMP_ROOT/stdout.txt"
ERR="$TMP_ROOT/stderr.txt"

# JUnit XML を組み立てる。
# 引数は `<binary-id>|<test-name>|<pass|fail>` の並び。
make_junit() {
  local path="$1"
  shift
  {
    printf '<?xml version="1.0" encoding="UTF-8"?>\n'
    printf '<testsuites name="nextest-run" tests="%d">\n' "$#"
    local spec binary test status
    for spec in "$@"; do
      binary="${spec%%|*}"
      test="${spec#*|}"
      status="${test##*|}"
      test="${test%|*}"
      printf '  <testsuite name="%s" tests="1">\n' "$binary"
      printf '    <testcase name="%s" classname="%s" time="0.1">\n' "$test" "$binary"
      if [[ "$status" == "fail" ]]; then
        printf '      <failure type="test failure">assertion failed</failure>\n'
      fi
      printf '    </testcase>\n'
      printf '  </testsuite>\n'
    done
    printf '</testsuites>\n'
  } >"$path"
}

run_checker() {
  local junit="$1"
  local expected="$2"
  set +e
  "$RUNNER" --junit "$junit" --expected "$expected" >"$OUT" 2>"$ERR"
  local rc=$?
  set -e
  echo "$rc"
}

# --- RED-1: 実測 FAIL 集合と expected が一致すれば exit 0 ---
make_junit "$TMP_ROOT/base.xml" \
  'lsharp-driver::default_path_delegation|test_alpha|fail' \
  'lsharp-driver::default_path_delegation|test_beta|pass' \
  'lsharp-wasm::e2e|test_gamma|fail'

cat >"$TMP_ROOT/expected-ok.txt" <<'TXT'
# コメント行は無視される

lsharp-driver::default_path_delegation test_alpha  # cluster: default_path_delegation
lsharp-wasm::e2e test_gamma  # cluster: snapshot
TXT

RC="$(run_checker "$TMP_ROOT/base.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" == "0" ]] || fail "一致しているのに non-zero で落ちた (rc=$RC): $(cat "$ERR")"
assert_file_contains "$OUT" "baseline と一致"

# --- RED-2: expected に実在しない test 名を仕込むと non-zero ---
# (計画の受入条件そのもの。baseline の更新漏れ / rename を検出する)
cat >"$TMP_ROOT/expected-ghost.txt" <<'TXT'
lsharp-driver::default_path_delegation test_alpha
lsharp-wasm::e2e test_gamma
lsharp-wasm::e2e test_this_test_does_not_exist
TXT

RC="$(run_checker "$TMP_ROOT/base.xml" "$TMP_ROOT/expected-ghost.txt")"
[[ "$RC" != "0" ]] || fail "存在しない test 名を仕込んでも exit 0 になった"
assert_file_contains "$ERR" "test_this_test_does_not_exist"
assert_file_contains "$ERR" "存在しない"

# --- RED-3: expected FAIL が pass に転じたら non-zero ---
make_junit "$TMP_ROOT/fixed.xml" \
  'lsharp-driver::default_path_delegation|test_alpha|pass' \
  'lsharp-wasm::e2e|test_gamma|fail'

RC="$(run_checker "$TMP_ROOT/fixed.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" != "0" ]] || fail "期待 FAIL が pass に転じても exit 0 になった"
assert_file_contains "$ERR" "test_alpha"
assert_file_contains "$ERR" "pass に転じた"

# --- RED-4: expected に無い FAIL (= 新規 regression) は non-zero ---
make_junit "$TMP_ROOT/regressed.xml" \
  'lsharp-driver::default_path_delegation|test_alpha|fail' \
  'lsharp-wasm::e2e|test_gamma|fail' \
  'lsharp-types::infer|test_new_regression|fail'

RC="$(run_checker "$TMP_ROOT/regressed.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" != "0" ]] || fail "新規 FAIL があっても exit 0 になった"
assert_file_contains "$ERR" "test_new_regression"
assert_file_contains "$ERR" "新規"

# --- RED-5: 同名 test が別 binary に居ても混同しない ---
# 実在する事例: `support::tests::test_support_selfhost_typeinfer_runtime_bundle_cached` は
# tests/e2e/support.rs の `mod` 共有により 5 つの binary へ重複計上される。
# test 名だけで集合を作ると 5 件が 1 件に潰れて差分が壊れる。
make_junit "$TMP_ROOT/dup.xml" \
  'lsharp-wasm::e2e_a|support::tests::test_shared|fail' \
  'lsharp-wasm::e2e_b|support::tests::test_shared|pass'

cat >"$TMP_ROOT/expected-dup.txt" <<'TXT'
lsharp-wasm::e2e_a support::tests::test_shared
TXT

RC="$(run_checker "$TMP_ROOT/dup.xml" "$TMP_ROOT/expected-dup.txt")"
[[ "$RC" == "0" ]] || fail "binary 修飾が効いていない (rc=$RC): $(cat "$ERR")"

# 逆向き: 落ちている方ではなく通っている方を expected に書いたら落ちること
cat >"$TMP_ROOT/expected-dup-wrong.txt" <<'TXT'
lsharp-wasm::e2e_b support::tests::test_shared
TXT

RC="$(run_checker "$TMP_ROOT/dup.xml" "$TMP_ROOT/expected-dup-wrong.txt")"
[[ "$RC" != "0" ]] || fail "binary を取り違えても exit 0 になった"

# --- RED-6: JUnit が読めなければ黙って通さない ---
RC="$(run_checker "$TMP_ROOT/does-not-exist.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" != "0" ]] || fail "JUnit XML が無いのに exit 0 になった"

# --- RED-7: expected リストが空でも、FAIL が 0 件なら通る ---
make_junit "$TMP_ROOT/allgreen.xml" \
  'lsharp-types::infer|test_ok|pass'
: >"$TMP_ROOT/expected-empty.txt"

RC="$(run_checker "$TMP_ROOT/allgreen.xml" "$TMP_ROOT/expected-empty.txt")"
[[ "$RC" == "0" ]] || fail "FAIL 0 件 / expected 0 件で落ちた (rc=$RC): $(cat "$ERR")"

# --- RED-8: 追跡対象の expected ファイルが実在し、書式が壊れていないこと ---
TRACKED="$ROOT/docs/development/validation/workspace-expected-failures.txt"
[[ -f "$TRACKED" ]] || fail "expected-failures の正本が無い: $TRACKED"
RC="$(run_checker "$TMP_ROOT/base.xml" "$TRACKED")"
# 中身は合致しないので non-zero で良いが、書式エラーで死んではいけない
assert_file_contains "$ERR" "実測"

echo "PASS: scripts/ci/test-check-workspace-baseline.sh"
