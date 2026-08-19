#!/usr/bin/env python3
"""NativeCodegen.ls の helper emitter が emit する機械語バイト列を cargo 無しで取り出す。

`selfhost/src/Backend/Native/NativeCodegen.ls` は native の機械語をバイト列リテラルとして
持っている。これを読むだけなら Rust ツールチェインは要らないが、**リテラルを出現順に
並べるだけでは誤る**:

- `(concat-three-byte-vectors-rooted (byte-vector-2 ...) heap-base (byte-vector-3 ...))` のように
  `let` 束縛が引数順で並べ替えられる。grep の出現順とバイト順は一致しない
- `read-stdin` / `int-to-string` / `string-concat` の chunk 群は `(ref-new (vector-new N))` へ
  `append-encoded-u32-rooted` を積む形式で、リテラル抽出では 1 word も拾えない

そのため本スクリプトは S 式を評価してバイト列を組み立てる。評価に必要な形は 8 つだけ。

用途:
  --list      heap frontier を進める helper を両 lane で列挙する (bounds check の棚卸し)
  --dump NAME 指定 helper のバイト列を hex で出す
  --selftest  既知の helper に対する期待値と照合する
"""

import re
import sys

SOURCE = "selfhost/src/Backend/Native/NativeCodegen.ls"


# --- S 式の読み取り -------------------------------------------------------


def tokenize(text):
    """行コメントを落として括弧と atom に分割する。"""
    text = re.sub(r";[^\n]*", "", text)
    return re.findall(r"\(|\)|\[|\]|[^\s()\[\]]+", text)


def parse(tokens, i=0):
    out = []
    while i < len(tokens):
        tok = tokens[i]
        if tok in "([":
            sub, i = parse(tokens, i + 1)
            out.append(sub)
        elif tok in ")]":
            return out, i + 1
        else:
            out.append(tok)
            i += 1
    return out, i


class Unsupported(Exception):
    """評価器が知らない形。helper が新しい構築形式を使い始めたら上がる。"""


def _u32_le(value):
    value &= 0xFFFFFFFF
    return [value & 0xFF, (value >> 8) & 0xFF, (value >> 16) & 0xFF, (value >> 24) & 0xFF]


def evaluate(expr, env):
    if isinstance(expr, str):
        if re.fullmatch(r"-?\d+", expr):
            return int(expr)
        if expr in env:
            return env[expr]
        raise Unsupported(expr)
    if not expr:
        raise Unsupported("empty form")
    head = expr[0]
    if isinstance(head, list):
        raise Unsupported("list in head position")

    if head == "let":
        binds = expr[1]
        env = dict(env)
        for i in range(0, len(binds), 2):
            env[binds[i]] = evaluate(binds[i + 1], env)
        result = None
        for body in expr[2:]:
            result = evaluate(body, env)
        return result
    if head == "do":
        result = None
        for body in expr[1:]:
            result = evaluate(body, env)
        return result
    if head.startswith("byte-vector-"):
        return [evaluate(a, env) & 0xFF for a in expr[1:]]
    if head == "encode-u32-le":
        return _u32_le(evaluate(expr[1], env))
    if head.startswith("concat-") and "byte-vector" in head:
        out = []
        for a in expr[1:]:
            out += evaluate(a, env)
        return out
    if head == "vector-new":
        # 容量指定の空ベクタ。バイト列としては空。
        return []
    if head == "ref-new":
        return {"cell": evaluate(expr[1], env)}
    if head == "ref-get":
        return evaluate(expr[1], env)["cell"]
    if head == "ref-set":
        ref = evaluate(expr[1], env)
        ref["cell"] = evaluate(expr[2], env)
        return ref
    if head in ("root_push", "root_pop"):
        # root stack の出し入れはバイト列に影響しない。
        return None
    if head == "append-encoded-u32-rooted":
        ref = evaluate(expr[1], env)
        ref["cell"] = ref["cell"] + _u32_le(evaluate(expr[2], env))
        return ref
    if head == "emit-aarch64-bl":
        offset = evaluate(expr[1], env)
        return _u32_le(0x94000000 | ((offset >> 2) & 0x03FFFFFF))
    raise Unsupported(head)


# --- helper の取り出し ----------------------------------------------------


def load_lines(path=SOURCE):
    with open(path, encoding="utf-8") as handle:
        return handle.read().split("\n")


def helper_names(lines, prefix):
    return [
        re.match(r"\(defn (\S+)", line).group(1)
        for line in lines
        if line.startswith(f"(defn {prefix}")
    ]


def helper_bytes(lines, name):
    """helper 名からバイト列を返す。評価できなければ (None, 理由)。"""
    starts = [i for i, line in enumerate(lines) if line.startswith(f"(defn {name} ")]
    if not starts:
        return None, "定義が見つからない"
    start = starts[0]
    following = [i for i, line in enumerate(lines) if line.startswith("(defn ") and i > start]
    end = following[0] if following else len(lines)
    ast, _ = parse(tokenize("\n".join(lines[start:end])))
    decl = ast[0]  # (defn name [] body...)
    try:
        result = None
        for body in decl[3:]:
            result = evaluate(body, {})
        return result, None
    except Unsupported as err:
        return None, f"未対応の形: {err}"


# --- frontier bump と limit 参照の検出 ------------------------------------


def scan_x86(data):
    """x86-64: cursor/limit は heap 先頭 16 bytes (r14 基準)。"""
    bump = [
        i
        for i in range(len(data) - 2)
        if data[i] == 0x49 and data[i + 1] == 0x89 and (data[i + 2] & 0xC7) == 0x06
    ]
    # mov rN, [r14+8] (limit のロード) と cmp rN, [r14+8] (直接比較) の双方を数える。
    limit = [
        i
        for i in range(len(data) - 3)
        if data[i] == 0x49
        and data[i + 1] in (0x8B, 0x39, 0x3B)
        and (data[i + 2] & 0xC7) == 0x46
        and data[i + 3] == 8
    ]
    return bump, limit


def scan_aarch64(data):
    """aarch64: x21 が heap base、x22 が frontier。limit を持つ場所は無い。"""
    words = [int.from_bytes(bytes(data[i : i + 4]), "little") for i in range(0, len(data) - 3, 4)]
    bump = [
        i
        for i, w in enumerate(words)
        if (w & 0xFFE0FC00) == 0x8B000000 and (w & 31) == 22 and ((w >> 5) & 31) == 22
    ]
    cmps = []
    for i, w in enumerate(words):
        if (w & 0xFFE0001F) == 0xEB00001F:
            cmps.append((i, f"cmp x{(w >> 5) & 31}, x{(w >> 16) & 31}"))
        elif (w & 0xFF80001F) == 0xF100001F:
            cmps.append((i, f"cmp x{(w >> 5) & 31}, #{(w >> 10) & 0xFFF}"))
        elif (w & 0xFFE0001F) == 0x6B00001F:
            cmps.append((i, f"cmp w{(w >> 5) & 31}, w{(w >> 16) & 31}"))
        elif (w & 0xFF80001F) == 0x7100001F:
            cmps.append((i, f"cmp w{(w >> 5) & 31}, #{(w >> 10) & 0xFFF}"))
    return words, bump, cmps


def cmd_list(lines):
    print("=== x86-64: heap frontier を進める helper ===")
    unresolved = []
    for name in helper_names(lines, "emit-x86-selfhost-"):
        data, err = helper_bytes(lines, name)
        if err:
            unresolved.append((name, err))
            continue
        bump, limit = scan_x86(data)
        if not bump:
            continue
        print(f"  {name:50s} {len(data):4d}B  bump={len(bump)}  limit 参照={len(limit)}")
    print("  評価不能:", ", ".join(f"{n} ({e})" for n, e in unresolved) or "なし")

    print()
    print("=== aarch64: heap frontier を進める helper ===")
    unresolved = []
    for name in helper_names(lines, "emit-aarch64-selfhost-"):
        data, err = helper_bytes(lines, name)
        if err:
            unresolved.append((name, err))
            continue
        words, bump, cmps = scan_aarch64(data)
        if not bump:
            continue
        offsets = ", ".join(hex(i * 4) for i in bump)
        print(f"  {name:50s} {len(words):4d}W  bump@[{offsets}]")
        for i, text in cmps:
            print(f"       cmp@{i * 4:#06x}  {text}")
    print("  評価不能:", ", ".join(f"{n} ({e})" for n, e in unresolved) or "なし")


def cmd_dump(lines, name):
    data, err = helper_bytes(lines, name)
    if err:
        print(f"{name}: {err}", file=sys.stderr)
        return 1
    print(f"{name}: {len(data)} bytes")
    for offset in range(0, len(data), 16):
        chunk = " ".join(f"{b:02x}" for b in data[offset : offset + 16])
        print(f"  {offset:04x}  {chunk}")
    return 0


# --- self test ------------------------------------------------------------

# x86 map-new の先頭。cursor ロード → 65,296 加算 → limit ロード → cmp → ja → bump の順で、
# 「並べ替えを正しく解決できているか」がここで落ちる (出現順に並べると bump が cmp の前に来る)。
SELFTEST_X86_MAP_NEW_HEAD = (
    "51 49 8b 06 48 89 c7 b9 10 ff 00 00 48 01 cf 49 8b 4e 08 48 39 cf 77 28 49 89 3e"
)

# aarch64 map-new の無条件 64 KiB bump。movz x2,#1,lsl#16 → add x22,x22,x2。
SELFTEST_A64_MAP_NEW_WORDS = [0xD2A00022, 0x8B0202D6]


def cmd_selftest(lines):
    failures = []

    data, err = helper_bytes(lines, "emit-x86-selfhost-map-new-helper")
    if err:
        failures.append(f"x86 map-new を評価できない: {err}")
    else:
        head = " ".join(f"{b:02x}" for b in data[: len(SELFTEST_X86_MAP_NEW_HEAD.split())])
        if head != SELFTEST_X86_MAP_NEW_HEAD:
            failures.append(f"x86 map-new の先頭が期待と違う:\n  期待 {SELFTEST_X86_MAP_NEW_HEAD}\n  実際 {head}")
        bump, limit = scan_x86(data)
        if len(bump) != 1 or len(limit) != 1:
            failures.append(f"x86 map-new の bump/limit 検出が期待と違う: bump={len(bump)} limit={len(limit)}")

    data, err = helper_bytes(lines, "emit-aarch64-selfhost-map-new-helper")
    if err:
        failures.append(f"aarch64 map-new を評価できない: {err}")
    else:
        words, bump, _ = scan_aarch64(data)
        if len(bump) != 1:
            failures.append(f"aarch64 map-new の bump 検出数が 1 でない: {len(bump)}")
        else:
            at = bump[0]
            got = words[at - 1 : at + 1]
            if got != SELFTEST_A64_MAP_NEW_WORDS:
                failures.append(
                    "aarch64 map-new の bump 前後が期待と違う: "
                    + ", ".join(f"{w:#010x}" for w in got)
                )

    # append-encoded-u32-rooted 形式が読めること (リテラル抽出では 0 word になる helper)。
    data, err = helper_bytes(lines, "emit-aarch64-selfhost-read-stdin-helper")
    if err:
        failures.append(f"aarch64 read-stdin を評価できない: {err}")
    elif len(data) % 4 != 0 or len(data) == 0:
        failures.append(f"aarch64 read-stdin のバイト数が word 境界でない: {len(data)}")

    if failures:
        for line in failures:
            print("FAIL: " + line, file=sys.stderr)
        return 1
    print("selftest: ok")
    return 0


def main(argv):
    lines = load_lines()
    if len(argv) >= 2 and argv[1] == "--list":
        cmd_list(lines)
        return 0
    if len(argv) >= 3 and argv[1] == "--dump":
        return cmd_dump(lines, argv[2])
    if len(argv) >= 2 and argv[1] == "--selftest":
        return cmd_selftest(lines)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
