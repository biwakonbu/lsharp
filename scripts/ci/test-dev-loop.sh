#!/usr/bin/env bash
# scripts/dev-loop.sh の契約テスト。
# fake compiler を注入して、再生成が起きる条件と起きない条件を観測する。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/dev-loop.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$1', got '$2'"
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

assert_file_not_contains() {
  local path="$1"
  local unexpected="$2"
  ! grep -F -- "$unexpected" "$path" >/dev/null || fail "$path unexpectedly contains: $unexpected"
}

log_lines() {
  if [[ -f "$LOG_FILE" ]]; then
    grep -c . "$LOG_FILE" || true
  else
    echo 0
  fi
}

[[ -x "$RUNNER" ]] || fail "dev loop runner is missing or not executable: $RUNNER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-dev-loop.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

TEST_ROOT="$TMP_ROOT/repo"
LOG_FILE="$TMP_ROOT/invocations.log"
FAKE_COMPILER="$TMP_ROOT/fake-lsharp"

mkdir -p "$TEST_ROOT/scripts" "$TEST_ROOT/selfhost/src/App" "$TEST_ROOT/target/debug"
cp "$RUNNER" "$TEST_ROOT/scripts/dev-loop.sh"
chmod +x "$TEST_ROOT/scripts/dev-loop.sh"

cat >"$TEST_ROOT/selfhost/src/App/EmbeddedCli.ls" <<'LS'
(module App.EmbeddedCli)
(defn main [] 0)
LS

cat >"$TEST_ROOT/selfhost/src/App/Other.ls" <<'LS'
(module App.Other)
(defn helper [] 1)
LS

# fake compiler: 引数を log に残し、-o の指す先へ dummy component を書く。
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compile|%s\n' "$*" >>"$LSHARP_DEV_TEST_LOG"
out=""
prev=""
for arg in "$@"; do
  if [[ "$prev" == "-o" ]]; then
    out="$arg"
  fi
  prev="$arg"
done
[[ -n "$out" ]] || exit 90
mkdir -p "$(dirname "$out")"
printf '\0asm\1\0\0\0fake-component' >"$out"
SH
chmod +x "$FAKE_COMPILER"

export LSHARP_DEV_TEST_LOG="$LOG_FILE"
export LSHARP_DEV_COMPILER="$FAKE_COMPILER"

run_dev_loop() {
  (cd "$TEST_ROOT" && ./scripts/dev-loop.sh "$@") >"$TMP_ROOT/stdout.txt" 2>"$TMP_ROOT/stderr.txt"
}

# --- RED-1: 初回は再生成し、fingerprint 一致の 2 回目は再生成しない ---
run_dev_loop || fail "first dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_eq "1" "$(log_lines)"

run_dev_loop || fail "second dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_eq "1" "$(log_lines)"

# --- RED-3: 生成先が target/debug 配下でないこと ---
COMPONENT="$TEST_ROOT/.lsharp-dev/bin/lsharp.component.wasm"
[[ -f "$COMPONENT" ]] || fail "sidecar component was not generated at $COMPONENT"
[[ -f "$TEST_ROOT/.lsharp-dev/bin/lsharp" ]] || fail "driver binary was not copied into .lsharp-dev/bin"
[[ ! -e "$TEST_ROOT/target/debug/lsharp.component.wasm" ]] \
  || fail "sidecar must not be placed next to target/debug/lsharp"
assert_file_contains "$LOG_FILE" ".lsharp-dev/bin/lsharp.component.wasm"
assert_file_not_contains "$LOG_FILE" "target/debug/lsharp.component.wasm"

# --- RED-2: selfhost/src を 1 ファイル変えると再生成がちょうど 1 回走る ---
printf '(defn extra [] 2)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
run_dev_loop || fail "third dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_eq "2" "$(log_lines)"

run_dev_loop || fail "fourth dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_eq "2" "$(log_lines)"

# --- RED-4: compiler binary が新しくなったら fingerprint 一致でも binary を再コピーする ---
printf '\n# touched\n' >>"$FAKE_COMPILER"
touch "$FAKE_COMPILER"
COPIED_BEFORE="$(wc -c <"$TEST_ROOT/.lsharp-dev/bin/lsharp" | tr -d ' ')"
run_dev_loop || fail "fifth dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
COPIED_AFTER="$(wc -c <"$TEST_ROOT/.lsharp-dev/bin/lsharp" | tr -d ' ')"
[[ "$COPIED_BEFORE" != "$COPIED_AFTER" ]] \
  || fail "stale driver binary was not refreshed after the compiler changed"
# binary 更新は component 再生成を伴わない (L# source は変わっていない)
assert_eq "2" "$(log_lines)"

# --- RED-5: 再生成時に embedded component への委譲を止めていること ---
# fake compiler の実行環境に LSHARP_DISABLE_EMBEDDED_COMPONENT が渡っていることを確認する。
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compile|disable=%s|%s\n' "${LSHARP_DISABLE_EMBEDDED_COMPONENT:-unset}" "$*" \
  >>"$LSHARP_DEV_TEST_LOG"
out=""
prev=""
for arg in "$@"; do
  if [[ "$prev" == "-o" ]]; then
    out="$arg"
  fi
  prev="$arg"
done
[[ -n "$out" ]] || exit 90
mkdir -p "$(dirname "$out")"
printf '\0asm\1\0\0\0fake-component-2' >"$out"
SH
chmod +x "$FAKE_COMPILER"
printf '(defn extra2 [] 3)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
run_dev_loop || fail "sixth dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_file_contains "$LOG_FILE" "disable=1"

# --- RED-6: compile が失敗したら fingerprint を更新せず非ゼロで終わる ---
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
printf 'compile|failing\n' >>"$LSHARP_DEV_TEST_LOG"
exit 7
SH
chmod +x "$FAKE_COMPILER"
printf '(defn extra3 [] 4)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
if run_dev_loop; then
  fail "dev-loop must fail when the compiler fails"
fi
BEFORE_RETRY="$(log_lines)"
# fingerprint が更新されていなければ、次の実行でもう一度 compile を試みる
if run_dev_loop; then
  fail "dev-loop must keep failing while the compiler fails"
fi
AFTER_RETRY="$(log_lines)"
[[ "$AFTER_RETRY" -gt "$BEFORE_RETRY" ]] \
  || fail "fingerprint must not be updated after a failed compile"

# --- RED-7: compiler が entry file を書き換えても source tree を汚さない ---
# 実測: `lsharp compile` は入力 file を in-place で整形して書き戻す。dev-loop は
# selfhost/src を毎回 dirty にしてはならない (build.rs の rerun-if-changed も無効化される)。
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compile|mutating|%s\n' "$*" >>"$LSHARP_DEV_TEST_LOG"
entry="$2"
printf '\n;; compiler が書き戻した行\n' >>"$entry"
out=""
prev=""
for arg in "$@"; do
  if [[ "$prev" == "-o" ]]; then
    out="$arg"
  fi
  prev="$arg"
done
[[ -n "$out" ]] || exit 90
mkdir -p "$(dirname "$out")"
printf '\0asm\1\0\0\0fake-component-3' >"$out"
SH
chmod +x "$FAKE_COMPILER"

ENTRY_FILE="$TEST_ROOT/selfhost/src/App/EmbeddedCli.ls"
printf '(defn extra4 [] 5)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
ENTRY_BEFORE="$(shasum -a 256 "$ENTRY_FILE" | awk '{print $1}')"
run_dev_loop || fail "seventh dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
ENTRY_AFTER="$(shasum -a 256 "$ENTRY_FILE" | awk '{print $1}')"
assert_eq "$ENTRY_BEFORE" "$ENTRY_AFTER"
assert_file_contains "$TMP_ROOT/stderr.txt" "entry file を書き換えました"

# 復元した内容が fingerprint の基準と一致するので、次の実行は skip する
LINES_AFTER_RESTORE="$(log_lines)"
run_dev_loop || fail "eighth dev-loop run failed: $(cat "$TMP_ROOT/stderr.txt")"
assert_eq "$LINES_AFTER_RESTORE" "$(log_lines)"

# --- RED-8: entry 以外まで書き換えられたら fail-closed ---
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compile|mutating-other\n' >>"$LSHARP_DEV_TEST_LOG"
printf '\n;; 想定外の書き戻し\n' >>"$(dirname "$2")/Other.ls"
out=""
prev=""
for arg in "$@"; do
  if [[ "$prev" == "-o" ]]; then
    out="$arg"
  fi
  prev="$arg"
done
[[ -n "$out" ]] || exit 90
mkdir -p "$(dirname "$out")"
printf '\0asm\1\0\0\0fake-component-4' >"$out"
SH
chmod +x "$FAKE_COMPILER"
printf '(defn extra5 [] 6)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
if run_dev_loop; then
  fail "dev-loop must fail when the compiler mutates sources beyond the entry file"
fi
assert_file_contains "$TMP_ROOT/stderr.txt" "selfhost/src が予期せず変更されました"

# --- RED-9: compile が失敗したときも entry の書き戻しを復元する ---
# `lsharp compile` は整形を「コンパイル前」に書き戻す (prepare_source_for_compile)。
# つまり「整形差分あり + 型エラー」という編集中に最も起きやすい組み合わせでは、
# entry が書き換えられた直後に compile が失敗する。復元より先に die してはならない。
cat >"$FAKE_COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compile|mutating-then-failing\n' >>"$LSHARP_DEV_TEST_LOG"
printf '\n;; compile 前の整形書き戻し\n' >>"$2"
exit 7
SH
chmod +x "$FAKE_COMPILER"
printf '(defn extra6 [] 7)\n' >>"$TEST_ROOT/selfhost/src/App/Other.ls"
ENTRY_BEFORE_FAILURE="$(shasum -a 256 "$ENTRY_FILE" | awk '{print $1}')"
if run_dev_loop; then
  fail "dev-loop must fail when the compiler fails after mutating the entry file"
fi
assert_eq "$ENTRY_BEFORE_FAILURE" "$(shasum -a 256 "$ENTRY_FILE" | awk '{print $1}')"
[[ ! -e "$TEST_ROOT/.lsharp-dev/.entry-backup" ]] \
  || fail "dev-loop must not leave .entry-backup behind after a failed compile"

# fingerprint を記録していないので、次の実行でもう一度 compile を試みる
LINES_BEFORE_RETRY9="$(log_lines)"
if run_dev_loop; then
  fail "dev-loop must keep failing while the compiler fails"
fi
[[ "$(log_lines)" -gt "$LINES_BEFORE_RETRY9" ]] \
  || fail "fingerprint must not be updated after a failed compile that mutated the entry"

echo "PASS: scripts/ci/test-dev-loop.sh"
