# ADR: v0.2 nested module/private/impl の source metadata projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: Rust source intent adapter の nested declaration traversal
- Related: `EC-M2-01`, `docs/adr/decisions-v0.2-source-intent-nodes.md`,
  `docs/adr/decisions-v0.2-source-type-definition-metadata.md`

## Context

`EC-M2-01` の source node contract は、トップレベルの `defn` や `TypeDef` だけでなく、
`module`、`private`、`impl` の内側にある宣言も同じ typed node registry へ投影することを
要求する。既存の Rust adapter は各 wrapper を再帰走査していたが、wrapper の組み合わせを
一つの positive contract として固定した証跡がなく、宣言順と directive span の保持が回帰で
失われても検知できなかった。

## Decision

- `ModuleDecl` と `ImplDef` は body/methods を source order のまま再帰走査し、`Private` は
  inner declaration をその位置で走査する。
- nested declaration の `:intent` / `:claim` / `:assumption` / `:open-question` は、
  wrapper 名や宣言順から ID を推測せず、既存の wire ID・本文・metadata form span を
  `IntentGraph` node registry へそのまま投影する。
- この slice は Rust parser/AST → source adapter の registry traversal に限定する。
  selfhost/native stage0 parity、source edge/evidence の aggregate、RecordDef metadata、
  CLI/MCP、Mac Apple Silicon / Linux x86_64 の artifact/runtime evidence は別タスクとして
  `[~]` のまま管理する。

## Evidence

- `source_adapter_projects_nested_module_private_and_impl_metadata_in_declaration_order` は、
  module 内の top-level `defn`、`private` 内の `defn`、`impl` method の metadata を一つの
  source fixture から投影し、4 node の wire ID・本文・宣言順・directive span を検証する。
- `cargo test -p lsharp-types --test validation_source`

## Boundary

この判断は Rust source adapter の nested declaration projection が verified であることだけを
示す。selfhost parser/native stage0 が同じ wrapper と span contract を満たすこと、全 source
edge/evidence/manifest の統合、対応 target の実行証跡、EC-M2-01 aggregate の完了は意味しない。
