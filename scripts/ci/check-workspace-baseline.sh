#!/usr/bin/env bash
# `cargo test --workspace` が常時 FAIL する集合を baseline として固定し、差分を検出する。
#
# 背景 (ISSUES.md I-11): workspace は既知の理由で多数の test が FAIL する。
# その test 名がどこにも記録されていないため「workspace GREEN」を受入条件に置いた作業が
# 判定不能で、新規 regression が既知 FAIL に埋もれる。本 script は
# docs/development/validation/workspace-expected-failures.txt を正本として、
#
#   - expected に無い FAIL (= 新規 regression)
#   - expected に有るのに pass に転じた test (= baseline の更新漏れ)
#   - expected に有るのに実測に現れない test (= rename / 削除の追随漏れ)
#
# の 3 方向すべてを非 0 で報せる。「新規 FAIL だけ見る」設計にすると baseline が
# 腐っても誰も気づかないため、片方向にはしない。
#
# 測定手段は cargo-nextest。--no-fail-fast 相当が既定で全 target を完走し、
# JUnit XML を吐けるので test 名を機械抽出でき、process-per-test なので
# 1 件の crash が binary 全体を巻き込まない。
#
# 使い方:
#   scripts/ci/check-workspace-baseline.sh              # nextest を回して判定
#   scripts/ci/check-workspace-baseline.sh --junit X    # 測定済み XML で判定のみ
#
# --junit を持たせてあるのは、契約テスト (test-check-workspace-baseline.sh) が
# 差分ロジックだけを秒単位で検証できるようにするため。実測込みでしか試せない
# 契約テストは数時間かかり、結局誰も回さなくなる。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

EXPECTED="$ROOT/docs/development/validation/workspace-expected-failures.txt"
JUNIT=""
PROFILE="baseline"

die() {
  echo "ERROR: $*" >&2
  exit 1
}

usage() {
  cat >&2 <<'USAGE'
usage: scripts/ci/check-workspace-baseline.sh [options]

  --junit <path>      nextest を回さず、この JUnit XML を実測値として使う
  --expected <path>   expected-failures の正本 (既定: docs/development/validation/workspace-expected-failures.txt)
  --profile <name>    nextest profile (既定: baseline)
  -h, --help          このヘルプ
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --junit)
      [[ $# -ge 2 ]] || die "--junit requires a path"
      JUNIT="$2"
      shift 2
      ;;
    --expected)
      [[ $# -ge 2 ]] || die "--expected requires a path"
      EXPECTED="$2"
      shift 2
      ;;
    --profile)
      [[ $# -ge 2 ]] || die "--profile requires a name"
      PROFILE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      die "unknown argument: $1"
      ;;
  esac
done

[[ -f "$EXPECTED" ]] || die "expected-failures file not found: $EXPECTED"

if [[ -z "$JUNIT" ]]; then
  command -v cargo-nextest >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1 \
    || die "cargo-nextest not found; install it or pass --junit <path>"
  echo "cargo nextest run --workspace --profile $PROFILE を実行する (数時間かかる)" >&2
  # FAIL があっても続行する。判定は下の差分で行う。
  (cd "$ROOT" && cargo nextest run --workspace --profile "$PROFILE") || true
  JUNIT="$ROOT/target/nextest/$PROFILE/junit.xml"
fi

[[ -f "$JUNIT" ]] || die "JUnit XML not found: $JUNIT"

python3 - "$JUNIT" "$EXPECTED" <<'PY'
import sys
import xml.etree.ElementTree as ET

junit_path, expected_path = sys.argv[1], sys.argv[2]

try:
    root = ET.parse(junit_path).getroot()
except ET.ParseError as exc:
    print(f"ERROR: JUnit XML を解析できない: {junit_path}: {exc}", file=sys.stderr)
    raise SystemExit(2)

# test の同一性は必ず binary で修飾する。
# 実例: `support::tests::test_support_selfhost_typeinfer_runtime_bundle_cached` は
# tests/e2e/support.rs の `mod` 共有で 5 つの binary へ重複計上される。
# test 名だけを鍵にすると 5 件が 1 件へ潰れて差分が壊れる。
actual_fail = set()
actual_all = set()
for case in root.iter("testcase"):
    binary = case.get("classname") or ""
    name = case.get("name") or ""
    ident = f"{binary} {name}".strip()
    if not ident:
        continue
    actual_all.add(ident)
    if case.find("failure") is not None or case.find("error") is not None:
        actual_fail.add(ident)

expected = set()
malformed = []
with open(expected_path, encoding="utf-8") as fh:
    for lineno, raw in enumerate(fh, 1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        # 書式は nextest の表示形式に合わせて `<binary-id> <test-name>`。
        parts = line.split()
        if len(parts) != 2:
            malformed.append((lineno, raw.rstrip("\n")))
            continue
        expected.add(line)

new_failures = sorted(actual_fail - expected)
now_passing = sorted(t for t in expected - actual_fail if t in actual_all)
vanished = sorted(t for t in expected - actual_fail if t not in actual_all)

print(f"実測: test {len(actual_all)} 件 / FAIL {len(actual_fail)} 件 (JUnit: {junit_path})")
print(f"expected-failures: {len(expected)} 件 ({expected_path})")

problems = 0

if malformed:
    problems += 1
    print("", file=sys.stderr)
    print(f"expected-failures の書式が壊れている ({len(malformed)} 行)。", file=sys.stderr)
    print("1 行は `<binary-id> <test-name>` の 2 語でなければならない:", file=sys.stderr)
    for lineno, text in malformed:
        print(f"  {expected_path}:{lineno}: {text}", file=sys.stderr)

if new_failures:
    problems += 1
    print("", file=sys.stderr)
    print(f"新規 FAIL が {len(new_failures)} 件ある (baseline に無い):", file=sys.stderr)
    for t in new_failures:
        print(f"  + {t}", file=sys.stderr)

if now_passing:
    problems += 1
    print("", file=sys.stderr)
    print(f"expected FAIL のうち {len(now_passing)} 件が pass に転じた。", file=sys.stderr)
    print("直ったのは良いことだが、baseline を更新しないと再発を検出できなくなる:", file=sys.stderr)
    for t in now_passing:
        print(f"  - {t}", file=sys.stderr)

if vanished:
    problems += 1
    print("", file=sys.stderr)
    print(f"expected FAIL のうち {len(vanished)} 件が実測に存在しない。", file=sys.stderr)
    print("rename / 削除 / filter 漏れのいずれか。baseline を追随させること:", file=sys.stderr)
    for t in vanished:
        print(f"  ? {t}", file=sys.stderr)

if problems:
    print("", file=sys.stderr)
    print(f"ERROR: workspace baseline と実測が乖離している ({problems} 種類)", file=sys.stderr)
    raise SystemExit(1)

print("OK: 実測 FAIL 集合は baseline と一致している")
PY
