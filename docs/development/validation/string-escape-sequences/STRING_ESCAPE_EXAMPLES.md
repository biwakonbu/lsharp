# L# String Escape Sequence Examples

This document shows concrete examples of what bytes L# produces for various string literals.

## Example 1: Newline

```lisp
(print-string "\n")
```

**Lexer Processing:**
1. Reads: `\n` (two characters in source: backslash, n)
2. Recognizes escape sequence
3. Produces: Single newline character (0x0A)

**Output Bytes:**
```
0x0A
```

**What you see:** Cursor moves to next line (no visible character)

---

## Example 2: Backslash

```lisp
(print-string "\\")
```

**Lexer Processing:**
1. Reads: `\\` (two characters in source: backslash, backslash)
2. Recognizes escape sequence
3. Produces: Single backslash character (0x5C)

**Output Bytes:**
```
0x5C
```

**What you see:** `\`

---

## Example 3: Text with Newline

```lisp
(print-string "hello\nworld")
```

**Lexer Processing:**
1. Reads: `hello\nworld`
2. `hello` → 5 characters unchanged
3. `\n` → recognized as escape sequence → becomes 0x0A
4. `world` → 5 characters unchanged

**Output Bytes:**
```
0x68 0x65 0x6C 0x6C 0x6F 0x0A 0x77 0x6F 0x72 0x6C 0x64
 h    e    l    l    o    LF   w    o    r    l    d
```

**What you see:**
```
hello
world
```

---

## Example 4: Backslash Followed by 'n'

```lisp
(print-string "\\\n")
```

**Lexer Processing:**
1. Reads: `\\\n` (four characters in source: backslash, backslash, backslash, n)
2. First `\\` → recognized as escape sequence → becomes 0x5C (single backslash)
3. `\n` → recognized as escape sequence → becomes 0x0A (newline)

**Output Bytes:**
```
0x5C 0x0A
 \    LF
```

**What you see:**
```
\
```

(Backslash followed by newline - cursor on next line)

---

## Example 5: Multiple Escapes

```lisp
(print-string "\\t\\n")
```

**Lexer Processing:**
1. `\\` → escape sequence → 0x5C (backslash)
2. `t` → literal character → 0x74 (t)
3. `\\` → escape sequence → 0x5C (backslash)
4. `n` → literal character → 0x6E (n)

**Output Bytes:**
```
0x5C 0x74 0x5C 0x6E
 \    t    \    n
```

**What you see:** `\t\n`

---

## Example 6: Quote Escaping

```lisp
(print-string "He said \"hi\"")
```

**Lexer Processing:**
1. `He said ` → literal characters
2. `\"` → escape sequence → 0x22 (quote)
3. `hi` → literal characters
4. `\"` → escape sequence → 0x22 (quote)

**Output Bytes:**
```
0x48 0x65 0x20 0x73 0x61 0x69 0x64 0x20 0x22 0x68 0x69 0x22
 H    e    SP   s    a    i    d    SP   "    h    i    "
```

**What you see:** `He said "hi"`

---

## Escape Sequence Reference Table

| Escape Sequence | Description | Hex | Decimal | Visual |
|-----------------|-------------|-----|---------|--------|
| `\n` | Newline (LF) | 0x0A | 10 | (next line) |
| `\r` | Carriage Return (CR) | 0x0D | 13 | (moves to start of line) |
| `\t` | Tab | 0x09 | 9 | (tab character) |
| `\\` | Backslash | 0x5C | 92 | `\` |
| `\"` | Double Quote | 0x22 | 34 | `"` |
| `\x` (any other) | Unknown escape | 0x5C + byte(x) | 92 + code | `\x` |

---

## Processing Flow Diagram

```
Source Code String
        ↓
    [LEXER]
Recognizes escape sequences:
- \n → 0x0A
- \\ → 0x5C
- \t → 0x09
- \" → 0x22
- \r → 0x0D
        ↓
    Processed String (raw bytes)
        ↓
    [PARSER → IR → CODEGEN]
    (NO re-escaping happens here)
        ↓
    [WASM OUTPUT]
    Raw bytes emitted exactly as is
        ↓
    [RUNTIME OUTPUT]
    Bytes written to stdout
```

---

## Important Notes

1. **ONE escape sequence per `\`:** Each backslash starts ONE escape sequence
   - `\\` = one backslash (not two)
   - `\n` = one newline (not backslash + n)

2. **No double escaping:** The L# compiler does NOT apply JSON escaping or any secondary encoding to string bytes

3. **UTF-8 safe:** Non-escape characters are processed as UTF-8, so emoji and other multi-byte characters work correctly

4. **Unknown escapes:** If you write `\q` (invalid escape), the lexer produces literal backslash + q

---

## Testing

To verify this behavior, compile and run:

```lisp
(defn main []
  (do
    (print-string "Test 1: ")
    (print-string "\\n")
    (print-string "\n")
    (print-string "Test 2: ")
    (print-string "\\\\")
    (print-string "\n")
    0))
```

This prints:
```
Test 1: \
Test 2: \
```

Where:
- First `\` is from `print-string "\\n"` (produces backslash)
- Then newline from `print-string "\n"`
- Second `\` is from `print-string "\\\\"` (produces backslash)
- Then newline from final `print-string "\n"`
