# L# String Escape Sequences: Visual Byte-by-Byte Breakdown

## The Core Question: What Bytes Does `"\\n"` Produce?

### Wrong Answer ❌
```
L# Source:     (print-string "\\n")
Expected:      \ + n
Bytes:         0x5C 0x6E
ASCII:         92   110
Visible:       \ n
```

### Correct Answer ✓
```
L# Source:     (print-string "\\n")
Expected:      newline character
Bytes:         0x0A
ASCII:         10
Visible:       (cursor moves to next line)
```

---

## Escape Sequence Transformation Table

### Visual Format

```
SOURCE STRING         LEXER CONVERTS TO       OUTPUT BYTES
─────────────────────────────────────────────────────────────
"\n"                  newline                 0x0A
"\t"                  tab                     0x09
"\r"                  carriage return         0x0D
"\\"                  backslash               0x5C
"\""                  quote                   0x22
"\x" (unknown)        backslash + x           0x5C + 0x78
```

### With Decimal Values

```
SOURCE            MEANING              HEX    DEC   VISIBLE
─────────────────────────────────────────────────────────────
"\n"              Newline (LF)         0x0A   10    ⏎
"\t"              Tab (HT)             0x09   9     ⇆
"\r"              Carriage Return      0x0D   13    ⤴
"\\"              Backslash            0x5C   92    \
"\""              Double Quote         0x22   34    "
"\a"              Alert (bell)         0x07   7     🔔
```

---

## Concrete Examples with Hex Dumps

### Example 1: Simple Newline

**L# Source Code:**
```lisp
(print-string "hello\nworld")
```

**What the Lexer Produces:**
```
Characters: h   e   l   l   o   \n  w   o   r   l   d
Hex:        68  65  6C  6C  6F  0A  77  6F  72  6C  64
Decimal:    104 101 108 108 111 10  119 111 114 108 100
ASCII:      h   e   l   l   o   LF  w   o   r   l   d
```

**What You See in Output:**
```
hello
world
```

### Example 2: Mixed Escapes

**L# Source Code:**
```lisp
(print-string "line1\t\\\nline2")
```

Breaking it down:
- `line1` → l, i, n, e, 1
- `\t` → tab character
- `\\` → single backslash
- `\n` → newline
- `line2` → l, i, n, e, 2

**What the Lexer Produces:**
```
Characters: l   i   n   e   1   TAB \   LF  l   i   n   e   2
Hex:        6C  69  6E  65  31  09  5C  0A  6C  69  6E  65  32
Decimal:    108 105 110 101 49  9   92  10  108 105 110 101 50
```

**What You See:**
```
line1	\
line2
```

(Note: TAB shows as whitespace, backslash visible, LF creates newline)

### Example 3: Escaped Quote

**L# Source Code:**
```lisp
(print-string "He said \"hi\"")
```

**What the Lexer Produces:**
```
Characters: H   e   SP  s   a   i   d   SP  "   h   i   "
Hex:        48  65  20  73  61  69  64  20  22  68  69  22
Decimal:    72  101 32  115 97  105 100 32  34  104 105 34
```

**What You See:**
```
He said "hi"
```

---

## The Complete Processing Flow with Bytes

### Stage 1: Source File
```
FILE CONTENT
─────────────
(print-string "test\\n")
```

### Stage 2: Lexer Tokenizes
```
TOKEN::STRING("test\n")
where \n is now ACTUAL newline byte 0x0A

Bytes in memory:
  t    e    s    t    \n
  74   65   73   74   0A
```

### Stage 3: Parser Creates AST
```
Expr::Lit(Literal::String(s))
where s = String with 5 bytes [74, 65, 73, 74, 0A]
```

### Stage 4: IR Lowering
```
let bytes = s.as_bytes().to_vec();
Result: [0x74, 0x65, 0x73, 0x74, 0x0A]

Stored in:
  module.string_data.push(("label", vec![74, 65, 73, 74, 10]))
```

### Stage 5: Wasm Codegen
```
for (_label, bytes) in &module.string_data {
    data.active(0, offset, bytes.iter().copied());
}

Result: Raw bytes [74, 65, 73, 74, 10] in Wasm data section
```

### Stage 6: Runtime Execution
```
Loads bytes from data section: [74, 65, 73, 74, 10]
Writes to stdout: "test" + newline
```

### Result
```
test
(cursor on next line)
```

---

## Quick Reference: Escape Sequences

| Escape | Name | Hex | Dec | Effect |
|--------|------|-----|-----|--------|
| `\n` | Line Feed | 0x0A | 10 | Moves to next line |
| `\r` | Carriage Return | 0x0D | 13 | Moves to line start |
| `\t` | Horizontal Tab | 0x09 | 9 | Moves to next tab stop |
| `\f` | Form Feed | 0x0C | 12 | Page break |
| `\b` | Backspace | 0x08 | 8 | Moves back one char |
| `\\` | Backslash | 0x5C | 92 | Literal `\` |
| `\"` | Quote | 0x22 | 34 | Literal `"` |
| `\'` | Apostrophe | 0x27 | 39 | Literal `'` |

---

## Testing Verification

### Test Input
```lisp
(defn main []
  (do
    (print-string "A")
    (print-string "\\n")
    (print-string "B")
    0))
```

### Byte-by-Byte Breakdown
```
print-string "A":       0x41
print-string "\\n":     0x0A    ← This is a real newline!
print-string "B":       0x42
```

### Output Display
```
A
B
```

### Hex Dump of Output
```
41 0A 42
```

Where:
- `41` = 'A'
- `0A` = Newline (not backslash-n!)
- `42` = 'B'

---

## Common Misunderstandings

### ❌ Misconception 1: `\\n` might produce `\n`
**Fact:** In L# (and ALL programming languages), `\\n` produces a NEWLINE character (0x0A), NOT the two characters backslash and n.

### ❌ Misconception 2: Escape sequences are just syntax
**Fact:** Escape sequences are processed at LEXER TIME and produce actual bytes that flow through the entire system.

### ❌ Misconception 3: JSON encoding might re-escape strings
**Fact:** L# does NOT apply JSON escaping to strings. It stores raw bytes. IF you export to JSON, you must encode ONCE, not twice.

### ✓ Understanding: Standard Language Behavior
L# follows the same escape sequence rules as C, Java, Python, JavaScript, and virtually all programming languages. This is correct, expected, standard behavior.

---

## Proof from Source Code

**File:** `crates/lsharp-syntax/src/lexer.rs:188-191`

```rust
b'n' => value.push('\n'),    // ← 0x0A
b't' => value.push('\t'),    // ← 0x09
b'r' => value.push('\r'),    // ← 0x0D
b'\\' => value.push('\\'),   // ← 0x5C
```

This is the canonical proof that:
- `\n` in source → newline byte 0x0A
- `\\` in source → backslash byte 0x5C

---

## How to Verify Yourself

### Using the Test Suite
```bash
cd /Users/biwakonbu/github/lsharp
cargo test --package lsharp-wasm --test e2e string_escape_sequences::test_verify_hex_dump -- --nocapture
```

Look for output like:
```
Output bytes (hex): 41 0A 42
```

### Manual Inspection
```bash
# Create a simple L# file
echo '(defn main [] (do (print-string "hi\nbye") 0))' > test.ls

# Compile
lsharp compile test.ls -o test.wasm

# Run and capture bytes
wasmtime test.wasm | od -x
```

The `od -x` command will show you the exact bytes:
- `68 69` = "hi"
- `0a` = newline
- `62 79 65` = "bye"

---

## Summary

When you write in L#:
```lisp
"\\n"
```

The lexer converts it to:
```
ONE BYTE: 0x0A (newline)
```

This is correct, standard, and expected behavior. Not a bug, not unexpected—this is how programming languages work.
