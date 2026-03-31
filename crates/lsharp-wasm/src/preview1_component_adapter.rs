use std::path::Path;

use crate::component_adapter::{ComponentAdapterError, embed_component_metadata_for_world};

const PREVIEW1_COMPONENT_ADAPTER_WAT: &str = include_str!("preview1_component_adapter.wat");

pub fn build_preview1_component_adapter(wit_dir: &Path) -> Result<Vec<u8>, ComponentAdapterError> {
    let mut adapter = wat::parse_str(PREVIEW1_COMPONENT_ADAPTER_WAT).map_err(|err| {
        ComponentAdapterError::Error {
            msg: format!("preview1 adapter WAT の生成に失敗しました: {err}"),
        }
    })?;
    embed_component_metadata_for_world(&mut adapter, wit_dir, "preview1-adapter")?;
    Ok(adapter)
}
