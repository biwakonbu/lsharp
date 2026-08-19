#!/usr/bin/env python3
"""LINT-SPAN-01 受入条件 3 の確認: let (tag 7) / do (tag 9) ノードに対する
長さ probe が既存コードに無いことを機械的に確かめる (cargo 非依存)。

やること:
  1. selfhost/src/**.ls の全 defn をトップレベル括弧走査で切り出す
  2. tag 7 / 9 / (ast-let) / (ast-do) の分岐直下 600 文字で呼ばれる関数名を集める
     (= let/do 受け取り候補。窓は過大近似で、無関係な兄弟分岐も拾う安全側の見積り)
  3. 候補 defn の本文に (vector-length <ノード風仮引数>) があるかを全数走査する

出力は「候補」であって「確定」ではない。**0 件なら probe は無いと言い切れる**が、
非 0 の場合は 1 件ずつ、その仮引数が本当に let/do を保持しうるかを人が判定する。
判定結果は docs/adr/decisions-lint-span-ast-representation.md の Evidence 節が正本。
"""
import pathlib, re, sys, collections

SRC = pathlib.Path("selfhost/src")

DEFN_RE = re.compile(r"^\(defn\s+([^\s\[\]()]+)\s*\n?\s*\[([^\]]*)\]", re.MULTILINE)

def split_defns(text, path):
    """トップレベル (defn ...) を括弧の対応で切り出す。"""
    out = []
    i = 0
    n = len(text)
    while True:
        i = text.find("\n(defn ", i)
        if i < 0:
            break
        start = i + 1
        depth = 0
        j = start
        in_str = False
        while j < n:
            c = text[j]
            if in_str:
                if c == '\\':
                    j += 2
                    continue
                if c == '"':
                    in_str = False
            elif c == '"':
                in_str = True
            elif c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        body = text[start:j]
        m = DEFN_RE.match(body)
        if m:
            line = text.count("\n", 0, start) + 1
            out.append((m.group(1), m.group(2).split(), body, path, line))
        i = j
    return out

defns = []
for p in sorted(SRC.rglob("*.ls")):
    t = "\n" + p.read_text(errors="replace")
    defns.extend(split_defns(t, p))

by_name = {}
for name, params, body, path, line in defns:
    by_name.setdefault(name, []).append((params, body, path, line))

print(f"走査した defn: {len(defns)} 個 / {len(by_name)} 名")

# 2. let/do 分岐の直下で呼ばれる関数を集める
BRANCH = re.compile(r"\(=\s+(?:tag|node-tag|expr-tag|\(vector-get\s+\w[\w-]*\s+0\))\s+(?:7|9|\(ast-let\)|\(ast-do\))\)")
CALL = re.compile(r"\(([a-zA-Z][\w!?<>=*/+-]*)\s")

seeds = collections.defaultdict(list)   # 関数名 -> 由来
for name, params, body, path, line in defns:
    for m in BRANCH.finditer(body):
        # 分岐条件の直後 600 文字を then 節の近似として見る
        seg = body[m.end(): m.end() + 600]
        for c in CALL.finditer(seg):
            fn = c.group(1)
            if fn in by_name:
                seeds[fn].append(f"{path}:{line} ({name})")

print(f"let/do 分岐の直下で呼ばれる既知 defn: {len(seeds)} 名")
for fn in sorted(seeds):
    print(f"  {fn}")

# 4. 候補 defn の本文で、ノードを保持しそうな仮引数に対する vector-length を探す
NODEISH = {"node", "expr", "e", "ast", "n", "form", "body", "target", "inner", "sub", "child"}
hits = []
for fn in sorted(seeds):
    for params, body, path, line in by_name[fn]:
        cands = [p for p in params if p in NODEISH]
        # 第 1 引数は名前に関わらずノードでありうるので含める
        if params and params[0] not in cands:
            cands.append(params[0])
        for p in cands:
            for m in re.finditer(rf"\(vector-length\s+{re.escape(p)}\)", body):
                ln = line + body.count("\n", 0, m.start())
                snippet = body[max(0, m.start()-60): m.end()+40].replace("\n", " ")
                hits.append((f"{path}:{ln}", fn, p, snippet.strip()))

print(f"\n候補 defn 内でノード風仮引数に掛かる vector-length: {len(hits)} 箇所")
for loc, fn, p, snip in hits:
    print(f"  {loc}  [{fn}] 引数={p}\n      ... {snip}")
