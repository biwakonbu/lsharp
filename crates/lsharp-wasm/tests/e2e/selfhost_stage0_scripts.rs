#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
fn ops07_unique_temp_dir(label: &str) -> std::path::PathBuf {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time が epoch より前")
        .as_nanos();
    let dir = project_root
        .join("target/e2e-selfhost-fixtures")
        .join(format!(
            "lsharp-ops07-{label}-{}-{}",
            std::process::id(),
            nanos
        ));
    std::fs::create_dir_all(&dir).expect("temp dir の作成に失敗");
    dir
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    let mut perms = std::fs::metadata(path)
        .expect("metadata の取得に失敗")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("permission の設定に失敗");
}

#[cfg(unix)]
fn write_native_stage0_fixture(path: &std::path::Path, target: &str, source_commit: &str) {
    let bin_dir = path.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("native stage0 fixture bin の作成に失敗");
    for executable in ["compiler", "transport-driver", "materializer"] {
        let executable_path = bin_dir.join(executable);
        std::fs::write(
            &executable_path,
            "#!/usr/bin/env bash\nset -euo pipefail\nexit 0\n",
        )
        .unwrap_or_else(|e| panic!("{} の書き込みに失敗: {e}", executable_path.display()));
        make_executable(&executable_path);
    }
    std::fs::write(bin_dir.join("materializer.py"), "#!/usr/bin/env python3\n")
        .expect("native stage0 fixture materializer の書き込みに失敗");
    std::fs::write(
        path.join("manifest.json"),
        format!(
            r#"{{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "{target}",
  "source_commit": "{source_commit}",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}}
"#
        ),
    )
    .expect("native stage0 fixture manifest の書き込みに失敗");
}

#[test]
fn test_native_macos_aarch64_materializer_supports_explicit_codesign_identity() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let materializer = project_root.join("scripts/ci/materialize-native-macos-aarch64-bundle.py");
    let source = std::fs::read_to_string(&materializer)
        .unwrap_or_else(|e| panic!("{} 読み込み失敗: {e}", materializer.display()));

    for required in [
        "LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY",
        "codesign",
        "--force",
        "--sign",
        "--timestamp=none",
        "program.native",
        "capture_output=True",
    ] {
        assert!(
            source.contains(required),
            "macOS native materializer は optional code signing の `{required}` を固定するべき"
        );
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn test_e2e_native_macos_aarch64_materializer_executes_tiny_stage_code() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let materializer = project_root.join("scripts/ci/materialize-native-macos-aarch64-bundle.py");
    let temp_root = ops07_unique_temp_dir("native-macos-aarch64-materializer");
    let stage_dir = temp_root.join("stage");
    let code_name = "tiny-stage-code.bin";
    let entrypoint_name = "entrypoint-offset.txt";
    std::fs::create_dir_all(&stage_dir).expect("stage dir の作成に失敗");
    std::fs::write(
        stage_dir.join(code_name),
        [0x00, 0x00, 0x80, 0x52, 0xc0, 0x03, 0x5f, 0xd6],
    )
    .expect("tiny stage code の書き込みに失敗");
    std::fs::write(stage_dir.join(entrypoint_name), "0\n")
        .expect("entrypoint offset の書き込みに失敗");

    let materialize = Command::new("python3")
        .arg(&materializer)
        .arg(&stage_dir)
        .arg(code_name)
        .arg(entrypoint_name)
        .env("LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY", "-")
        .output()
        .expect("macOS arm64 materializer の実行に失敗");
    assert!(
        materialize.status.success(),
        "macOS arm64 materializer が失敗した: status={:?}, stdout={}, stderr={}",
        materialize.status.code(),
        String::from_utf8_lossy(&materialize.stdout),
        String::from_utf8_lossy(&materialize.stderr)
    );
    assert!(
        materialize.stderr.is_empty(),
        "macOS arm64 materializer の成功時 stderr は空であるべき: {:?}",
        String::from_utf8_lossy(&materialize.stderr)
    );

    let resign = Command::new("python3")
        .arg(&materializer)
        .arg(&stage_dir)
        .arg(code_name)
        .arg(entrypoint_name)
        .env("LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY", "-")
        .output()
        .expect("macOS arm64 materializer の再実行に失敗");
    assert!(
        resign.status.success(),
        "macOS arm64 materializer の再署名が失敗した: status={:?}, stdout={}, stderr={}",
        resign.status.code(),
        String::from_utf8_lossy(&resign.stdout),
        String::from_utf8_lossy(&resign.stderr)
    );
    assert!(
        resign.stderr.is_empty(),
        "macOS arm64 materializer の再署名成功時 stderr は空であるべき: {:?}",
        String::from_utf8_lossy(&resign.stderr)
    );

    let program = stage_dir.join("program.native");
    assert!(
        program.is_file(),
        "materializer は program.native を生成するべき"
    );
    assert!(
        std::fs::metadata(&program)
            .expect("program.native の metadata 取得に失敗")
            .permissions()
            .mode()
            & 0o111
            != 0,
        "program.native は executable であるべき"
    );
    let execution = Command::new(&program)
        .output()
        .expect("tiny stage program の実行に失敗");
    assert!(
        execution.status.success(),
        "tiny stage program は exit 0 で終了するべき: status={:?}, stdout={}, stderr={}",
        execution.status.code(),
        String::from_utf8_lossy(&execution.stdout),
        String::from_utf8_lossy(&execution.stderr)
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
fn write_release_fixture_launcher(path: &std::path::Path) {
    std::fs::write(
        path,
        r#"#!/usr/bin/env bash
set -euo pipefail
cmd="${1:-}"
case "$cmd" in
  --version)
    echo "lsharp 0.0.0-test"
    ;;
  check)
    echo "type:Int"
    ;;
  test)
    echo "examples:1 invariants:1 failures:0"
    ;;
  fmt)
    cat "${2:?missing source path}"
    ;;
  compile)
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "-o" || "$1" == "--output" ]]; then
        out="$2"
        shift 2
      else
        shift
      fi
    done
    printf '\0asm' > "${out:?missing output path}"
    echo "wasm-size:4"
    ;;
  build)
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "-o" || "$1" == "--output" ]]; then
        out="$2"
        shift 2
      else
        shift
      fi
    done
    printf '\0asm' > "${out:?missing output path}"
    echo "wasm-size:4"
    ;;
  doc)
    json=0
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --json)
          json=1
          shift
          ;;
        -o|--output)
          out="$2"
          shift 2
          ;;
        *)
          shift
          ;;
      esac
    done
    if [[ "$json" == "1" ]]; then
      printf '{"package":"fixture"}\n' > "${out:?missing output path}"
    else
      printf '<html><body>fixture doc</body></html>\n' > "${out:?missing output path}"
    fi
    ;;
  *)
    echo "unsupported command: $cmd" >&2
    exit 1
    ;;
esac
"#,
    )
    .expect("fixture launcher の書き込みに失敗");
    make_executable(path);
}

#[cfg(unix)]
fn current_source_commit(project_root: &std::path::Path) -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .expect("current checkout の source commit 取得に失敗");
    assert!(
        output.status.success(),
        "current checkout の source commit 取得に失敗: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("current checkout の source commit は UTF-8 であるべき")
        .trim()
        .to_owned()
}

#[cfg(unix)]
fn create_stage0_release_fixture(
    temp_root: &std::path::Path,
    source_commit: &str,
) -> std::path::PathBuf {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checksum_script = project_root.join("scripts/checksum.sh");
    let release_dir = temp_root.join("release");
    let archive_root = release_dir.join("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu");
    std::fs::create_dir_all(&archive_root).expect("fixture archive root の作成に失敗");
    write_native_stage0_fixture(
        &archive_root,
        "x86_64-unknown-linux-gnu",
        source_commit,
    );

    let package_checksums = Command::new("bash")
        .arg(&checksum_script)
        .arg(&archive_root)
        .output()
        .expect("checksum.sh package run failed");
    assert!(
        package_checksums.status.success(),
        "package checksum の生成に失敗: stderr={}",
        String::from_utf8_lossy(&package_checksums.stderr)
    );
    std::fs::write(archive_root.join("checksums.txt"), package_checksums.stdout)
        .expect("package checksums.txt 書き込み失敗");

    let archive_path = release_dir.join("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu.tar.gz");
    let tar_output = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-czf")
        .arg(&archive_path)
        .arg("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu")
        .current_dir(&release_dir)
        .output()
        .expect("fixture archive 作成に失敗");
    assert!(
        tar_output.status.success(),
        "fixture archive 作成が失敗した: stderr={}",
        String::from_utf8_lossy(&tar_output.stderr)
    );

    let release_checksums = Command::new("bash")
        .arg(&checksum_script)
        .arg(&release_dir)
        .output()
        .expect("checksum.sh release run failed");
    assert!(
        release_checksums.status.success(),
        "release checksum の生成に失敗: stderr={}",
        String::from_utf8_lossy(&release_checksums.stderr)
    );
    std::fs::write(release_dir.join("checksums.txt"), release_checksums.stdout)
        .expect("release checksums.txt 書き込み失敗");

    release_dir
}

#[cfg(unix)]
#[test]
fn test_e2e_ops07_fetch_stage0_script_fetches_native_stage0_fixture_release_asset() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fetch_script = project_root.join("scripts/fetch-stage0.sh");
    let source_commit = current_source_commit(&project_root);
    let temp_root = ops07_unique_temp_dir("fetch-stage0");
    let release_dir = create_stage0_release_fixture(&temp_root, &source_commit);

    let stage0_dir = temp_root.join("stage0");
    let output = Command::new("bash")
        .arg(&fetch_script)
        .env(
            "STAGE0_RELEASE_BASE_URL",
            format!("file://{}", release_dir.display()),
        )
        .env("STAGE0_VERSION", "v0.0.0-test")
        .env("STAGE0_TARGET", "x86_64-unknown-linux-gnu")
        .env("STAGE0_DIR", &stage0_dir)
        .current_dir(&project_root)
        .output()
        .expect("fetch-stage0.sh の実行に失敗");

    assert!(
        output.status.success(),
        "fetch-stage0.sh が fixture release asset で失敗した: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stage0_dir.join("manifest.json").is_file()
            && stage0_dir.join("bin/compiler").is_file()
            && stage0_dir.join("bin/transport-driver").is_file()
            && stage0_dir.join("bin/materializer").is_file()
            && stage0_dir.join("checksums.txt").is_file(),
        "fetch-stage0.sh は native stage0 manifest と executable payload を stage0/ 配下へ展開するべき"
    );
    let manifest = std::fs::read_to_string(stage0_dir.join("manifest.json"))
        .expect("fetched stage0 manifest の読み込みに失敗");
    assert!(
        manifest.contains(&format!("\"source_commit\": \"{source_commit}\"")),
        "fetched stage0 manifest は source commit provenance を保持するべき"
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
#[test]
fn test_e2e_ops07_fetch_stage0_script_rejects_stale_source_commit_without_replacing_stage0() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fetch_script = project_root.join("scripts/fetch-stage0.sh");
    let temp_root = ops07_unique_temp_dir("fetch-stage0-reject-stale-source");
    let release_dir = create_stage0_release_fixture(
        &temp_root,
        "1111111111111111111111111111111111111111",
    );
    let stage0_dir = temp_root.join("stage0");
    std::fs::create_dir_all(&stage0_dir).expect("existing stage0 の作成に失敗");
    std::fs::write(stage0_dir.join("keep.txt"), "keep existing stage0\n")
        .expect("existing stage0 sentinel の書き込みに失敗");

    let output = Command::new("bash")
        .arg(&fetch_script)
        .env(
            "STAGE0_RELEASE_BASE_URL",
            format!("file://{}", release_dir.display()),
        )
        .env("STAGE0_VERSION", "v0.0.0-test")
        .env("STAGE0_TARGET", "x86_64-unknown-linux-gnu")
        .env("STAGE0_DIR", &stage0_dir)
        .current_dir(&project_root)
        .output()
        .expect("stale source commit fixture fetch-stage0.sh の実行に失敗");

    assert!(
        !output.status.success(),
        "fetch-stage0.sh は current checkout と異なる source_commit を受け入れてはならない"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("source_commit does not match current checkout"),
        "stale source commit の診断は checkout との不一致を示すべき: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(stage0_dir.join("keep.txt"))
            .expect("existing stage0 sentinel の読み込みに失敗"),
        "keep existing stage0\n",
        "stale source commit は既存 stage0 を置換してはならない"
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
#[test]
fn test_e2e_ops07_fetch_stage0_script_rejects_app_cli_archive_without_replacing_stage0() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fetch_script = project_root.join("scripts/fetch-stage0.sh");
    let checksum_script = project_root.join("scripts/checksum.sh");
    let temp_root = ops07_unique_temp_dir("fetch-stage0-reject-app-cli");
    let release_dir = temp_root.join("release");
    let archive_root = release_dir.join("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu");
    std::fs::create_dir_all(&archive_root).expect("reject fixture archive root の作成に失敗");
    std::fs::write(archive_root.join("program.native"), "not a native stage0 package\n")
        .expect("reject fixture program の書き込みに失敗");

    let package_checksums = Command::new("bash")
        .arg(&checksum_script)
        .arg(&archive_root)
        .output()
        .expect("reject fixture package checksum の生成に失敗");
    assert!(
        package_checksums.status.success(),
        "reject fixture package checksum の生成に失敗: stderr={}",
        String::from_utf8_lossy(&package_checksums.stderr)
    );
    std::fs::write(archive_root.join("checksums.txt"), package_checksums.stdout)
        .expect("reject fixture package checksums の書き込みに失敗");

    let archive_path = release_dir.join("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu.tar.gz");
    let tar_output = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-czf")
        .arg(&archive_path)
        .arg("lsharp-stage0-v0.0.0-test-x86_64-unknown-linux-gnu")
        .current_dir(&release_dir)
        .output()
        .expect("reject fixture archive の作成に失敗");
    assert!(
        tar_output.status.success(),
        "reject fixture archive の作成に失敗: stderr={}",
        String::from_utf8_lossy(&tar_output.stderr)
    );

    let release_checksums = Command::new("bash")
        .arg(&checksum_script)
        .arg(&release_dir)
        .output()
        .expect("reject fixture release checksum の生成に失敗");
    assert!(
        release_checksums.status.success(),
        "reject fixture release checksum の生成に失敗: stderr={}",
        String::from_utf8_lossy(&release_checksums.stderr)
    );
    std::fs::write(release_dir.join("checksums.txt"), release_checksums.stdout)
        .expect("reject fixture release checksums の書き込みに失敗");

    let stage0_dir = temp_root.join("stage0");
    std::fs::create_dir_all(&stage0_dir).expect("existing stage0 の作成に失敗");
    std::fs::write(stage0_dir.join("keep.txt"), "keep existing stage0\n")
        .expect("existing stage0 sentinel の書き込みに失敗");
    let output = Command::new("bash")
        .arg(&fetch_script)
        .env(
            "STAGE0_RELEASE_BASE_URL",
            format!("file://{}", release_dir.display()),
        )
        .env("STAGE0_VERSION", "v0.0.0-test")
        .env("STAGE0_TARGET", "x86_64-unknown-linux-gnu")
        .env("STAGE0_DIR", &stage0_dir)
        .current_dir(&project_root)
        .output()
        .expect("reject fixture fetch-stage0.sh の実行に失敗");

    assert!(
        !output.status.success(),
        "fetch-stage0.sh は App.Cli archive を native stage0 として受け入れてはならない"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("native stage0 manifest"),
        "reject 診断は native stage0 manifest を示すべき: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(stage0_dir.join("keep.txt")).expect("existing stage0 sentinel の読み込みに失敗"),
        "keep existing stage0\n",
        "invalid archive は既存 stage0 を置換してはならない"
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
#[test]
fn test_e2e_ops07_bootstrap_script_builds_stage_chain_from_stage0_package() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bootstrap_script = project_root.join("scripts/bootstrap.sh");
    let temp_root = ops07_unique_temp_dir("bootstrap");
    let stage0_dir = temp_root.join("stage0");
    let stage1_dir = temp_root.join("stage1");
    let stage2_dir = temp_root.join("stage2");
    let fixture_src = temp_root.join("EmbeddedCli.ls");
    std::fs::create_dir_all(&stage0_dir).expect("stage0 dir の作成に失敗");
    std::fs::write(&fixture_src, "(defn main [] 42)\n").expect("fixture source の書き込みに失敗");

    std::fs::write(
        stage0_dir.join("lsharp"),
        r#"#!/usr/bin/env bash
set -euo pipefail
cmd="${1:-}"
case "$cmd" in
  compile)
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "-o" || "$1" == "--output" ]]; then
        out="$2"
        shift 2
      else
        shift
      fi
    done
    printf '\0asmstage-component' > "${out:?missing output path}"
    echo "wasm-size:20"
    ;;
  --version)
    echo "fixture-stage0"
    ;;
  *)
    echo "unsupported command: $cmd" >&2
    exit 1
    ;;
esac
"#,
    )
    .expect("fixture stage0 launcher の書き込みに失敗");
    make_executable(&stage0_dir.join("lsharp"));
    std::fs::write(
        stage0_dir.join("lsharp-lsp"),
        "#!/usr/bin/env bash\necho fixture-lsp\n",
    )
    .expect("fixture lsp の書き込みに失敗");
    make_executable(&stage0_dir.join("lsharp-lsp"));
    std::fs::write(
        stage0_dir.join("lsharp.component.wasm"),
        b"\0asmfixture-stage0-component",
    )
    .expect("fixture component の書き込みに失敗");

    let output = Command::new("bash")
        .arg(&bootstrap_script)
        .env("STAGE0_DIR", &stage0_dir)
        .env("STAGE1_DIR", &stage1_dir)
        .env("STAGE2_DIR", &stage2_dir)
        .env("ENTRY_FILE", &fixture_src)
        .current_dir(&project_root)
        .output()
        .expect("bootstrap.sh の実行に失敗");

    assert!(
        output.status.success(),
        "bootstrap.sh が fixture stage0 package で失敗した: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stage1_dir.join("lsharp").is_file()
            && stage1_dir.join("lsharp.component.wasm").is_file()
            && stage2_dir.join("lsharp").is_file()
            && stage2_dir.join("lsharp.component.wasm").is_file(),
        "bootstrap.sh は stage1/stage2 launcher + component を生成するべき"
    );
    assert_eq!(
        std::fs::read(stage1_dir.join("lsharp.component.wasm")).expect("stage1 read failed"),
        std::fs::read(stage2_dir.join("lsharp.component.wasm")).expect("stage2 read failed"),
        "bootstrap.sh は stage1/stage2 component の byte-identical compare を行うべき"
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
#[test]
fn test_e2e_ops07_release_bundle_script_packages_stage_bundle_fixture_archive() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle_script = project_root.join("scripts/release-bundle.sh");
    let release_smoke_script = project_root.join("scripts/ci/release-smoke.sh");
    let temp_root = ops07_unique_temp_dir("release-bundle");
    let stage_dir = temp_root.join("stage2");
    let dist_dir = temp_root.join("dist");
    std::fs::create_dir_all(&stage_dir).expect("stage dir の作成に失敗");

    write_release_fixture_launcher(&stage_dir.join("lsharp"));
    std::fs::write(
        stage_dir.join("lsharp-lsp"),
        "#!/usr/bin/env bash\nif [[ \"${1:-}\" == \"--version\" ]]; then echo 'lsharp-lsp 0.0.0-test'; else echo 'lsharp-lsp help'; fi\n",
    )
    .expect("fixture lsp の書き込みに失敗");
    make_executable(&stage_dir.join("lsharp-lsp"));
    std::fs::write(
        stage_dir.join("lsharp.component.wasm"),
        b"\0asmfixture-component",
    )
    .expect("fixture component の書き込みに失敗");

    let bundle_output = Command::new("bash")
        .arg(&bundle_script)
        .env("STAGE_DIR", &stage_dir)
        .env("DIST_DIR", &dist_dir)
        .env("VERSION", "v0.0.0-test")
        .env("TARGET", "x86_64-unknown-linux-gnu")
        .current_dir(&project_root)
        .output()
        .expect("release-bundle.sh の実行に失敗");

    assert!(
        bundle_output.status.success(),
        "release-bundle.sh が fixture stage bundle で失敗した: status={:?}, stdout={}, stderr={}",
        bundle_output.status.code(),
        String::from_utf8_lossy(&bundle_output.stdout),
        String::from_utf8_lossy(&bundle_output.stderr)
    );

    let archive_path = dist_dir.join("lsharp-v0.0.0-test-x86_64-unknown-linux-gnu.tar.gz");
    assert!(
        dist_dir.join("lsharp").is_file()
            && dist_dir.join("lsharp.component.wasm").is_file()
            && dist_dir
                .join("lsharp-v0.0.0-test-x86_64-unknown-linux-gnu/manifest.json")
                .is_file()
            && archive_path.is_file(),
        "release-bundle.sh は rollback manifest を含む dist/lsharp と archive を生成するべき"
    );

    let smoke_output = Command::new("bash")
        .arg(&release_smoke_script)
        .arg(&archive_path)
        .env("WORK_DIR", temp_root.join("release-smoke"))
        .current_dir(&project_root)
        .output()
        .expect("release-smoke.sh の実行に失敗");

    assert!(
        smoke_output.status.success(),
        "release-bundle archive は release-smoke.sh を通るべき: status={:?}, stdout={}, stderr={}",
        smoke_output.status.code(),
        String::from_utf8_lossy(&smoke_output.stdout),
        String::from_utf8_lossy(&smoke_output.stderr)
    );

    std::fs::remove_dir_all(&temp_root).ok();
}

#[cfg(unix)]
fn write_dev_runner_fixture_launcher(path: &std::path::Path) {
    std::fs::write(
        path,
        r#"#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LSHARP_PATH:-}" || -n "${LSHARP_DISABLE_EMBEDDED_COMPONENT:-}" ]]; then
  echo "fixture launcher received host delegation environment" >&2
  exit 97
fi

log_path="${INVOCATION_LOG:?missing invocation log}"
printf '%s' "$(basename "$(dirname "$0")")" >> "$log_path"
for arg in "$@"; do
  printf '|%s' "$arg" >> "$log_path"
done
printf '\n' >> "$log_path"

case "${1:-}" in
  compile)
    out=""
    shift
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "-o" || "$1" == "--output" ]]; then
        out="$2"
        shift 2
      else
        shift
      fi
    done
    printf '\0asmfixture-dev-component' > "${out:?missing output path}"
    ;;
  *)
    printf 'delegated:'
    for arg in "$@"; do
      printf ' %s' "$arg"
    done
    printf '\n'
    ;;
esac
"#,
    )
    .expect("dev runner fixture launcher の書き込みに失敗");
    make_executable(path);
}

#[test]
fn test_e2e_v2_16a_selfhost_dev_runner_has_no_cargo_reference() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runner_script = project_root.join("scripts/selfhost-dev.sh");
    let runner_contents =
        std::fs::read_to_string(&runner_script).expect("selfhost-dev.sh の読み込みに失敗");

    assert!(
        !runner_contents.contains("cargo"),
        "V2-16a: selfhost-dev.sh は cargo を含まず、stage2/lsharp へ委譲するべき"
    );
}

#[cfg(unix)]
#[test]
fn test_e2e_selfhost_dev_runner_bootstraps_reuses_stage2_and_forces_rebuild() {
    use std::process::Command;

    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runner_script = project_root.join("scripts/selfhost-dev.sh");
    let temp_root = ops07_unique_temp_dir("selfhost-dev");
    let stage0_dir = temp_root.join("stage0");
    let stage1_dir = temp_root.join("stage1");
    let stage2_dir = temp_root.join("stage2");
    let entry_file = temp_root.join("EmbeddedCli.ls");
    let command_source = temp_root.join("sample.ls");
    let invocation_log = temp_root.join("invocations.log");
    let stage2_stamp = stage2_dir.join(".selfhost-dev-source.sha256");

    std::fs::create_dir_all(&stage0_dir).expect("stage0 dir の作成に失敗");
    std::fs::write(&entry_file, "(defn main [] 42)\n").expect("entry fixture の書き込みに失敗");
    std::fs::write(&command_source, "(defn answer [] 42)\n")
        .expect("command fixture の書き込みに失敗");
    write_dev_runner_fixture_launcher(&stage0_dir.join("lsharp"));
    std::fs::write(
        stage0_dir.join("lsharp.component.wasm"),
        b"\0asmfixture-stage0-component",
    )
    .expect("stage0 component fixture の書き込みに失敗");

    let first_run = Command::new("bash")
        .arg(&runner_script)
        .arg("check")
        .arg(&command_source)
        .arg("--fixture-option")
        .arg("value")
        .env("STAGE0_DIR", &stage0_dir)
        .env("STAGE1_DIR", &stage1_dir)
        .env("STAGE2_DIR", &stage2_dir)
        .env("ENTRY_FILE", &entry_file)
        .env("LSHARP_PATH", "/misleading/external/lsharp")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .env("INVOCATION_LOG", &invocation_log)
        .current_dir(&project_root)
        .output()
        .expect("selfhost-dev.sh 初回実行に失敗");

    assert!(
        first_run.status.success(),
        "初回 selfhost-dev.sh が失敗した: status={:?}, stdout={}, stderr={}",
        first_run.status.code(),
        String::from_utf8_lossy(&first_run.stdout),
        String::from_utf8_lossy(&first_run.stderr)
    );
    assert!(
        stage1_dir.join("lsharp").is_file()
            && stage1_dir.join("lsharp.component.wasm").is_file()
            && stage2_dir.join("lsharp").is_file()
            && stage2_dir.join("lsharp.component.wasm").is_file(),
        "初回 runner は bootstrap して stage1/stage2 bundle を生成するべき"
    );
    assert!(
        stage2_stamp.is_file(),
        "初回 bootstrap は stage2 配下に source fingerprint stamp を保存するべき"
    );
    let first_stamp = std::fs::read_to_string(&stage2_stamp).expect("初回 stamp 読み込み失敗");
    assert!(
        String::from_utf8_lossy(&first_run.stdout).contains(&format!(
            "delegated: check {} --fixture-option value",
            command_source.display()
        )),
        "初回 runner は command argv を stage2/lsharp へ委譲するべき"
    );

    let first_log =
        std::fs::read_to_string(&invocation_log).expect("初回 invocation log 読み込み失敗");
    assert_eq!(
        first_log
            .lines()
            .filter(|line| line.starts_with("stage0|compile|"))
            .count(),
        1,
        "初回は stage0 launcher で 1 回 compile するべき: {first_log}"
    );
    assert_eq!(
        first_log
            .lines()
            .filter(|line| line.starts_with("stage1|compile|"))
            .count(),
        1,
        "初回は stage1 launcher で 1 回 compile するべき: {first_log}"
    );
    assert!(
        first_log.contains(&format!(
            "stage2|check|{}|--fixture-option|value",
            command_source.display()
        )),
        "初回は生成済み stage2 launcher に command を渡すべき: {first_log}"
    );

    let second_run = Command::new("bash")
        .arg(&runner_script)
        .arg("--stage0-dir")
        .arg(&stage0_dir)
        .arg("--stage1-dir")
        .arg(&stage1_dir)
        .arg("--stage2-dir")
        .arg(&stage2_dir)
        .arg("--entry-file")
        .arg(&entry_file)
        .arg("test")
        .arg(&command_source)
        .arg("--filter")
        .arg("smoke")
        .env("LSHARP_PATH", "/misleading/external/lsharp")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .env("INVOCATION_LOG", &invocation_log)
        .current_dir(&project_root)
        .output()
        .expect("selfhost-dev.sh 再利用実行に失敗");

    assert!(
        second_run.status.success(),
        "再利用 selfhost-dev.sh が失敗した: status={:?}, stdout={}, stderr={}",
        second_run.status.code(),
        String::from_utf8_lossy(&second_run.stdout),
        String::from_utf8_lossy(&second_run.stderr)
    );
    let second_log =
        std::fs::read_to_string(&invocation_log).expect("再利用 invocation log 読み込み失敗");
    assert_eq!(
        second_log
            .lines()
            .filter(|line| line.contains("|compile|"))
            .count(),
        2,
        "完成済み stage2 は bootstrap を再実行しないべき: {second_log}"
    );
    assert!(
        second_log.contains(&format!(
            "stage2|test|{}|--filter|smoke",
            command_source.display()
        )),
        "再利用時も command argv を stage2/lsharp へ委譲するべき: {second_log}"
    );
    assert_eq!(
        std::fs::read_to_string(&stage2_stamp).expect("再利用 stamp 読み込み失敗"),
        first_stamp,
        "source が不変なら stage2 stamp は変化しないべき"
    );

    std::fs::write(&entry_file, "(defn main [] 43)\n")
        .expect("変更後 entry fixture の書き込みに失敗");
    let refreshed_run = Command::new("bash")
        .arg(&runner_script)
        .arg("--stage0-dir")
        .arg(&stage0_dir)
        .arg("--stage1-dir")
        .arg(&stage1_dir)
        .arg("--stage2-dir")
        .arg(&stage2_dir)
        .arg("--entry-file")
        .arg(&entry_file)
        .arg("fmt")
        .arg(&command_source)
        .env("LSHARP_PATH", "/misleading/external/lsharp")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .env("INVOCATION_LOG", &invocation_log)
        .current_dir(&project_root)
        .output()
        .expect("ENTRY_FILE 変更後 selfhost-dev.sh 実行に失敗");

    assert!(
        refreshed_run.status.success(),
        "ENTRY_FILE 変更後 selfhost-dev.sh が失敗した: status={:?}, stdout={}, stderr={}",
        refreshed_run.status.code(),
        String::from_utf8_lossy(&refreshed_run.stdout),
        String::from_utf8_lossy(&refreshed_run.stderr)
    );
    let refreshed_log =
        std::fs::read_to_string(&invocation_log).expect("更新後 invocation log 読み込み失敗");
    assert_eq!(
        refreshed_log
            .lines()
            .filter(|line| line.contains("|compile|"))
            .count(),
        4,
        "ENTRY_FILE の fingerprint が変化したら --bootstrap なしでも再生成するべき: {refreshed_log}"
    );
    let refreshed_stamp =
        std::fs::read_to_string(&stage2_stamp).expect("更新後 stamp 読み込み失敗");
    assert_ne!(
        refreshed_stamp, first_stamp,
        "ENTRY_FILE の変更は stage2 stamp を更新するべき"
    );
    assert!(
        refreshed_log.contains(&format!("stage2|fmt|{}", command_source.display())),
        "再生成後も stage2 launcher に command を渡すべき: {refreshed_log}"
    );

    let forced_run = Command::new("bash")
        .arg(&runner_script)
        .arg("--bootstrap")
        .arg("--stage0-dir")
        .arg(&stage0_dir)
        .arg("--stage1-dir")
        .arg(&stage1_dir)
        .arg("--stage2-dir")
        .arg(&stage2_dir)
        .arg("--entry-file")
        .arg(&entry_file)
        .arg("doc")
        .arg(&command_source)
        .env("LSHARP_PATH", "/misleading/external/lsharp")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .env("INVOCATION_LOG", &invocation_log)
        .current_dir(&project_root)
        .output()
        .expect("selfhost-dev.sh forced bootstrap 実行に失敗");

    assert!(
        forced_run.status.success(),
        "--bootstrap selfhost-dev.sh が失敗した: status={:?}, stdout={}, stderr={}",
        forced_run.status.code(),
        String::from_utf8_lossy(&forced_run.stdout),
        String::from_utf8_lossy(&forced_run.stderr)
    );
    let forced_log =
        std::fs::read_to_string(&invocation_log).expect("forced invocation log 読み込み失敗");
    assert_eq!(
        forced_log
            .lines()
            .filter(|line| line.contains("|compile|"))
            .count(),
        6,
        "--bootstrap は完成済み stage2 でも bootstrap を再実行するべき: {forced_log}"
    );
    assert!(
        forced_log.contains(&format!("stage2|doc|{}", command_source.display())),
        "--bootstrap 後も再生成済み stage2 launcher に command を渡すべき: {forced_log}"
    );

    let help_run = Command::new("bash")
        .arg(&runner_script)
        .arg("--help")
        .current_dir(&project_root)
        .output()
        .expect("selfhost-dev.sh --help 実行に失敗");
    assert!(help_run.status.success(), "--help は成功するべき");
    assert!(
        String::from_utf8_lossy(&help_run.stdout).contains("usage:"),
        "--help は usage を表示するべき"
    );

    let missing_command = Command::new("bash")
        .arg(&runner_script)
        .current_dir(&project_root)
        .output()
        .expect("selfhost-dev.sh command なし実行に失敗");
    assert!(
        !missing_command.status.success(),
        "command なしは失敗するべき"
    );
    assert!(
        String::from_utf8_lossy(&missing_command.stderr).contains("usage:"),
        "command なしは concise usage を stderr に表示するべき"
    );

    std::fs::remove_dir_all(&temp_root).ok();
}
