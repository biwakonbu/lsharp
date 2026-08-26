#!/usr/bin/env python3
"""e2e test の「実行失敗が test の失敗にならない」形を列挙する。

`I-79` / `I-82` の全数調査で使ったもの。4 つの形を分けて数える。

  (b)  Ok 側に assertion があり、Err では skip される   -> assertion が「走らない」
  (c)  Result を束縛して {:?} 表示のみ                  -> assertion が「無い」
  (a') match の Err 腕が eprintln、Ok 腕にも assertion 無し
  (d)  恒真な assert! (本スクリプトでは検出しない。手で見ること)

使い方:
    python3 scripts/sweep_unchecked_result.py [--root <dir>]

**出力をそのまま件数として使わないこと。** 偽陽性が出る形が判っている:
  - タプル match (`match (out_a, out_b) { .. }`) を未検査と読む
  - helper に assertion を委譲している呼び出しを assertion 無しと読む
判定は必ず該当箇所を開いて行う。判定の正本は
docs/adr/decisions-harness-swallowed-error-arms.md。
"""
import argparse
import pathlib
import re
import sys

FAIL = ("panic!", "unreachable!", "todo!", "unimplemented!", "resume_unwind")
ASSERT = ("assert!", "assert_eq!", "assert_ne!", "panic!")
# Err 腕が失敗を「記録」しているとみなすトークン (後で assert される想定)
RECORD = (".push(", "= true", "Err(", "return", "?;", "continue", "break")
CHECKED = re.compile(
    r"\b(unwrap|expect|expect_err|unwrap_or_else|unwrap_err|is_ok|is_err|map_err|ok\(\))\b"
)


def block_end(s: str, i: int) -> int:
    """s[i] == '{' に対応する '}' の index。見つからなければ -1。"""
    depth = 0
    while i < len(s):
        if s[i] == "{":
            depth += 1
        elif s[i] == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def arm_body(s: str, j: int):
    """`=>` の直後 index j から match 腕の本文を切り出す。"""
    while j < len(s) and s[j] in " \n":
        j += 1
    if j < len(s) and s[j] == "{":
        k = block_end(s, j)
        return s[j : k + 1], k + 1
    k, depth = j, 0
    while k < len(s):
        c = s[k]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            if depth == 0:
                break
            depth -= 1
        elif c == "," and depth == 0:
            break
        k += 1
    return s[j:k], k


def line_of(s: str, i: int) -> int:
    return s.count("\n", 0, i) + 1


def sweep_skipped_assertions(root: pathlib.Path):
    """形 (b) / (a'): match と if-let-Ok。Ok 側に assertion があるものだけ。"""
    hits = []
    for p in sorted(root.rglob("*.rs")):
        s = p.read_text()

        for mm in re.finditer(r"\bmatch\b", s):
            b = s.find("{", mm.end())
            if b < 0:
                continue
            e = block_end(s, b)
            if e < 0:
                continue
            blk = s[b : e + 1]
            ok_body = err_body = None
            for am in re.finditer(r"\b(Ok|Err)\((?:_|[A-Za-z_:()]+)?\)?\s*=>\s*", blk):
                # ネストした match の腕を拾わないよう深さ 1 の腕だけ採る
                depth = blk.count("{", 0, am.start()) - blk.count("}", 0, am.start())
                if depth != 1:
                    continue
                body, _ = arm_body(blk, am.end())
                if am.group(1) == "Ok" and ok_body is None:
                    ok_body = body
                if am.group(1) == "Err" and err_body is None:
                    err_body = body
            if ok_body is None or err_body is None:
                continue
            if any(t in err_body for t in FAIL):
                continue
            if any(t in err_body for t in RECORD):
                continue
            if any(t in err_body for t in ASSERT):
                continue
            kind = "b:match" if any(t in ok_body for t in ASSERT) else "a':match"
            hits.append((str(p), line_of(s, mm.start()), kind,
                         err_body.strip().replace("\n", " ")[:60]))

        for m in re.finditer(r"\bif let Ok\(", s):
            j = s.find("{", m.end())
            if j < 0:
                continue
            k = block_end(s, j)
            if k < 0:
                continue
            if s[k + 1 : k + 30].lstrip().startswith("else"):
                continue
            if not any(t in s[j : k + 1] for t in ASSERT):
                continue
            hits.append((str(p), line_of(s, m.start()), "b:if-let",
                         s[m.start() : j].strip().replace("\n", " ")[:60]))
    return hits


def sweep_unchecked_bindings(root: pathlib.Path):
    """形 (c): Result を束縛して一度も検査しないもの。"""
    helpers = set()
    for p in root.rglob("*.rs"):
        for m in re.finditer(
            r"fn\s+([a-zA-Z0-9_]+)\s*\([^;{]*?\)\s*->\s*Result<", p.read_text(), re.S
        ):
            helpers.add(m.group(1))

    hits = []
    for p in sorted(root.rglob("*.rs")):
        s = p.read_text()
        for m in re.finditer(
            r"\blet\s+(?:mut\s+)?([a-zA-Z0-9_]+)\s*(?::[^=]+)?=\s*([a-zA-Z0-9_]+)\s*\(", s
        ):
            var, fn = m.group(1), m.group(2)
            if fn not in helpers:
                continue
            semi = s.find(";", m.end())
            if semi < 0:
                continue
            stmt = s[m.start() : semi]
            if CHECKED.search(stmt) or stmt.rstrip().endswith("?"):
                continue
            fnstart = s.rfind("\nfn ", 0, m.start())
            b = s.find("{", fnstart if fnstart > 0 else 0)
            e = block_end(s, b)
            scope = s[semi : e if e > 0 else len(s)]
            uses = list(re.finditer(r"\b" + re.escape(var) + r"\b", scope))
            checked = False
            for u in uses:
                ctx = scope[max(0, u.start() - 60) : u.end() + 60]
                if (
                    CHECKED.search(ctx)
                    or re.search(r"match\s+&?\(?[^)]*\b" + re.escape(var) + r"\b", ctx)
                    or re.search(r"if let (Ok|Err)\([^)]*\)\s*=\s*&?" + re.escape(var), ctx)
                    or "assert" in ctx
                ):
                    checked = True
                    break
            if checked:
                continue
            hits.append((str(p), line_of(s, m.start()), "c:binding", f"{fn} -> {var}"))
    return hits


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="crates/lsharp-wasm/tests/e2e")
    args = ap.parse_args()
    root = pathlib.Path(args.root)
    if not root.is_dir():
        print(f"root が無い: {root}", file=sys.stderr)
        return 2

    hits = sweep_skipped_assertions(root) + sweep_unchecked_bindings(root)
    for h in sorted(hits):
        print(f"{h[0]}:{h[1]}\t{h[2]}\t{h[3]}")

    counts = {}
    for h in hits:
        counts[h[2]] = counts.get(h[2], 0) + 1
    print("", file=sys.stderr)
    for k in sorted(counts):
        print(f"  {k}: {counts[k]} 件", file=sys.stderr)
    print(f"  合計 {len(hits)} 件 (偽陽性を含む。手で判定すること)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
