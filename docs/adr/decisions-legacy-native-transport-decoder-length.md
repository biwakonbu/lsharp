# ADR: native selfhost transport payload length の fail-closed 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/decode-native-selfhost-transport.py`,
  `scripts/ci/test-decode-native-selfhost-transport.sh`
- Related: [LEGACY-MAINT-01](../../TODO.md), `LEGACY-BOOT-01`,
  [native selfhost operations](../development/operations/rust-boundary-reduction.md)

## Context

native selfhost compiler の marker transport は、code/data の byte length を先頭に宣言し、
8-byte packed word 列から stage artifact を復元する。従来の decoder は宣言長に達した時点で
flat payload の残りの packed word を無視していたため、transport の余剰 bytes が検出されず、
復元 artifact と producer output の不一致を manifest 生成前に見逃す可能性があった。

## Decision

flat payload は `ceil(declared_len / 8)` 行と厳密に一致する場合だけ受理する。code payload と
data payload の両方へ同じ検査を適用し、segmented payload の各 segment も同じ packed line
count 検査を通す。宣言長または segment 長が負の場合は payload length error として拒否する。
従来の marker、segment table、manifest の出力形式、正常な flat/segmented bytes の順序は変更しない。

## Evidence

- RED: overlong code/data fixture を追加し、変更前 decoder が余剰 packed word を無視して malformed
  transport を受理することを確認した。
- GREEN: `bash scripts/ci/test-decode-native-selfhost-transport.sh` は flat/segmented の正常系と
  overlong code/data の fail-closed 診断を含めて pass。
- `git diff --check`、対象 Python の構文検査、native selfhost transport の既存 shell contract は
  継続検証対象とする。

## Consequences

transport の余剰 payload は artifact materialization 前に拒否され、manifest の長さと実 bytes の
乖離を隠さない。これは decoder/producer の入力境界を閉じる verified partial slice であり、
selfhost/native manifest producer parity、stage0 provenance、Mac/Linux の current-source runtime
evidence、`LEGACY-BOOT-01` aggregate の完了を意味しない。
