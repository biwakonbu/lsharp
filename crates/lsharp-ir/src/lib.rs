//! L# 中間表現 (IR)
//!
//! MVP ではフラット化された命令列を使用。
//! 将来的に SSA 形式の BasicBlock ベースに拡張する。

pub mod cache;
pub mod closure;
mod compile;
mod compile_surface;
mod instruction;
mod linker;
pub mod lower;
mod model;
pub mod module_graph;
pub mod root_lifetime;

use sha2::{Digest, Sha256};
use std::fmt;

#[cfg(test)]
use std::collections::HashMap;

pub use cache::{CompilationCache, ModuleCacheEntry, ModuleIrSegments};
#[cfg(test)]
pub(crate) use compile::{
    MultiFileLoweringMode, cached_program_or_parse, compile_multi_file_with_mode,
    merge_scc_declarations, parse_program_for_incremental,
};
pub use compile::{
    analyze_multi_file_incremental_with_overrides, analyze_single_file_incremental,
    compile_multi_file, compile_multi_file_incremental, compile_multi_file_with_cache,
};
pub(crate) use compile_surface::ModuleTypeSurface;
pub use instruction::{Instruction, IrType};
pub use linker::link_modules;
pub use model::{Function, GcField, GcTypeDef, GcTypeKind, GlobalDef, ImportFunc, Module};

#[cfg(test)]
use compile_surface::{collect_import_modules, collect_import_visibility};

#[cfg(test)]
include!("incremental_trackers.rs");

/// SHA-256 ベースのソース fingerprint。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    pub fn from_source(source: &str) -> Self {
        Self::from_bytes(source.as_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        Self(digest.into())
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let source = std::fs::read_to_string(path)?;
        Ok(Self::from_source(&source))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for SourceFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
include!("lib_tests.rs");
