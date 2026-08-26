#!/usr/bin/env python3
"""`--ignored` lane の実測ログを期待 FAIL 台帳と突き合わせる (cargo 非依存)。

台帳 `docs/development/validation/ignored-lane-expected-failures.txt` は
`lsharp-wasm::e2e <module>::<test>  # 注記` の形で「落ちることが分かっている test」を持つ。
このスクリプトは lane を回した生ログを読み、台帳との差分を 4 種に分けて出す。

  新規 FAIL   実測で落ちたが台帳に無い          -> 台帳へ追記するか、回帰として直す
  解消        台帳にあるが実測で pass した      -> 台帳から落とす
  未出現      台帳にあるが結果行そのものが無い  -> 完走していない / ログを渡し忘れている
  台帳外      台帳に無い test の結果行          -> 台帳取得時から test が増えている

**test 名は `module::test` で照合する。** 結果行から剥がすのは `e2e::` だけで、module 名は
残す。binary の識別は台帳の `lsharp-wasm::e2e ` prefix が担う。module 名を持たない台帳行は
移行漏れ (操作ミス) として exit 2 で落とす -- 黙って「未出現」に混ぜると、完走していないのか
移行し損ねたのかが区別できなくなる。

**完走判定は「宣言数 (`running N tests`) == 結果行のユニーク数」を*ログごとに*行う。** Summary 行の
passed + failed では足りない。中断すると Summary 自体が出ないか、部分集計になる。
重複行が出たら同じログに 2 回分の run が混ざっているので、集計してはいけない。
ログを複数渡した場合、宣言数は和を取り、**ログ間で同じ `module::test` が出たら**
同じ module を 2 回渡しているのでエラーにする。

この規則の副次効果として、**「全 module のログが揃っていること」が検査になる。**
module X の台帳エントリがあるのに X を覆うログが無ければ、そのエントリは「未出現」で非 0 になる。

使い方:
    python3 scripts/compare_ignored_lane.py <lane.log> [<lane.log> ...]
    python3 scripts/compare_ignored_lane.py <lane.log> --ledger <別の台帳>

判断の正本は docs/adr/decisions-ignored-lane-ledger-scope.md。
lane 自体の回し方は AGENTS.md を見よ。全量は 12 時間規模なので切り離して回すこと。
"""

import argparse
import collections
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = REPO / "docs/development/validation/ignored-lane-expected-failures.txt"
BINARY_PREFIX = "e2e::"
LEDGER_PREFIX = "lsharp-wasm::e2e "

EXIT_OK = 0
EXIT_DIVERGED = 1
EXIT_LEDGER_MALFORMED = 2


def load_ledger(path):
    """台帳を {`module::test`: 注記} に読む。注記は差分表示でそのまま出す。

    module 名を持たない行は移行漏れなので、名前を集めて呼び出し側に返す。
    """
    entries = {}
    bare = []
    for line in path.read_text().splitlines():
        if not line.startswith(LEDGER_PREFIX):
            continue
        name, _, note = line[len(LEDGER_PREFIX):].partition("  #")
        name = name.strip()
        if not name:
            continue
        if "::" not in name:
            bare.append(name)
            continue
        entries[name] = note.strip()
    return entries, bare


def load_run(path):
    """1 ログを読む。返すのは (宣言数, {`module::test`: 結果}, 重複 Counter, Summary)。"""
    text = path.read_text(errors="replace")
    # cargo は test が 1 件のとき `running 1 tests` ではなく `running 1 test` と書く。
    # 単数形を取りこぼすと宣言数が None になり、その module だけ完走判定が無言で消える。
    m = re.search(r"^running (\d+) tests?$", text, re.MULTILINE)
    declared = int(m.group(1)) if m else None

    results = {}
    seen = collections.Counter()
    for line in text.splitlines():
        mm = re.match(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", line)
        if not mm:
            continue
        name = mm.group(1)
        # 剥がすのは binary 名だけ。module 名は照合キーの一部として残す。
        if name.startswith(BINARY_PREFIX):
            name = name[len(BINARY_PREFIX):]
        seen[name] += 1
        results[name] = mm.group(2)

    summary = re.search(
        r"^test result: \w+\. (\d+) passed; (\d+) failed;.*finished in ([\d.]+)s",
        text, re.MULTILINE)
    return declared, results, seen, summary


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("logs", nargs="+", help="lane の生ログ (module 分割なら複数渡す)")
    ap.add_argument("--ledger", default=str(DEFAULT_LEDGER), help="期待 FAIL 台帳")
    args = ap.parse_args()

    ledger, bare = load_ledger(pathlib.Path(args.ledger))
    if bare:
        print(f"台帳     : {args.ledger}")
        print(f"エラー   : module 名を持たない台帳行が {len(bare)} 件ある。")
        print("           `lsharp-wasm::e2e <module>::<test>` の形へ移行すること")
        print("           (正本: docs/adr/decisions-ignored-lane-ledger-scope.md)")
        for n in bare[:10]:
            print(f"  ! {n}")
        return EXIT_LEDGER_MALFORMED

    declared_total = 0
    results = {}
    owner = {}            # `module::test` -> 最初に出したログ
    cross_dup = []        # ログ間で重複した名前
    incomplete = []       # ログ単位の完走判定に落ちたログ
    per_log = []

    for log in args.logs:
        path = pathlib.Path(log)
        declared, res, seen, summary = load_run(path)
        repeated = sorted(n for n, c in seen.items() if c > 1)
        complete = declared is not None and declared == len(res) and not repeated
        if declared is not None:
            declared_total += declared
        if not complete:
            incomplete.append((log, declared, len(res), repeated))
        for name, status in res.items():
            if name in owner:
                cross_dup.append((name, owner[name], log))
            else:
                owner[name] = log
            results[name] = status
        per_log.append((log, declared, len(res), len(repeated), complete, summary))

    print(f"台帳     : {len(ledger)} 件  ({args.ledger})")
    print(f"ログ     : {len(args.logs)} 本")
    for log, declared, got, repeated, complete, summary in per_log:
        tail = ""
        if summary:
            tail = (f"  [{summary.group(1)} passed / {summary.group(2)} failed"
                    f" / {summary.group(3)}s]")
        else:
            tail = "  [test result 行なし]"
        print(f"  {'OK' if complete else 'NG'} {pathlib.Path(log).name}: "
              f"宣言 {declared} / 結果行 {got} / 重複 {repeated}{tail}")
    print(f"宣言数   : {declared_total}")
    print(f"結果行   : {len(results)} 件 (ユニーク)")
    print(f"重複行   : {len(cross_dup)} 件 (ログ間)"
          + (f"  -> {[n for n, _, _ in cross_dup[:5]]}" if cross_dup else ""))
    for name, first, second in cross_dup[:10]:
        print(f"  = {name}  ({pathlib.Path(first).name} と {pathlib.Path(second).name})")

    complete = not incomplete and not cross_dup
    print(f"完走判定 : {'OK' if complete else 'NG'}"
          f"  (ログごとに 宣言数 == 結果行ユニーク数 かつ 重複 0、ログ間の重複も 0)")
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
    return EXIT_OK if (complete and not diverged) else EXIT_DIVERGED


if __name__ == "__main__":
    sys.exit(main())
