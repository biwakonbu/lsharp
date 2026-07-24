use wasmtime::{Config, Engine, Module, component::Component};

/// Wasm artifact の実行形式を検証する mode。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmValidationMode {
    Core,
    CoreWasmGc,
    Component,
}

/// cache から読み込んだ Wasm bytes が target の形式として検証できるか確認する。
pub fn validate_wasm_artifact(bytes: &[u8], mode: WasmValidationMode) -> Result<(), String> {
    let mut config = Config::new();
    if matches!(mode, WasmValidationMode::CoreWasmGc) {
        config.wasm_gc(true);
    }
    let engine = Engine::new(&config)
        .map_err(|error| format!("Wasm engine の構築に失敗しました: {error}"))?;
    match mode {
        WasmValidationMode::Core | WasmValidationMode::CoreWasmGc => Module::new(&engine, bytes)
            .map(|_| ())
            .map_err(|error| format!("core Wasm の検証に失敗しました: {error}")),
        WasmValidationMode::Component => Component::new(&engine, bytes)
            .map(|_| ())
            .map_err(|error| format!("component の検証に失敗しました: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wasm_artifact_accepts_core_module() {
        let bytes = wat::parse_str("(module)").expect("core Wasm を生成できる");
        validate_wasm_artifact(&bytes, WasmValidationMode::Core).expect("core Wasm を検証できる");
    }

    #[test]
    fn test_validate_wasm_artifact_rejects_invalid_bytes() {
        let error = validate_wasm_artifact(b"not-a-wasm", WasmValidationMode::Core)
            .expect_err("不正な bytes は検証に失敗するべき");
        assert!(error.contains("core Wasm"), "検証境界を含むべき: {error}");
    }

    #[test]
    fn test_validate_wasm_artifact_accepts_empty_component() {
        let bytes = wat::parse_str("(component)").expect("empty component を生成できる");
        validate_wasm_artifact(&bytes, WasmValidationMode::Component)
            .expect("empty component を検証できる");
    }
}
