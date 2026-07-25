use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

use http_handler_bindings::wasi::http::outgoing_handler;
use http_handler_bindings::wasi::http::types as http_types;

include!("synthetic_http_state.rs");
include!("operations.rs");
