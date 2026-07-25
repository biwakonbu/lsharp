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
                paths
                    .iter()
                    .map(|path| Ok(format!("contents:{path}")))
                    .collect()
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

    let read_result =
        bindings::lsharp::core::host_fs::Host::read_file(&mut state.caps, "foo.ls".to_string())
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
    link_host_capabilities(&mut linker)
        .expect("host capability linker registration should succeed");
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
        .call_handle(&mut store, Resource::new_own(1), Resource::new_own(2))
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
