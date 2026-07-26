use super::*;

#[path = "http_handler_core.rs"]
mod core;

/// HTTP handler world 向けの Wasm Component を生成する。
pub(super) fn emit_wasm_http_handler_p2(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let core_wasm = core::emit_wasm_http_handler_core(module)?;
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-http-handler.wit");
    crate::component_adapter::componentize_core_module(
        &core_wasm,
        &wit_file,
        "lsharp-http-handler",
        &[],
    )
    .map_err(|err| CodegenError::Error {
        msg: format!("HTTP handler component 化に失敗しました: {err}"),
    })
}
