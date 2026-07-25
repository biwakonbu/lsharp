use wasmtime::component::Linker;

mod bindings {
    wasmtime::component::bindgen!({
        world: "lsharp-core",
        path: "../../wit/lsharp-core.wit",
    });
}

#[allow(dead_code, unused_mut, clippy::all)]
mod http_handler_bindings {
    include!(concat!(env!("OUT_DIR"), "/http_handler_bindings.rs"));
}

pub use bindings::lsharp::core::host_process::ProcessResult;
pub use http_handler_bindings::{
    LinkOptions as HttpHandlerLinkOptions, LsharpHttpHandler as HttpHandlerWorld,
};

type ReadFileHandler = Box<dyn FnMut(&str) -> Result<String, String> + Send + Sync>;
type WriteFileHandler = Box<dyn FnMut(&str, &[u8]) -> Result<(), String> + Send + Sync>;
type ReadFilesHandler = Box<dyn FnMut(&[String]) -> Vec<Result<String, String>> + Send + Sync>;
type RunProcessHandler =
    Box<dyn FnMut(&str, &[String]) -> Result<ProcessResult, String> + Send + Sync>;

/// Host 側 capability 実装。
pub struct HostCapabilities {
    read_file: ReadFileHandler,
    write_file: WriteFileHandler,
    read_files: ReadFilesHandler,
    run_process: RunProcessHandler,
}

impl HostCapabilities {
    pub fn new(
        read_file: impl FnMut(&str) -> Result<String, String> + Send + Sync + 'static,
        write_file: impl FnMut(&str, &[u8]) -> Result<(), String> + Send + Sync + 'static,
        read_files: impl FnMut(&[String]) -> Vec<Result<String, String>> + Send + Sync + 'static,
        run_process: impl FnMut(&str, &[String]) -> Result<ProcessResult, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            read_file: Box::new(read_file),
            write_file: Box::new(write_file),
            read_files: Box::new(read_files),
            run_process: Box::new(run_process),
        }
    }
}

/// store data から host capabilities を取得する。
pub trait HostCapabilitiesView {
    fn host_capabilities(&mut self) -> &mut HostCapabilities;
}

fn annotate_wasi_getter<T, U, F>(getter: F) -> F
where
    U: wasmtime_wasi::WasiView,
    F: Fn(&mut T) -> wasmtime_wasi::WasiImpl<&mut U> + Send + Sync + Copy + 'static,
{
    getter
}

impl bindings::lsharp::core::host_fs::Host for HostCapabilities {
    fn read_file(&mut self, path: String) -> Result<String, String> {
        (self.read_file)(&path)
    }

    fn write_file(&mut self, path: String, content: Vec<u8>) -> Result<(), String> {
        (self.write_file)(&path, &content)
    }

    fn read_files(&mut self, paths: Vec<String>) -> Vec<Result<String, String>> {
        (self.read_files)(&paths)
    }
}

impl bindings::lsharp::core::host_process::Host for HostCapabilities {
    fn run_process(&mut self, command: String, args: Vec<String>) -> Result<ProcessResult, String> {
        (self.run_process)(&command, &args)
    }
}

/// linker へ coarse-grained host API を登録する。
pub fn link_host_capabilities<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: HostCapabilitiesView,
{
    bindings::lsharp::core::host_fs::add_to_linker(linker, |state| state.host_capabilities())?;
    bindings::lsharp::core::host_process::add_to_linker(linker, |state| state.host_capabilities())?;
    Ok(())
}

/// HTTP handler world の imports を linker へ登録する。
pub fn link_http_handler_world<T, U>(
    linker: &mut Linker<T>,
    options: &HttpHandlerLinkOptions,
    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
) -> wasmtime::Result<()>
where
    T: Send,
    U: wasmtime_wasi::WasiView
        + http_handler_bindings::wasi::http::types::Host
        + http_handler_bindings::wasi::http::outgoing_handler::Host
        + Send,
{
    let http_options = http_handler_bindings::wasi::http::types::LinkOptions::from(options);
    let wasi_get =
        annotate_wasi_getter::<T, U, _>(move |state| wasmtime_wasi::WasiImpl(get(state)));

    http_handler_bindings::wasi::io::poll::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::clocks::monotonic_clock::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::io::error::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::io::streams::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::http::types::add_to_linker_get_host(linker, &http_options, get)?;
    http_handler_bindings::wasi::http::outgoing_handler::add_to_linker_get_host(linker, get)?;
    http_handler_bindings::wasi::clocks::wall_clock::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::random::random::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::cli::stderr::add_to_linker_get_host(linker, wasi_get)?;
    Ok(())
}

#[cfg(test)]
mod tests;
