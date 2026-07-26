//! source metadata → intent graph adapter の contract tests。
//!
//! テスト本体は node、edge、evidence の責務別 module に分け、ここでは Cargo の
//! integration-test target として公開する module tree だけを保持する。

#[path = "validation_source/edges.rs"]
mod edges;
#[path = "validation_source/evidence.rs"]
mod evidence;
#[path = "validation_source/nodes.rs"]
mod nodes;
