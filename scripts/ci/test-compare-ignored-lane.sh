#!/usr/bin/env bash
# scripts/compare_ignored_lane.py の契約テスト (cargo 非依存)。
#
# lane 本体は 12 時間規模なので、突合ロジックだけを合成ログで検証する。
# 判断の正本は docs/adr/decisions-ignored-lane-ledger-scope.md。
#
# 検査する契約:
#   1. test 名は `module::test` で照合する (`e2e::` だけを剥がす)
#   2. module 名を持たない台帳行は操作ミスとして非 0 (移行漏れを黙って未出現に混ぜない)
#   3. 複数ログを受け取り、宣言数は和、差分は和集合に対して出す
#   4. 完走判定はログごと (自分の `running N` == 自分の結果行ユニーク数、重複 0)
#   5. ログ間で同じ `module::test` が重複したら非 0
#   6. 台帳にあるが、その module を覆うログが無ければ「未出現」で非 0
#   7. 従来どおり 新規 FAIL / 解消 でも非 0、「台帳外」だけなら 0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/compare_ignored_lane.py"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[[ -f "$RUNNER" ]] || fail "comparator is missing: $RUNNER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-ignored-lane.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

OUT="$TMP_ROOT/stdout.txt"

# 合成ログを作る。引数は `<module>|<test>|<ok|FAILED>` の並び。
# 宣言数は結果行の数に合わせる (ズレを作りたいときは make_log_declared を使う)。
make_log() {
  local path="$1"
  shift
  make_log_declared "$path" "$#" "$@"
}

make_log_declared() {
  local path="$1"
  local declared="$2"
  shift 2
  {
    printf '\nrunning %d tests\n' "$declared"
    local spec module test status
    for spec in "$@"; do
      module="${spec%%|*}"
      test="${spec#*|}"
      status="${test##*|}"
      test="${test%|*}"
      printf 'test e2e::%s::%s ... %s\n' "$module" "$test" "$status"
    done
    printf '\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n'
  } > "$path"
}

# 台帳を作る。引数は台帳の test 名 (module 付き) の並び。
make_ledger() {
  local path="$1"
  shift
  {
    printf '# 合成台帳\n'
    local name
    for name in "$@"; do
      printf 'lsharp-wasm::e2e %s  # 合成注記\n' "$name"
    done
  } > "$path"
}

run_cmp() {
  local rc=0
  python3 "$RUNNER" "$@" > "$OUT" 2>&1 || rc=$?
  echo "$rc"
}

expect_rc() {
  local want="$1" got="$2" label="$3"
  [[ "$got" == "$want" ]] || {
    cat "$OUT" >&2
    fail "$label: exit $got (want $want)"
  }
}

expect_out() {
  grep -F -- "$1" "$OUT" >/dev/null || {
    cat "$OUT" >&2
    fail "$2: 出力に '$1' が無い"
  }
}

# --- 1. module 付きで一致すれば OK -------------------------------------------
make_ledger "$TMP_ROOT/ledger1.txt" "modA::t_red"
make_log "$TMP_ROOT/a.log" "modA|t_red|FAILED" "modA|t_green|ok"
rc="$(run_cmp "$TMP_ROOT/a.log" --ledger "$TMP_ROOT/ledger1.txt")"
expect_rc 0 "$rc" "case1 module 付き一致"
expect_out "判定: OK" "case1"

# --- 2. 別 module の同名 test を取り違えない ---------------------------------
# 台帳は modA::same だけを期待。modB::same が落ちたら「新規 FAIL」で非 0。
make_ledger "$TMP_ROOT/ledger2.txt" "modA::same"
make_log "$TMP_ROOT/b.log" "modA|same|FAILED" "modB|same|FAILED"
rc="$(run_cmp "$TMP_ROOT/b.log" --ledger "$TMP_ROOT/ledger2.txt")"
expect_rc 1 "$rc" "case2 同名 test の取り違え"
expect_out "modB::same" "case2"

# --- 3. module 名を持たない台帳行は操作ミス ----------------------------------
make_ledger "$TMP_ROOT/ledger3.txt" "t_bare"
make_log "$TMP_ROOT/c.log" "modA|t_bare|FAILED"
rc="$(run_cmp "$TMP_ROOT/c.log" --ledger "$TMP_ROOT/ledger3.txt")"
expect_rc 2 "$rc" "case3 module 名なし台帳行"
expect_out "module 名" "case3"

# --- 4. 複数ログ: 宣言数は和、差分は和集合 -----------------------------------
make_ledger "$TMP_ROOT/ledger4.txt" "modA::t_red" "modB::t_red"
make_log "$TMP_ROOT/d1.log" "modA|t_red|FAILED" "modA|t_ok|ok"
make_log "$TMP_ROOT/d2.log" "modB|t_red|FAILED"
rc="$(run_cmp "$TMP_ROOT/d1.log" "$TMP_ROOT/d2.log" --ledger "$TMP_ROOT/ledger4.txt")"
expect_rc 0 "$rc" "case4 複数ログ合流"
expect_out "宣言数   : 3" "case4 宣言数の和"

# --- 5. 完走判定はログごと ---------------------------------------------------
# d2 の宣言を 5 に膨らませる。和では気付けても、ログ単位なら NG になる。
make_log_declared "$TMP_ROOT/e2.log" 5 "modB|t_red|FAILED"
rc="$(run_cmp "$TMP_ROOT/d1.log" "$TMP_ROOT/e2.log" --ledger "$TMP_ROOT/ledger4.txt")"
expect_rc 1 "$rc" "case5 ログ単位の完走判定"
expect_out "完走していない" "case5"

# --- 6. ログ間の重複は非 0 ---------------------------------------------------
make_ledger "$TMP_ROOT/ledger6.txt" "modA::t_red"
make_log "$TMP_ROOT/f1.log" "modA|t_red|FAILED"
make_log "$TMP_ROOT/f2.log" "modA|t_red|FAILED"
rc="$(run_cmp "$TMP_ROOT/f1.log" "$TMP_ROOT/f2.log" --ledger "$TMP_ROOT/ledger6.txt")"
expect_rc 1 "$rc" "case6 ログ間の重複"
expect_out "重複" "case6"

# --- 7. 台帳の module を覆うログが無ければ未出現 -----------------------------
# modB のログを渡し忘れた形。「ログを 18 本揃える」を検査に変える不変条件。
make_ledger "$TMP_ROOT/ledger7.txt" "modA::t_red" "modB::t_red"
rc="$(run_cmp "$TMP_ROOT/d1.log" --ledger "$TMP_ROOT/ledger7.txt")"
expect_rc 1 "$rc" "case7 module を覆うログが無い"
expect_out "未出現    : 1 件" "case7"

# --- 8. 解消 (台帳にあるが pass した) は非 0 ---------------------------------
make_ledger "$TMP_ROOT/ledger8.txt" "modA::t_red"
make_log "$TMP_ROOT/g.log" "modA|t_red|ok"
rc="$(run_cmp "$TMP_ROOT/g.log" --ledger "$TMP_ROOT/ledger8.txt")"
expect_rc 1 "$rc" "case8 解消"
expect_out "解消      : 1 件" "case8"

# --- 9. 台帳外だけなら 0 (test が増えただけ) ---------------------------------
make_ledger "$TMP_ROOT/ledger9.txt" "modA::t_red"
make_log "$TMP_ROOT/h.log" "modA|t_red|FAILED" "modA|t_new|ok"
rc="$(run_cmp "$TMP_ROOT/h.log" --ledger "$TMP_ROOT/ledger9.txt")"
expect_rc 0 "$rc" "case9 台帳外のみ"
expect_out "台帳外    : 1 件" "case9"

# --- 10. 同一ログ内の重複は従来どおり NG -------------------------------------
make_ledger "$TMP_ROOT/ledger10.txt" "modA::t_red"
make_log "$TMP_ROOT/i.log" "modA|t_red|FAILED" "modA|t_red|FAILED"
rc="$(run_cmp "$TMP_ROOT/i.log" --ledger "$TMP_ROOT/ledger10.txt")"
expect_rc 1 "$rc" "case10 同一ログ内の重複"
expect_out "完走していない" "case10"

echo "PASS: compare_ignored_lane.py の契約 10 件"
