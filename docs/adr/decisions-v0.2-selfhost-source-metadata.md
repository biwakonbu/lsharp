# ADR: v0.2 selfhost source metadata の ordered storage

- Status: accepted
- Scope: EC-M2-01 selfhost parser slice
- Related: `EC-M2-01`, `docs/adr/decisions-v0.2-source-intent-nodes.md`, `docs/adr/decisions-v0.2-source-intent-edges.md`

## Context

Rust の source adapter は `intent` / `claim` / `assumption` / `open-question` と node/edge の wire ID を canonical graph へ投影できる。一方、selfhost parser は未対応 directive を読み飛ばしていたため、Rust-free の source validation producer を追加する前に source の順序・payload・spanを失っていた。

## Decision

`Syntax.Parser` は `defn` metadata の M2 node/edge directive を、既存の ordered metadata form と同じ `[kind, payload, directive-start, directive-end]` の形で保持する。payload は2つの string を `[wire-id, text-or-endpoint]` の vectorとして保存し、parser は ID の kind推測、duplicate検査、typed graph投影を行わない。既存の legacy metadata form の kind/slot は変更しない。

## Evidence and boundary

`test_e2e_selfhost_parser_preserves_source_intent_metadata_forms` は `intent`、`claim`、`motivates` の directive 順、kind、wire ID、本文/endpointを selfhost parser runtime で確認する。Rust host が compile/run する parser bundle の evidenceであり、native stage0 evidenceではない。

TypeDef、nested module/private/impl の metadata traversal、evidence record の多フィールド parser、typed graph validation、`validate` / `--emit-manifest` CLI、EmbeddedCli/MCP、Mac Apple Silicon / Linux x86_64 native artifact/runtime parityは後続の残タスクとして保持する。
