//! v0.2 M2 の intent/evidence graph manifest 出力境界。
//!
//! manifest の public API と `IntentGraph` への extension はここに残し、
//! wire projection の実装は private child module に分離する。

use crate::validation::IntentGraph;

#[path = "validation_output/manifest.rs"]
mod manifest;

/// graph を version 1 manifest JSON に変換する。
pub fn to_manifest_json_string(graph: &IntentGraph) -> serde_json::Result<String> {
    manifest::to_manifest_json_string(graph)
}

/// graph を version 1 manifest の JSON value に変換する。
pub fn to_manifest_json_value(graph: &IntentGraph) -> serde_json::Value {
    manifest::to_manifest_json_value(graph)
}

impl IntentGraph {
    /// graph を version 1 manifest JSON に変換する。
    pub fn to_manifest_json_string(&self) -> serde_json::Result<String> {
        to_manifest_json_string(self)
    }

    /// graph を version 1 manifest の JSON value に変換する。
    pub fn to_manifest_json_value(&self) -> serde_json::Value {
        to_manifest_json_value(self)
    }
}
