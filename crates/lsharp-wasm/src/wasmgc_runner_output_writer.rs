use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::{run_wasm_wasmgc_component_output_with_stdout_sink, run_wasm_wasmgc_with_stdout_sink};

/// WasmGC component output を `std::io::Write` へ接続して実行する。
///
/// canonical import 一回分の bytes は `Write::write_all` で全量を消費し、partial write は
/// 内部で再試行する。`WriteZero` / write error は trap として返し、main の正常終了後だけ
/// `flush` を呼び出すため、exit code と flush error の順序も固定される。
pub fn run_wasm_wasmgc_component_output_to_writer<W>(
    wasm_bytes: &[u8],
    writer: W,
) -> Result<i32, String>
where
    W: Write + Send + 'static,
{
    let writer = Arc::new(Mutex::new(writer));
    let writer_for_sink = Arc::clone(&writer);
    let exit_code = run_wasm_wasmgc_component_output_with_stdout_sink(wasm_bytes, move |bytes| {
        let mut writer = writer_for_sink
            .lock()
            .map_err(|_| "WasmGC component output writer の mutex が poisoned です".to_string())?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("WasmGC component output writer failed: {error}"))
    })?;
    let mut writer = writer
        .lock()
        .map_err(|_| "WasmGC component output writer の mutex が poisoned です".to_string())?;
    writer
        .flush()
        .map_err(|error| format!("WasmGC component output writer flush failed: {error}"))?;
    Ok(exit_code)
}

/// canonical output を stdout 相当の WASI `fd_write` 境界へ接続して実行する。
///
/// `fd_write` handler は fd と一つの bytes chunk を受け取り、実際に消費した byte 数または
/// WASI errno を返す。partial write は `write_all` が再試行し、zero/over-report/errno は
/// fail-closed に停止する。handler の背後にある実 WASI context の所有権は呼び出し側に残す。
pub fn run_wasm_wasmgc_component_output_to_fd_write<F>(
    wasm_bytes: &[u8],
    fd: u32,
    fd_write: F,
) -> Result<i32, String>
where
    F: Fn(u32, &[u8]) -> Result<usize, u16> + Send + Sync + 'static,
{
    run_wasm_wasmgc_component_output_to_writer(
        wasm_bytes,
        ComponentOutputFdWriteAdapter { fd, fd_write },
    )
}

struct ComponentOutputFdWriteAdapter<F> {
    fd: u32,
    fd_write: F,
}

impl<F> Write for ComponentOutputFdWriteAdapter<F>
where
    F: Fn(u32, &[u8]) -> Result<usize, u16>,
{
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = (self.fd_write)(self.fd, bytes)
            .map_err(|errno| io::Error::other(format!("WASI fd_write errno {errno}")))?;
        if written > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "WASI fd_write over-reported bytes: {written} > {}",
                    bytes.len()
                ),
            ));
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// WasmGC core module を `std::io::Write` へ接続して実行する。
///
/// 各 `print-string` chunk は `Write::write_all` で全量を消費し、partial write は内部で再試行する。
/// `WriteZero` や I/O error は sink error として Wasm 実行へ返し、正常終了後には `flush` する。
pub fn run_wasm_wasmgc_to_writer<W>(wasm_bytes: &[u8], writer: W) -> Result<i32, String>
where
    W: Write + Send + 'static,
{
    let writer = Arc::new(Mutex::new(writer));
    let writer_for_sink = Arc::clone(&writer);
    let exit_code = run_wasm_wasmgc_with_stdout_sink(wasm_bytes, move |bytes| {
        let mut writer = writer_for_sink
            .lock()
            .map_err(|_| "WasmGC stdout writer の mutex が poisoned です".to_string())?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("WasmGC stdout writer failed: {error}"))
    })?;
    let mut writer = writer
        .lock()
        .map_err(|_| "WasmGC stdout writer の mutex が poisoned です".to_string())?;
    writer
        .flush()
        .map_err(|error| format!("WasmGC stdout writer flush failed: {error}"))?;
    Ok(exit_code)
}
