#!/usr/bin/env python3
"""#[test] 関数のうち、最後の top-level 文が無条件 panic! / unreachable! のものを列挙する。

そうした test は入力が何であれ必ず赤になる。診断ダンプとして書かれた足場が
そのまま checked in されている形で、台帳に恒久的な赤を 1 件積む。

使い方:
    python3 scripts/sweep_always_failing_tests.py [--root <dir>]

**出力をそのまま件数として使わないこと。** マクロ内や cfg 分岐など、
brace matching だけでは判定しきれない形がある。必ず該当箇所を開いて確かめる。
"""

import argparse
import pathlib
import re
import sys

TEST_ATTR = re.compile(r"#\[test\]")
FN_DECL = re.compile(r"\bfn\s+([A-Za-z0-9_]+)\s*\(")
TERMINAL = re.compile(r"^(panic!|unreachable!|todo!|unimplemented!)\s*[\(\[{]")


def body_span(text, open_idx):
    """open_idx の '{' に対応する '}' の位置を返す (文字列/コメントは概ね無視)。"""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i += 1
            while i < n and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def last_top_level_stmt(body):
    """body (中身のみ) の最後の top-level 文の先頭 offset を返す。"""
    depth = 0
    starts = [0]
    i = 0
    n = len(body)
    while i < n:
        c = body[i]
        if c == '"':
            i += 1
            while i < n and body[i] != '"':
                i += 2 if body[i] == "\\" else 1
        elif c in "{([":
            depth += 1
        elif c in "})]":
            depth -= 1
        elif c == ";" and depth == 0:
            starts.append(i + 1)
        i += 1
    # 末尾が空白だけの区切りは捨てる
    while len(starts) > 1 and not body[starts[-1]:].strip():
        starts.pop()
    return starts[-1]


def scan(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    hits = []
    for m in TEST_ATTR.finditer(text):
        fn = FN_DECL.search(text, m.end())
        if not fn or text.count("\n", m.end(), fn.start()) > 6:
            continue
        open_idx = text.find("{", fn.end())
        if open_idx < 0:
            continue
        close_idx = body_span(text, open_idx)
        if close_idx < 0:
            continue
        body = text[open_idx + 1:close_idx]
        tail = body[last_top_level_stmt(body):].lstrip()
        if TERMINAL.match(tail):
            line = text.count("\n", 0, fn.start()) + 1
            ignored = "#[ignore" in text[m.start():fn.start()]
            hits.append((line, fn.group(1), ignored, tail.split("\n", 1)[0][:60]))
    return hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="crates/lsharp-wasm/tests/e2e")
    args = ap.parse_args()
    root = pathlib.Path(args.root)
    if not root.exists():
        print(f"root が無い: {root}", file=sys.stderr)
        return 2
    total = 0
    for path in sorted(root.rglob("*.rs")):
        for line, name, ignored, head in scan(path):
            total += 1
            mark = "ignore" if ignored else "**NOT ignored**"
            print(f"{path}:{line} {name} [{mark}] -> {head}")
    print(f"\n合計 {total} 件")
    return 0


if __name__ == "__main__":
    sys.exit(main())
