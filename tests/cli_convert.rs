#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[derive(Clone, Copy)]
struct SourceSetSpec<'a> {
    name: &'a str,
    kind: &'a str,
    path: &'a str,
}

fn make_executable(path: &Path) {
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn write_script(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write");
    make_executable(path);
}

fn write_edt_script(path: &Path, calls_log: &Path) {
    let body = format!(
        "args=\"$*\"\n\
printf '%s\\n' \"$args\" >> \"{}\"\n\
mode=\"\"\n\
project=\"\"\n\
config_files=\"\"\n\
base_project_name=\"\"\n\
project_name=\"\"\n\
source_name=\"\"\n\
prev=\"\"\n\
for arg in \"$@\"; do\n\
  if [ \"$prev\" = \"-command\" ]; then mode=\"$arg\"; fi\n\
  if [ \"$prev\" = \"--project\" ]; then project=\"$arg\"; fi\n\
  if [ \"$prev\" = \"--configuration-files\" ]; then config_files=\"$arg\"; fi\n\
  if [ \"$prev\" = \"--base-project-name\" ]; then base_project_name=\"$arg\"; fi\n\
  project_name=$(basename \"$project\")\n\
  source_name=$(basename \"$config_files\")\n\
  prev=\"$arg\"\n\
done\n\
case \"$mode\" in\n\
  export)\n\
    mkdir -p \"$config_files\"\n\
    rm -rf \"$config_files\"/*\n\
    if printf '%s' \"$project_name\" | grep -q '^processor-'; then\n\
      printf '<ExternalDataProcessor><Properties><Name>%s</Name></Properties></ExternalDataProcessor>\\n' \"$project_name\" > \"$config_files/$project_name.xml\"\n\
    else\n\
      printf '<Configuration />\\n' > \"$config_files/Configuration.xml\"\n\
    fi\n\
    ;;\n\
  import)\n\
    if [ -f \"$config_files/Configuration.xml\" ]; then\n\
      mkdir -p \"$project\"\n\
      case \"$source_name\" in\n\
        main)\n\
          imported_name=\"BaseProject\"\n\
          ;;\n\
        ext-sales)\n\
          if [ \"$base_project_name\" != \"BaseProject\" ]; then\n\
            printf 'unexpected base project: %s\\n' \"$base_project_name\" >&2\n\
            exit 23\n\
          fi\n\
          imported_name=\"SalesExtension\"\n\
          ;;\n\
        *)\n\
          imported_name=\"Imported\"\n\
          ;;\n\
      esac\n\
      printf '<projectDescription><name>%s</name></projectDescription>\\n' \"$imported_name\" > \"$project/.project\"\n\
    else\n\
      mkdir -p \"$project\"\n\
      for descriptor in \"$config_files\"/*.xml; do\n\
        if [ ! -f \"$descriptor\" ]; then continue; fi\n\
        descriptor_name=$(basename \"$descriptor\" .xml)\n\
        mkdir -p \"$project/$descriptor_name\"\n\
        printf '<projectDescription><name>%s</name></projectDescription>\\n' \"$descriptor_name\" > \"$project/$descriptor_name/.project\"\n\
      done\n\
    fi\n\
    ;;\n\
esac\n\
exit 0",
        calls_log.display()
    );
    write_script(path, &body);
}

fn write_config(
    path: &Path,
    base_path: &Path,
    work_path: &Path,
    edt_path: &Path,
    format: &str,
    source_sets: &[SourceSetSpec<'_>],
    platform_version: Option<&str>,
) {
    let mut config = format!(
        "basePath: '{}'\nworkPath: '{}'\nformat: {format}\nbuilder: DESIGNER\ninfobase:\n  connection: 'File=/tmp/ib'\nsource-set:\n",
        base_path.display(),
        work_path.display(),
    );
    for source_set in source_sets {
        config.push_str(&format!(
            "  - name: {}\n    type: {}\n    path: {}\n",
            source_set.name, source_set.kind, source_set.path
        ));
    }
    config.push_str("tools:\n");
    if let Some(version) = platform_version {
        config.push_str(&format!("  platform:\n    version: '{version}'\n"));
    }
    config.push_str(&format!(
        "  edt_cli:\n    path: '{}'\n    interactive-mode: false\n",
        edt_path.display()
    ));
    fs::write(path, config).expect("config");
}

fn write_live_workspace_lock(work_path: &Path, command: &str) {
    let canonical_work = fs::canonicalize(work_path).expect("canonical work");
    let lock_owner = "integration-test-lock-owner";
    let started_at = chrono::Utc::now().to_rfc3339();

    fs::write(
        canonical_work.join(".v8-runner.workspace.lock"),
        serde_json::json!({
            "tool": "v8-runner",
            "pid": std::process::id(),
            "owner_id": lock_owner,
            "created_at": started_at,
        })
        .to_string(),
    )
    .expect("workspace lock");
    fs::write(
        canonical_work.join(".v8-runner.workspace.lock.json"),
        serde_json::json!({
            "pid": std::process::id(),
            "lock_owner": lock_owner,
            "command": command,
            "started_at": started_at,
            "canonical_work_path": canonical_work,
        })
        .to_string(),
    )
    .expect("workspace lock sidecar");
}

fn setup_project() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
) {
    let dir = tempdir().expect("tempdir");
    let base_path = dir.path().join("project");
    let work_path = dir.path().join("work");
    let config_path = dir.path().join("v8project.yaml");
    let edt_cli_path = dir.path().join("edt").join("1cedtcli");
    let calls_log = dir.path().join("edt-calls.log");

    fs::create_dir_all(&base_path).expect("base");
    fs::create_dir_all(&work_path).expect("work");
    write_edt_script(&edt_cli_path, &calls_log);

    (
        dir,
        config_path,
        base_path,
        work_path,
        edt_cli_path,
        calls_log,
    )
}

fn write_designer_source(path: &Path) {
    fs::create_dir_all(path).expect("designer source");
    fs::write(path.join("Configuration.xml"), "<Configuration />\n").expect("xml");
}

fn write_designer_external_source(path: &Path, names: &[&str]) {
    fs::create_dir_all(path).expect("designer external source");
    for name in names {
        fs::write(
            path.join(format!("{name}.xml")),
            format!(
                "<ExternalDataProcessor><Properties><Name>{name}</Name></Properties></ExternalDataProcessor>\n"
            ),
        )
        .expect("xml");
    }
}

fn write_edt_source(path: &Path, name: &str) {
    fs::create_dir_all(path).expect("edt source");
    fs::write(
        path.join(".project"),
        format!("<projectDescription><name>{name}</name></projectDescription>\n"),
    )
    .expect("project");
}

#[test]
fn convert_without_source_set_processes_all_source_sets_into_work_path_out() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "DESIGNER",
        &[
            SourceSetSpec {
                name: "main",
                kind: "CONFIGURATION",
                path: "main",
            },
            SourceSetSpec {
                name: "ext-sales",
                kind: "EXTENSION",
                path: "ext-sales",
            },
        ],
        Some("8.3.24"),
    );
    write_designer_source(&base_path.join("main"));
    write_designer_source(&base_path.join("ext-sales"));
    let stale_output = work_path.join("convert/out/main/edt/stale.txt");
    fs::create_dir_all(stale_output.parent().expect("parent")).expect("stale dir");
    fs::write(&stale_output, "stale").expect("stale file");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "convert");
    assert_eq!(payload["data"]["direction"], "DESIGNER_TO_EDT");
    assert_eq!(payload["data"]["scope"], "ALL");
    assert_eq!(
        payload["data"]["outputs"]
            .as_array()
            .expect("outputs")
            .len(),
        2
    );
    assert_eq!(payload["data"]["outputs"][0]["source_set"], "main");
    assert_eq!(payload["data"]["outputs"][1]["source_set"], "ext-sales");

    let main_target = work_path.join("convert/out/main/edt");
    let extension_target = work_path.join("convert/out/ext-sales/edt");
    assert!(main_target.join(".project").exists());
    assert!(extension_target.join(".project").exists());
    assert!(!stale_output.exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert_eq!(calls.matches("-command import").count(), 2);
    assert!(calls.contains("--version 8.3.24"));
    assert!(calls.contains("--base-project-name BaseProject"));
    assert!(!calls.contains("--build true"));
}

#[test]
fn convert_single_source_set_uses_inferred_edt_to_designer_direction() {
    let (_dir, config_path, base_path, work_path, _edt_cli_path, calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &_edt_cli_path,
        "EDT",
        &[
            SourceSetSpec {
                name: "main",
                kind: "CONFIGURATION",
                path: "main",
            },
            SourceSetSpec {
                name: "ext-sales",
                kind: "EXTENSION",
                path: "ext-sales",
            },
        ],
        None,
    );
    write_edt_source(&base_path.join("main"), "MainConfiguration");
    write_edt_source(&base_path.join("ext-sales"), "SalesExtension");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--no-color",
            "convert",
            "--source-set",
            "main",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Convert completed successfully"));
    assert!(stdout.contains("direction: edt-to-designer"));
    assert!(stdout.contains("scope: source-set main"));
    assert!(stdout.contains(
        work_path
            .join("convert/out/main/designer")
            .display()
            .to_string()
            .as_str()
    ));

    let target = work_path.join("convert/out/main/designer");
    assert!(target.join("Configuration.xml").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert_eq!(calls.matches("-command export").count(), 1);
    assert!(calls.contains(base_path.join("main").display().to_string().as_str()));
    assert!(!calls.contains(base_path.join("ext-sales").display().to_string().as_str()));
}

#[test]
fn convert_single_extension_source_set_infers_base_project_name_from_configuration_source() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "DESIGNER",
        &[
            SourceSetSpec {
                name: "main",
                kind: "CONFIGURATION",
                path: "main",
            },
            SourceSetSpec {
                name: "ext-sales",
                kind: "EXTENSION",
                path: "ext-sales",
            },
        ],
        Some("8.3.24"),
    );
    write_designer_source(&base_path.join("main"));
    write_designer_source(&base_path.join("ext-sales"));

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
            "--source-set",
            "ext-sales",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "convert");
    assert_eq!(payload["data"]["direction"], "DESIGNER_TO_EDT");
    assert_eq!(payload["data"]["scope"], "SINGLE");
    assert_eq!(payload["data"]["source_set"], "ext-sales");

    let target = work_path.join("convert/out/ext-sales/edt");
    assert!(target.join(".project").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert_eq!(calls.matches("-command import").count(), 2);
    assert!(calls.contains(base_path.join("main").display().to_string().as_str()));
    assert!(calls.contains("--base-project-name BaseProject"));
    assert!(calls.contains("--version 8.3.24"));
}

#[test]
fn convert_unknown_source_set_json_keeps_convert_command_identity_before_workspace_lock() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, _calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "DESIGNER",
        &[SourceSetSpec {
            name: "main",
            kind: "CONFIGURATION",
            path: "main",
        }],
        None,
    );
    write_designer_source(&base_path.join("main"));
    write_live_workspace_lock(&work_path, "convert");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
            "--source-set",
            "missing",
        ])
        .output()
        .expect("run convert");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["command"], "convert");
    assert!(payload["data"]["message"]
        .as_str()
        .expect("message")
        .contains("unknown source-set 'missing'"));
}

#[test]
fn convert_workspace_lock_conflict_uses_runtime_error_after_valid_preflight() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, _calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "DESIGNER",
        &[SourceSetSpec {
            name: "main",
            kind: "CONFIGURATION",
            path: "main",
        }],
        None,
    );
    write_designer_source(&base_path.join("main"));
    write_live_workspace_lock(&work_path, "convert");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--no-color",
            "convert",
        ])
        .output()
        .expect("run convert");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR: runtime error: cannot start convert"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn convert_external_edt_source_set_preserves_all_exported_descriptors() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "EDT",
        &[SourceSetSpec {
            name: "processors",
            kind: "EXTERNAL_DATA_PROCESSORS",
            path: "processors",
        }],
        None,
    );
    write_edt_source(&base_path.join("processors/processor-a"), "ProcessorA");
    write_edt_source(&base_path.join("processors/processor-b"), "ProcessorB");

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
            "--source-set",
            "processors",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "convert");
    assert_eq!(payload["data"]["direction"], "EDT_TO_DESIGNER");
    assert_eq!(payload["data"]["scope"], "SINGLE");

    let target = work_path.join("convert/out/processors/designer");
    assert!(target.join("processor-a.xml").exists());
    assert!(target.join("processor-b.xml").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert_eq!(calls.matches("-command export").count(), 2);
}

#[test]
fn convert_external_designer_source_set_does_not_require_configuration_source_set() {
    let (_dir, config_path, base_path, work_path, edt_cli_path, calls_log) = setup_project();
    write_config(
        &config_path,
        &base_path,
        &work_path,
        &edt_cli_path,
        "DESIGNER",
        &[SourceSetSpec {
            name: "processors",
            kind: "EXTERNAL_DATA_PROCESSORS",
            path: "processors",
        }],
        None,
    );
    write_designer_external_source(
        &base_path.join("processors"),
        &["processor-a", "processor-b"],
    );

    let output = std::process::Command::cargo_bin("v8-runner")
        .expect("binary")
        .args([
            "--config",
            &config_path.display().to_string(),
            "--output",
            "json",
            "convert",
        ])
        .output()
        .expect("run convert");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "convert");
    assert_eq!(payload["data"]["direction"], "DESIGNER_TO_EDT");
    assert_eq!(payload["data"]["scope"], "ALL");

    let target = work_path.join("convert/out/processors/edt");
    assert!(target.join("processor-a/.project").exists());
    assert!(target.join("processor-b/.project").exists());

    let calls = fs::read_to_string(calls_log).expect("calls");
    assert_eq!(calls.matches("-command import").count(), 1);
    assert!(!calls.contains("--base-project-name"));
}
