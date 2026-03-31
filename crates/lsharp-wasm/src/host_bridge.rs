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
    bindings::lsharp::core::host_process::add_to_linker(linker, |state| {
        state.host_capabilities()
    })?;
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
    let wasi_get = annotate_wasi_getter::<T, U, _>(move |state| {
        wasmtime_wasi::WasiImpl(get(state))
    });

    http_handler_bindings::wasi::io::poll::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::clocks::monotonic_clock::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::io::error::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::io::streams::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::http::types::add_to_linker_get_host(
        linker,
        &http_options,
        get,
    )?;
    http_handler_bindings::wasi::http::outgoing_handler::add_to_linker_get_host(linker, get)?;
    http_handler_bindings::wasi::clocks::wall_clock::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::random::random::add_to_linker_get_host(linker, wasi_get)?;
    http_handler_bindings::wasi::cli::stderr::add_to_linker_get_host(linker, wasi_get)?;
    Ok(())
}


#[cfg(test)]
mod tests {
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

    struct SyntheticHttpState {
        table: ResourceTable,
        ctx: WasiCtx,
        next_http_rep: u32,
        outgoing_responses_created: usize,
        response_set_ok: Option<bool>,
    }

    impl SyntheticHttpState {
        fn new() -> Self {
            let mut builder = WasiCtxBuilder::new();
            Self {
                table: ResourceTable::new(),
                ctx: builder.build(),
                next_http_rep: 1,
                outgoing_responses_created: 0,
                response_set_ok: None,
            }
        }

        fn fresh_resource<T: 'static>(&mut self) -> Resource<T> {
            let rep = self.next_http_rep;
            self.next_http_rep += 1;
            Resource::new_own(rep)
        }

        fn fresh_fields(&mut self) -> Resource<http_types::Fields> {
            self.fresh_resource()
        }

        fn fresh_incoming_body(&mut self) -> Resource<http_types::IncomingBody> {
            self.fresh_resource()
        }

        fn fresh_outgoing_body(&mut self) -> Resource<http_types::OutgoingBody> {
            self.fresh_resource()
        }

        fn fresh_outgoing_request(&mut self) -> Resource<http_types::OutgoingRequest> {
            self.fresh_resource()
        }

        fn fresh_request_options(&mut self) -> Resource<http_types::RequestOptions> {
            self.fresh_resource()
        }

        fn fresh_incoming_response(&mut self) -> Resource<http_types::IncomingResponse> {
            self.fresh_resource()
        }

        fn fresh_outgoing_response(&mut self) -> Resource<http_types::OutgoingResponse> {
            self.fresh_resource()
        }

        fn fresh_future_trailers(&mut self) -> Resource<http_types::FutureTrailers> {
            self.fresh_resource()
        }

        fn fresh_future_incoming_response(
            &mut self,
        ) -> Resource<http_types::FutureIncomingResponse> {
            self.fresh_resource()
        }

        fn fresh_pollable(&mut self) -> Resource<http_types::Pollable> {
            self.fresh_resource()
        }

        fn fresh_input_stream(&mut self) -> Resource<http_types::InputStream> {
            self.fresh_resource()
        }

        fn fresh_output_stream(&mut self) -> Resource<http_types::OutputStream> {
            self.fresh_resource()
        }

        fn is_valid_status_code(status_code: u16) -> bool {
            (100..=599).contains(&status_code)
        }

        fn is_valid_token(name: &str) -> bool {
            !name.is_empty()
                && name.bytes().all(|byte| {
                    matches!(
                        byte,
                        b'!' | b'#' | b'$' | b'%' | b'&' | b'\''
                            | b'*' | b'+' | b'-' | b'.' | b'^' | b'_'
                            | b'`' | b'|' | b'~' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
                    )
                })
        }

        fn is_valid_scheme(name: &str) -> bool {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        }

        fn is_valid_path(path: &str) -> bool {
            path.starts_with('/')
        }

        fn is_valid_authority(authority: &str) -> bool {
            !authority.is_empty() && !authority.chars().any(char::is_whitespace)
        }

        fn is_valid_header_values(values: &[Vec<u8>]) -> bool {
            values
                .iter()
                .all(|value| !value.iter().any(|byte| matches!(byte, b'\r' | b'\n' | 0)))
        }
    }

    impl WasiView for SyntheticHttpState {
        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }

        fn ctx(&mut self) -> &mut WasiCtx {
            &mut self.ctx
        }
    }

    impl http_types::HostFields for SyntheticHttpState {
        fn new(&mut self) -> Resource<http_types::Fields> {
            self.fresh_fields()
        }

        fn from_list(
            &mut self,
            entries: Vec<(http_types::FieldName, http_types::FieldValue)>,
        ) -> Result<Resource<http_types::Fields>, http_types::HeaderError> {
            if entries.iter().any(|(name, value)| {
                !Self::is_valid_token(name) || !Self::is_valid_header_values(std::slice::from_ref(value))
            }) {
                return Err(http_types::HeaderError::InvalidSyntax);
            }
            Ok(self.fresh_fields())
        }

        fn get(
            &mut self,
            _self_: Resource<http_types::Fields>,
            _name: http_types::FieldName,
        ) -> Vec<http_types::FieldValue> {
            Vec::new()
        }

        fn has(&mut self, _self_: Resource<http_types::Fields>, _name: http_types::FieldName) -> bool {
            false
        }

        fn set(
            &mut self,
            _self_: Resource<http_types::Fields>,
            name: http_types::FieldName,
            value: Vec<http_types::FieldValue>,
        ) -> Result<(), http_types::HeaderError> {
            if !Self::is_valid_token(&name) || !Self::is_valid_header_values(&value) {
                return Err(http_types::HeaderError::InvalidSyntax);
            }
            Ok(())
        }

        fn delete(
            &mut self,
            _self_: Resource<http_types::Fields>,
            name: http_types::FieldName,
        ) -> Result<(), http_types::HeaderError> {
            if !Self::is_valid_token(&name) {
                return Err(http_types::HeaderError::InvalidSyntax);
            }
            Ok(())
        }

        fn append(
            &mut self,
            _self_: Resource<http_types::Fields>,
            name: http_types::FieldName,
            value: http_types::FieldValue,
        ) -> Result<(), http_types::HeaderError> {
            if !Self::is_valid_token(&name)
                || !Self::is_valid_header_values(std::slice::from_ref(&value))
            {
                return Err(http_types::HeaderError::InvalidSyntax);
            }
            Ok(())
        }

        fn entries(&mut self, _self_: Resource<http_types::Fields>) -> Vec<(String, Vec<u8>)> {
            Vec::new()
        }

        fn clone(&mut self, _self_: Resource<http_types::Fields>) -> Resource<http_types::Fields> {
            self.fresh_fields()
        }

        fn drop(&mut self, _rep: Resource<http_types::Fields>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostIncomingRequest for SyntheticHttpState {
        fn method(&mut self, _self_: Resource<http_types::IncomingRequest>) -> http_types::Method {
            http_types::Method::Get
        }

        fn path_with_query(
            &mut self,
            _self_: Resource<http_types::IncomingRequest>,
        ) -> Option<String> {
            Some("/".to_string())
        }

        fn scheme(
            &mut self,
            _self_: Resource<http_types::IncomingRequest>,
        ) -> Option<http_types::Scheme> {
            Some(http_types::Scheme::Https)
        }

        fn authority(
            &mut self,
            _self_: Resource<http_types::IncomingRequest>,
        ) -> Option<String> {
            Some("example.test".to_string())
        }

        fn headers(
            &mut self,
            _self_: Resource<http_types::IncomingRequest>,
        ) -> Resource<http_types::Headers> {
            self.fresh_fields()
        }

        fn consume(
            &mut self,
            _self_: Resource<http_types::IncomingRequest>,
        ) -> Result<Resource<http_types::IncomingBody>, ()> {
            Ok(self.fresh_incoming_body())
        }

        fn drop(&mut self, _rep: Resource<http_types::IncomingRequest>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostOutgoingRequest for SyntheticHttpState {
        fn new(&mut self, _headers: Resource<http_types::Headers>) -> Resource<http_types::OutgoingRequest> {
            self.fresh_outgoing_request()
        }

        fn body(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
        ) -> Result<Resource<http_types::OutgoingBody>, ()> {
            Ok(self.fresh_outgoing_body())
        }

        fn method(&mut self, _self_: Resource<http_types::OutgoingRequest>) -> http_types::Method {
            http_types::Method::Get
        }

        fn set_method(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
            method: http_types::Method,
        ) -> Result<(), ()> {
            if matches!(
                method,
                http_types::Method::Other(ref name) if !Self::is_valid_token(name)
            ) {
                return Err(());
            }
            Ok(())
        }

        fn path_with_query(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
        ) -> Option<String> {
            Some("/".to_string())
        }

        fn set_path_with_query(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
            path_with_query: Option<String>,
        ) -> Result<(), ()> {
            if path_with_query
                .as_deref()
                .is_some_and(|path| !Self::is_valid_path(path))
            {
                return Err(());
            }
            Ok(())
        }

        fn scheme(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
        ) -> Option<http_types::Scheme> {
            Some(http_types::Scheme::Https)
        }

        fn set_scheme(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
            scheme: Option<http_types::Scheme>,
        ) -> Result<(), ()> {
            if matches!(
                scheme.as_ref(),
                Some(http_types::Scheme::Other(name)) if !Self::is_valid_scheme(name)
            ) {
                return Err(());
            }
            Ok(())
        }

        fn authority(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
        ) -> Option<String> {
            Some("example.test".to_string())
        }

        fn set_authority(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
            authority: Option<String>,
        ) -> Result<(), ()> {
            if authority
                .as_deref()
                .is_some_and(|value| !Self::is_valid_authority(value))
            {
                return Err(());
            }
            Ok(())
        }

        fn headers(
            &mut self,
            _self_: Resource<http_types::OutgoingRequest>,
        ) -> Resource<http_types::Headers> {
            self.fresh_fields()
        }

        fn drop(&mut self, _rep: Resource<http_types::OutgoingRequest>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostRequestOptions for SyntheticHttpState {
        fn new(&mut self) -> Resource<http_types::RequestOptions> {
            self.fresh_request_options()
        }

        fn connect_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
        ) -> Option<http_types::Duration> {
            None
        }

        fn set_connect_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
            _duration: Option<http_types::Duration>,
        ) -> Result<(), ()> {
            Ok(())
        }

        fn first_byte_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
        ) -> Option<http_types::Duration> {
            None
        }

        fn set_first_byte_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
            _duration: Option<http_types::Duration>,
        ) -> Result<(), ()> {
            Ok(())
        }

        fn between_bytes_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
        ) -> Option<http_types::Duration> {
            None
        }

        fn set_between_bytes_timeout(
            &mut self,
            _self_: Resource<http_types::RequestOptions>,
            _duration: Option<http_types::Duration>,
        ) -> Result<(), ()> {
            Ok(())
        }

        fn drop(&mut self, _rep: Resource<http_types::RequestOptions>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostResponseOutparam for SyntheticHttpState {
        fn send_informational(
            &mut self,
            _self_: Resource<http_types::ResponseOutparam>,
            status: u16,
            _headers: Resource<http_types::Headers>,
        ) -> Result<(), http_types::ErrorCode> {
            if !(100..=199).contains(&status) {
                return Err(http_types::ErrorCode::HttpProtocolError);
            }
            Ok(())
        }

        fn set(
            &mut self,
            _param: Resource<http_types::ResponseOutparam>,
            response: Result<Resource<http_types::OutgoingResponse>, http_types::ErrorCode>,
        ) {
            self.response_set_ok = Some(response.is_ok());
        }

        fn drop(&mut self, _rep: Resource<http_types::ResponseOutparam>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostIncomingResponse for SyntheticHttpState {
        fn status(&mut self, _self_: Resource<http_types::IncomingResponse>) -> http_types::StatusCode {
            200
        }

        fn headers(
            &mut self,
            _self_: Resource<http_types::IncomingResponse>,
        ) -> Resource<http_types::Headers> {
            self.fresh_fields()
        }

        fn consume(
            &mut self,
            _self_: Resource<http_types::IncomingResponse>,
        ) -> Result<Resource<http_types::IncomingBody>, ()> {
            Ok(self.fresh_incoming_body())
        }

        fn drop(&mut self, _rep: Resource<http_types::IncomingResponse>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostIncomingBody for SyntheticHttpState {
        fn stream(
            &mut self,
            _self_: Resource<http_types::IncomingBody>,
        ) -> Result<Resource<http_types::InputStream>, ()> {
            Ok(self.fresh_input_stream())
        }

        fn finish(
            &mut self,
            _this: Resource<http_types::IncomingBody>,
        ) -> Resource<http_types::FutureTrailers> {
            self.fresh_future_trailers()
        }

        fn drop(&mut self, _rep: Resource<http_types::IncomingBody>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostFutureTrailers for SyntheticHttpState {
        fn subscribe(
            &mut self,
            _self_: Resource<http_types::FutureTrailers>,
        ) -> Resource<http_types::Pollable> {
            self.fresh_pollable()
        }

        fn get(
            &mut self,
            _self_: Resource<http_types::FutureTrailers>,
        ) -> Option<Result<Result<Option<Resource<http_types::Trailers>>, http_types::ErrorCode>, ()>>
        {
            Some(Ok(Ok(None)))
        }

        fn drop(&mut self, _rep: Resource<http_types::FutureTrailers>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostOutgoingResponse for SyntheticHttpState {
        fn new(&mut self, _headers: Resource<http_types::Headers>) -> Resource<http_types::OutgoingResponse> {
            self.outgoing_responses_created += 1;
            self.fresh_outgoing_response()
        }

        fn status_code(
            &mut self,
            _self_: Resource<http_types::OutgoingResponse>,
        ) -> http_types::StatusCode {
            200
        }

        fn set_status_code(
            &mut self,
            _self_: Resource<http_types::OutgoingResponse>,
            status_code: http_types::StatusCode,
        ) -> Result<(), ()> {
            if !Self::is_valid_status_code(status_code) {
                return Err(());
            }
            Ok(())
        }

        fn headers(
            &mut self,
            _self_: Resource<http_types::OutgoingResponse>,
        ) -> Resource<http_types::Headers> {
            self.fresh_fields()
        }

        fn body(
            &mut self,
            _self_: Resource<http_types::OutgoingResponse>,
        ) -> Result<Resource<http_types::OutgoingBody>, ()> {
            Ok(self.fresh_outgoing_body())
        }

        fn drop(&mut self, _rep: Resource<http_types::OutgoingResponse>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostOutgoingBody for SyntheticHttpState {
        fn write(
            &mut self,
            _self_: Resource<http_types::OutgoingBody>,
        ) -> Result<Resource<http_types::OutputStream>, ()> {
            Ok(self.fresh_output_stream())
        }

        fn finish(
            &mut self,
            _this: Resource<http_types::OutgoingBody>,
            _trailers: Option<Resource<http_types::Trailers>>,
        ) -> Result<(), http_types::ErrorCode> {
            Ok(())
        }

        fn drop(&mut self, _rep: Resource<http_types::OutgoingBody>) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::HostFutureIncomingResponse for SyntheticHttpState {
        fn subscribe(
            &mut self,
            _self_: Resource<http_types::FutureIncomingResponse>,
        ) -> Resource<http_types::Pollable> {
            self.fresh_pollable()
        }

        fn get(
            &mut self,
            _self_: Resource<http_types::FutureIncomingResponse>,
        ) -> Option<Result<Result<Resource<http_types::IncomingResponse>, http_types::ErrorCode>, ()>>
        {
            Some(Ok(Ok(self.fresh_incoming_response())))
        }

        fn drop(
            &mut self,
            _rep: Resource<http_types::FutureIncomingResponse>,
        ) -> wasmtime::Result<()> {
            Ok(())
        }
    }

    impl http_types::Host for SyntheticHttpState {
        fn http_error_code(
            &mut self,
            _err: Resource<http_types::IoError>,
        ) -> Option<http_types::ErrorCode> {
            None
        }
    }

    impl outgoing_handler::Host for SyntheticHttpState {
        fn handle(
            &mut self,
            _request: Resource<http_types::OutgoingRequest>,
            _options: Option<Resource<http_types::RequestOptions>>,
        ) -> Result<Resource<http_types::FutureIncomingResponse>, http_types::ErrorCode> {
            Ok(self.fresh_future_incoming_response())
        }
    }

    struct TestState {
        caps: HostCapabilities,
    }

    struct HttpLinkerState {
        http: SyntheticHttpState,
    }

    impl HostCapabilitiesView for TestState {
        fn host_capabilities(&mut self) -> &mut HostCapabilities {
            &mut self.caps
        }
    }

    fn test_state() -> (TestState, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let read_calls = Arc::clone(&calls);
        let write_calls = Arc::clone(&calls);
        let read_files_calls = Arc::clone(&calls);
        let process_calls = Arc::clone(&calls);
        let state = TestState {
            caps: HostCapabilities::new(
                move |path| {
                    read_calls.lock().unwrap().push(format!("read:{path}"));
                    Ok(format!("contents:{path}"))
                },
                move |path, content| {
                    write_calls
                        .lock()
                        .unwrap()
                        .push(format!("write:{path}:{}", content.len()));
                    Ok(())
                },
                move |paths| {
                    read_files_calls
                        .lock()
                        .unwrap()
                        .push(format!("read-files:{}", paths.join(",")));
                    paths.iter().map(|path| Ok(format!("contents:{path}"))).collect()
                },
                move |command, args| {
                    process_calls
                        .lock()
                        .unwrap()
                        .push(format!("process:{command}:{}", args.join(",")));
                    Ok(ProcessResult {
                        exit_code: 0,
                        stdout: "ok".to_string(),
                        stderr: String::new(),
                    })
                },
            ),
        };
        (state, calls)
    }

    fn http_linker_state() -> HttpLinkerState {
        HttpLinkerState {
            http: SyntheticHttpState::new(),
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lsharp_host_bridge_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).expect("temp dir creation failed");
        dir
    }

    fn copy_dir_all(source: &Path, dest: &Path) {
        fs::create_dir_all(dest).expect("destination directory creation failed");
        for entry in fs::read_dir(source).expect("source directory should be readable") {
            let entry = entry.expect("directory entry should be readable");
            let path = entry.path();
            let target = dest.join(entry.file_name());
            if path.is_dir() {
                copy_dir_all(&path, &target);
            } else {
                fs::copy(&path, &target).expect("dependency file copy should succeed");
            }
        }
    }

    fn stage_http_handler_wit_workspace() -> PathBuf {
        let repo_wit_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("wit");
        let staged_root = unique_temp_dir("http_handler_wit");
        fs::copy(
            repo_wit_root.join("lsharp-http-handler.wit"),
            staged_root.join("lsharp-http-handler.wit"),
        )
        .expect("http handler world file should stage successfully");
        copy_dir_all(&repo_wit_root.join("deps"), &staged_root.join("deps"));
        staged_root
    }

    fn compile_http_handler_component(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).expect("source should parse");
        let mut infer = lsharp_types::infer::Infer::new();
        let type_results = infer
            .infer_program(&program)
            .expect("source should type-check");
        let mut lower = lsharp_ir::lower::Lower::new();
        let module = lower
            .lower_program(&program, &type_results)
            .expect("source should lower");
        crate::wasi::emit_wasm_wasi_p2(&module).expect("source should componentize")
    }

    #[test]
    fn test_host_capabilities_forward_callbacks() {
        let (mut state, calls) = test_state();

        let read_result = bindings::lsharp::core::host_fs::Host::read_file(
            &mut state.caps,
            "foo.ls".to_string(),
        )
        .expect("read-file bridge should not fail");
        assert_eq!(read_result, "contents:foo.ls".to_string());

        bindings::lsharp::core::host_fs::Host::write_file(
            &mut state.caps,
            "out.wasm".to_string(),
            vec![1, 2, 3, 4],
        )
        .expect("write-file bridge should not fail");

        let read_files_result = bindings::lsharp::core::host_fs::Host::read_files(
            &mut state.caps,
            vec!["a.ls".to_string(), "b.ls".to_string()],
        );
        assert_eq!(
            read_files_result,
            vec![
                Ok("contents:a.ls".to_string()),
                Ok("contents:b.ls".to_string())
            ]
        );

        let process_result = bindings::lsharp::core::host_process::Host::run_process(
            &mut state.caps,
            "lsharp".to_string(),
            vec!["check".to_string(), "foo.ls".to_string()],
        )
        .expect("run-process bridge should not fail");
        assert_eq!(process_result.exit_code, 0);
        assert_eq!(process_result.stdout, "ok");
        assert_eq!(process_result.stderr, "");

        let logged = calls.lock().unwrap().clone();
        assert!(logged.contains(&"read:foo.ls".to_string()));
        assert!(logged.contains(&"write:out.wasm:4".to_string()));
        assert!(logged.contains(&"read-files:a.ls,b.ls".to_string()));
        assert!(logged.contains(&"process:lsharp:check,foo.ls".to_string()));
    }

    #[test]
    fn test_link_host_capabilities_registers_interfaces() {
        let engine = wasmtime::Engine::default();
        let mut linker: Linker<TestState> = Linker::new(&engine);
        link_host_capabilities(&mut linker).expect("host capability linker registration should succeed");
    }

    #[test]
    fn test_http_handler_bindings_are_generated_from_staged_world() {
        let type_name = std::any::type_name::<HttpHandlerWorld>();
        assert!(
            type_name.ends_with("LsharpHttpHandler"),
            "generated bindings should expose the HTTP handler world root type: {type_name}"
        );

        let _incoming_handler_accessor: for<'a> fn(
            &'a HttpHandlerWorld,
        ) -> &'a http_handler_bindings::exports::wasi::http::incoming_handler::Guest =
            HttpHandlerWorld::wasi_http_incoming_handler;
    }

    #[test]
    fn test_http_handler_link_options_support_chainable_unstable_toggle() {
        let mut options = HttpHandlerLinkOptions::default();
        let before: *mut HttpHandlerLinkOptions = &mut options;
        let after: *mut HttpHandlerLinkOptions =
            options.informational_outbound_responses(true) as *mut _;
        assert_eq!(
            before, after,
            "generated link options should expose the unstable HTTP feature toggle"
        );
    }

    #[test]
    fn test_link_http_handler_world_registers_interfaces_with_wasi_view_state() {
        let engine = wasmtime::Engine::default();
        let mut linker: Linker<HttpLinkerState> = Linker::new(&engine);
        let options = HttpHandlerLinkOptions::default();
        let mut state = http_linker_state();
        let _ = &mut state;
        link_http_handler_world(&mut linker, &options, |state| &mut state.http)
            .expect("HTTP handler linker registration should succeed for a WasiView-backed host");
    }

    #[test]
    fn test_http_handler_world_instantiates_dummy_component_against_synthetic_host() {
        let staged_wit_root = stage_http_handler_wit_workspace();
        let wit_file = staged_wit_root.join("lsharp-http-handler.wit");
        let mut resolve = wit_parser::Resolve::default();
        let (package, _) = resolve
            .push_dir(&staged_wit_root)
            .expect("staged HTTP handler WIT should resolve");
        let world = resolve
            .select_world(&[package], Some("lsharp-http-handler"))
            .expect("HTTP handler world should be selectable");
        let dummy_core_module =
            wit_component::dummy_module(&resolve, world, wit_parser::ManglingAndAbi::Standard32);
        let component_bytes = crate::component_adapter::componentize_core_module(
            &dummy_core_module,
            &wit_file,
            "lsharp-http-handler",
            &[],
        )
        .expect("dummy HTTP handler core module should componentize");

        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&engine, &component_bytes)
            .expect("dummy HTTP handler component should validate");
        let mut linker: Linker<HttpLinkerState> = Linker::new(&engine);
        let options = HttpHandlerLinkOptions::default();
        link_http_handler_world(&mut linker, &options, |state| &mut state.http)
            .expect("HTTP handler linker registration should succeed");
        let mut store = wasmtime::Store::new(&engine, http_linker_state());

        let world = HttpHandlerWorld::instantiate(&mut store, &component, &linker)
            .expect("dummy HTTP handler component should instantiate");
        let _ = world.wasi_http_incoming_handler();

        let _ = fs::remove_dir_all(&staged_wit_root);
    }

    #[test]
    fn test_http_handler_world_calls_lsharp_handle_and_sets_response_outparam() {
        let component_bytes = compile_http_handler_component(r#"(defn handle [request] "ok")"#);
        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&engine, &component_bytes)
            .expect("HTTP handler source should validate as a component");
        let mut linker: Linker<HttpLinkerState> = Linker::new(&engine);
        let options = HttpHandlerLinkOptions::default();
        link_http_handler_world(&mut linker, &options, |state| &mut state.http)
            .expect("HTTP handler linker registration should succeed");
        let mut store = wasmtime::Store::new(&engine, http_linker_state());
        let world = HttpHandlerWorld::instantiate(&mut store, &component, &linker)
            .expect("HTTP handler source should instantiate against the HTTP world");

        world
            .wasi_http_incoming_handler()
            .call_handle(
                &mut store,
                Resource::new_own(1),
                Resource::new_own(2),
            )
            .expect("guest handle export should complete without trapping");

        assert_eq!(
            store.data().http.outgoing_responses_created,
            1,
            "guest should construct one outgoing response"
        );
        assert_eq!(
            store.data().http.response_set_ok,
            Some(true),
            "guest should resolve response-outparam with an Ok(response)"
        );
    }
}
