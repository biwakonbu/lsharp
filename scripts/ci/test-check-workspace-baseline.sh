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

# --junit を複数渡す版 (最後の引数が expected)。
run_checker_multi() {
  local args=()
  local expected="${!#}"
  local n=$(($# - 1))
  local i
  for ((i = 1; i <= n; i++)); do
    args+=(--junit "${!i}")
  done
  set +e
  "$RUNNER" "${args[@]}" --expected "$expected" >"$OUT" 2>"$ERR"
  local rc=$?
  set -e
  echo "$rc"
}

# SIGTERM で中断された run を模した JUnit を作る。
make_aborted_junit() {
  local path="$1"
  cat >"$path" <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="2">
  <testsuite name="lsharp-wasm::e2e" tests="2">
    <testcase name="test_alpha" classname="lsharp-wasm::e2e" time="0.1">
      <failure type="test failure with exit code 101">assertion failed</failure>
    </testcase>
    <testcase name="test_interrupted" classname="lsharp-wasm::e2e" time="0.1">
      <failure type="test abort" message="process aborted with signal 15 (SIGTERM)">signal</failure>
    </testcase>
  </testsuite>
</testsuites>
XML
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

# --- RED-9: --junit を複数渡すと FAIL 集合が合併される ---
# 実運用の理由: workspace 全体を 1 プロセスで回すと数時間かかり、途中で
# 中断されると全部やり直しになる。binary で分割して複数回に分けて測り、
# その結果を合併して判定できなければならない。
make_junit "$TMP_ROOT/part-a.xml" \
  'lsharp-driver::default_path_delegation|test_alpha|fail' \
  'lsharp-driver::default_path_delegation|test_beta|pass'
make_junit "$TMP_ROOT/part-b.xml" \
  'lsharp-wasm::e2e|test_gamma|fail' \
  'lsharp-wasm::e2e|test_delta|pass'

RC="$(run_checker_multi "$TMP_ROOT/part-a.xml" "$TMP_ROOT/part-b.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" == "0" ]] || fail "分割した JUnit の合併が効いていない (rc=$RC): $(cat "$ERR")"
assert_file_contains "$OUT" "baseline と一致"

# 片方だけでは足りない (合併しないと test_gamma が消失扱いになる)
RC="$(run_checker "$TMP_ROOT/part-a.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" != "0" ]] || fail "片方の JUnit だけで exit 0 になった"

# --- RED-10: 分割した JUnit が重複していたら黙って通さない ---
# 分割の切り方を間違えて同じ test を 2 回測ると、集合演算は通っても
# 「全部測った」ことの根拠にならない。重複は設定ミスとして落とす。
RC="$(run_checker_multi "$TMP_ROOT/part-a.xml" "$TMP_ROOT/part-a.xml" "$TMP_ROOT/expected-ok.txt")"
[[ "$RC" != "0" ]] || fail "同じ JUnit を 2 回渡しても exit 0 になった"
assert_file_contains "$ERR" "重複"

# --- RED-11: SIGTERM で中断された run を baseline にしてはならない ---
# 中断された nextest は残りの test を実行せず、走行中だった test を
# `<failure type="test abort" message="... signal 15 (SIGTERM)">` として記録する。
# これを黙って FAIL 集合に混ぜると、実行されなかった test が「pass」扱いになり、
# baseline が静かに壊れる。
make_aborted_junit "$TMP_ROOT/aborted.xml"
cat >"$TMP_ROOT/expected-aborted.txt" <<'TXT'
lsharp-wasm::e2e test_alpha
lsharp-wasm::e2e test_interrupted
TXT

RC="$(run_checker "$TMP_ROOT/aborted.xml" "$TMP_ROOT/expected-aborted.txt")"
[[ "$RC" != "0" ]] || fail "SIGTERM で中断された run を受け入れてしまった"
assert_file_contains "$ERR" "中断"
assert_file_contains "$ERR" "SIGTERM"

echo "PASS: scripts/ci/test-check-workspace-baseline.sh"
