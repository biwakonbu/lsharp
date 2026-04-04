use lsharp_ir::{CompilationCache, compile_multi_file, compile_multi_file_incremental};
use lsharp_wasm::incremental_bench::SelfhostIncrementalBenchFixture;

#[test]
fn test_e2e_selfhost_incremental_bench_fixture_single_change_matches_full_compile() {
    let fixture = SelfhostIncrementalBenchFixture::create().expect("fixture should be created");
    let mut cache = CompilationCache::new();

    compile_multi_file_incremental(fixture.entry_path(), &mut cache)
        .expect("warm incremental compile should succeed");
    fixture
        .apply_changed_module_variant()
        .expect("changed module variant should be written");

    let incremental = compile_multi_file_incremental(fixture.entry_path(), &mut cache)
        .expect("incremental compile after one-module change should succeed");
    let full = compile_multi_file(fixture.entry_path())
        .expect("full compile after one-module change should succeed");

    assert_eq!(
        incremental.dump(),
        full.dump(),
        "selfhost benchmark fixture の single-module change でも incremental compile は full compile と同じ IR を返すべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "selfhost benchmark fixture の single-module change でも string_data は full compile と一致するべき"
    );
}
