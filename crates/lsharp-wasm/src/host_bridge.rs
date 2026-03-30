use wasmtime::component::Linker;

mod bindings {
    wasmtime::component::bindgen!({
        world: "lsharp-core",
        path: "../../wit/lsharp-core.wit",
    });
}

pub use bindings::lsharp::core::host_process::ProcessResult;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestState {
        caps: HostCapabilities,
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
}
