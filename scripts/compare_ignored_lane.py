#!/usr/bin/env python3
"""`--ignored` lane の実測ログを期待 FAIL 台帳と突き合わせる (cargo 非依存)。

台帳 `docs/development/validation/ignored-lane-expected-failures.txt` は
`lsharp-wasm::e2e <test-name>  # 注記` の形で「落ちることが分かっている test」を持つ。
このスクリプトは lane を回した生ログを読み、台帳との差分を 4 種に分けて出す。

  新規 FAIL   実測で落ちたが台帳に無い          -> 台帳へ追記するか、回帰として直す
  解消        台帳にあるが実測で pass した      -> 台帳から落とす
  未出現      台帳にあるが結果行そのものが無い  -> 完走していない疑い
  台帳外      台帳に無い test の結果行          -> 台帳取得時から test が増えている

**完走判定は「宣言数 (`running N tests`) == 結果行のユニーク数」で行う。** Summary 行の
passed + failed では足りない。中断すると Summary 自体が出ないか、部分集計になる。
重複行が出たら同じログに 2 回分の run が混ざっているので、集計してはいけない。

使い方:
    python3 scripts/compare_ignored_lane.py <lane.log>
    python3 scripts/compare_ignored_lane.py <lane.log> --ledger <別の台帳>

lane 自体の回し方は AGENTS.md を見よ。数時間かかるので切り離して回すこと。
"""

import argparse
import collections
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = REPO / "docs/development/validation/ignored-lane-expected-failures.txt"
MODULE_PREFIX = "e2e::selfhost_native_stage_chain::"
LEDGER_PREFIX = "lsharp-wasm::e2e "


def load_ledger(path):
    """台帳を {test 名: 注記} に読む。注記は差分表示でそのまま出す。"""
    entries = {}
    for line in path.read_text().splitlines():
        if not line.startswith(LEDGER_PREFIX):
            continue
        name, _, note = line[len(LEDGER_PREFIX):].partition("  #")
        entries[name.strip()] = note.strip()
    return entries


def load_run(path):
    text = path.read_text(errors="replace")
    m = re.search(r"^running (\d+) tests", text, re.MULTILINE)
    declared = int(m.group(1)) if m else None

    results = {}
    seen = collections.Counter()
    for line in text.splitlines():
        mm = re.match(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", line)
        if not mm:
            continue
        name = mm.group(1)
        if name.startswith(MODULE_PREFIX):
            name = name[len(MODULE_PREFIX):]
        seen[name] += 1
        results[name] = mm.group(2)

    summary = re.search(
        r"^test result: \w+\. (\d+) passed; (\d+) failed;.*finished in ([\d.]+)s",
        text, re.MULTILINE)
    return declared, results, seen, summary


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("log", help="lane の生ログ")
    ap.add_argument("--ledger", default=str(DEFAULT_LEDGER), help="期待 FAIL 台帳")
    args = ap.parse_args()

    ledger = load_ledger(pathlib.Path(args.ledger))
    declared, results, seen, summary = load_run(pathlib.Path(args.log))

    print(f"台帳     : {len(ledger)} 件  ({args.ledger})")
    print(f"宣言数   : {declared}")
    print(f"結果行   : {len(results)} 件 (ユニーク)")
    repeated = sorted(n for n, c in seen.items() if c > 1)
    print(f"重複行   : {len(repeated)} 件" + (f"  -> {repeated[:5]}" if repeated else ""))
    if summary:
        print(f"Summary  : {summary.group(1)} passed / {summary.group(2)} failed / {summary.group(3)}s")
    else:
        print("Summary  : (test result 行が無い -- 未完走)")

    complete = declared is not None and declared == len(results) and not repeated
    print(f"完走判定 : {'OK' if complete else 'NG'}"
          f"  (宣言数 == 結果行ユニーク数 かつ 重複 0)")
    print()

    failed = {n for n, r in results.items() if r == "FAILED"}
    new_fail = sorted(failed - set(ledger))
    resolved = sorted(n for n in ledger if results.get(n) == "ok")
    missing = sorted(n for n in ledger if n not in results)
    extra = sorted(n for n in results if n not in ledger)

    print(f"新規 FAIL : {len(new_fail)} 件")
    for n in new_fail:
        print(f"  + {n}")
    print(f"解消      : {len(resolved)} 件")
    for n in resolved:
        print(f"  - {n}\n        [台帳注記] {ledger[n]}")
    print(f"未出現    : {len(missing)} 件")
    for n in missing:
        print(f"  ? {n}")
    print(f"台帳外    : {len(extra)} 件 (うち FAILED {sum(1 for n in extra if results[n] == 'FAILED')} 件)")
    for n in extra:
        print(f"  * {n}  -> {results[n]}")
    print()
    print(f"実測 FAIL 総数: {len(failed)}")

    # 完走していない、または台帳と食い違うなら非 0。
    # 「台帳外」は台帳取得時から test が増えただけのことがあるので、それ自体は失敗にしない。
    diverged = bool(new_fail or resolved or missing)
    if not complete:
        print("\n判定: NG -- 完走していない")
    elif diverged:
        print("\n判定: NG -- 台帳と実測が食い違う (台帳を更新するか回帰を直す)")
    else:
        print("\n判定: OK -- 完走し、台帳と一致した")
    return 0 if (complete and not diverged) else 1


if __name__ == "__main__":
    sys.exit(main())
