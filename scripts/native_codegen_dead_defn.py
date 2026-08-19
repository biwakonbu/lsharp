#!/usr/bin/env python3
"""NativeCodegen.ls の未参照 defn を棚卸しする (cargo 非依存)。

`I-25` の根拠を再現するためのスクリプト。2 列を同時に出す:

  1. selfhost/src/**.ls からの呼び出し元数 (0 なら production 未使用)
  2. crates/lsharp-wasm/tests/**.rs からの参照数と、その性質

2 列目が要るのは、「呼び出し元 0」が `.ls` に限った話だからである。
production 未使用でも test の埋め込み L# スニペットから呼ばれている defn は
削除できない。逆に否定 assertion (`!body.contains("...")`) しか無いものは、
削除しても test は pass する。

使い方:
    python3 scripts/native_codegen_dead_defn.py            # 未参照 defn を分類して出す
    python3 scripts/native_codegen_dead_defn.py --summary  # 件数だけ
"""

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
TARGET = REPO / "selfhost/src/Backend/Native/NativeCodegen.ls"
SRC_DIR = REPO / "selfhost/src"
TEST_DIR = REPO / "crates/lsharp-wasm/tests"

DEFN_RE = re.compile(r"^\(defn ([a-zA-Z0-9!?<>=*+/_-]+)", re.MULTILINE)


def collect_defn_names(text):
    """定義順を保ったまま defn 名を集める。"""
    return list(dict.fromkeys(DEFN_RE.findall(text)))


def count_ls_callers(name, ls_blobs):
    """`.ls` 側の呼び出し元数。定義行そのものは数えない。"""
    total = 0
    for lines in ls_blobs.values():
        for line in lines:
            if name not in line:
                continue
            if line.startswith(f"(defn {name} ") or line.startswith(f"(defn {name}\n"):
                continue
            # 部分一致を弾く (foo-bar が foo-bar-baz に含まれる等)
            total += len(re.findall(rf"(?<![a-zA-Z0-9!?<>=*+/_-]){re.escape(name)}(?![a-zA-Z0-9!?<>=*+/_-])", line))
    return total


def classify_test_refs(name, test_blobs):
    """crates test からの参照を「L# 呼び出し」と「ソース文字列 assertion」に分ける。"""
    call = 0
    assertion = 0
    negative = 0
    pattern = re.compile(rf"(?<![a-zA-Z0-9!?<>=*+/_-]){re.escape(name)}(?![a-zA-Z0-9!?<>=*+/_-])")
    for lines in test_blobs.values():
        for line in lines:
            if not pattern.search(line):
                continue
            if "contains(" in line:
                assertion += 1
                if re.search(r"!\s*[\w.]*contains\(", line) or line.lstrip().startswith("!"):
                    negative += 1
            else:
                call += 1
    return call, assertion, negative


def main():
    summary_only = "--summary" in sys.argv

    text = TARGET.read_text()
    names = collect_defn_names(text)

    ls_blobs = {p: p.read_text(errors="replace").split("\n") for p in SRC_DIR.rglob("*.ls")}
    test_blobs = {p: p.read_text(errors="replace").split("\n") for p in TEST_DIR.rglob("*.rs")}

    free = []          # test からも参照されない
    test_called = []   # test の L# スニペットから呼ばれる (削除すると壊れる)
    assert_only = []   # ソース文字列 assertion だけ

    for name in names:
        if count_ls_callers(name, ls_blobs) != 0:
            continue
        call, assertion, negative = classify_test_refs(name, test_blobs)
        if call:
            test_called.append((name, call, assertion))
        elif assertion:
            assert_only.append((name, assertion, negative))
        else:
            free.append(name)

    dead = len(free) + len(test_called) + len(assert_only)
    print(f"NativeCodegen.ls の defn: {len(names)}")
    print(f"selfhost/src からの呼び出し元 0: {dead}")
    print(f"  うち crates test からも参照が無い : {len(free)}")
    print(f"  うち test の L# スニペットが呼ぶ  : {len(test_called)}  <- 削除すると test が壊れる")
    print(f"  うちソース文字列 assertion だけ   : {len(assert_only)}")
    if summary_only:
        return

    print("\n--- test からも参照が無い (削除が test を壊さない) ---")
    for name in free:
        print(f"  {name}")

    print("\n--- test の L# スニペットから呼ばれる ---")
    for name, call, assertion in test_called:
        extra = f" + assertion {assertion}" if assertion else ""
        print(f"  {name}  (呼び出し {call}{extra})")

    print("\n--- ソース文字列 assertion だけ ---")
    for name, assertion, negative in assert_only:
        kind = f"否定 {negative} / 肯定 {assertion - negative}"
        print(f"  {name}  ({kind})")


if __name__ == "__main__":
    main()
