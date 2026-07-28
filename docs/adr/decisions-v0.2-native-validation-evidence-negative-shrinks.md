# ADR: v0.2 evidence `shrinks` の負値 boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: source `:evidence` の optional sampling `:shrinks`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-negative-seed.md`

## Context

canonical sampling の shrink path は非負整数列だが、source metadata の parser→selfhost Evidence
registry 経路と native `validate` smoke が負値を同じ境界で検証していなかった。selfhost direct consumer
と Rust parser の既存テストだけでは source入力の diagnostic span と CLI fail-closed を証明できない。

## Decision

- Rust source parser は `:shrinks [-1]` を parser `Unexpected` の stable code `LS0101` で拒否する。
- selfhost source parser→Evidence registry は負の shrink element を invalid-sampling code `11`、field
  `shrinks`、empty raw value、non-empty directive/form span 付きで拒否する。
- native source-file validation は exit `1`、`source validation error:11`、report/manifestなしを要求する。
- shrink の生成・短縮アルゴリズム、上限、manifest input、current-source stage0 artifact/runtime、
  supported target matrix は未検証のため対象外とする。

## Evidence

- Rust `intent_edges` の既存 negative-shrink parser testは `LS0101` を通過した。
- selfhost actual Wasm の source parser→Evidence registry fixtureを追加し、status `0`、code `11`、
  field `shrinks`、empty raw value、非空 span を確認した。
- native source-file smoke に negative-shrinks fixture、exit/report/manifest assertions を追加した。
- `bash -n` 両 smoke script と `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  を通過した。

## Boundary and follow-up

これは evidence `shrinks` の parser/consumer boundary に限定した verified partial sliceである。
generator/shrink の実行意味論、manifest/validate、current-source stage0 artifact/runtime、Mac/Linux
matrix、EC-M2-02 aggregate は未完了であり、TODO の `[~]` を維持する。
