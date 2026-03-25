use std::fs;
use std::path::Path;

fn assert_missing(path: &Path, forbidden: &str) {
    let contents = fs::read_to_string(path).expect("read source");
    let production = contents.split("#[cfg(test)]").next().unwrap_or(&contents);
    assert!(
        !production.contains(forbidden),
        "{} must not import {}",
        path.display(),
        forbidden
    );
}

#[test]
fn use_cases_do_not_depend_on_cli_or_output_types() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("use_cases");
    let files = [
        root.join("build_project.rs"),
        root.join("check_syntax.rs"),
        root.join("dump_config.rs"),
        root.join("init_project.rs"),
        root.join("launch_app.rs"),
        root.join("run_tests.rs"),
    ];

    for file in &files {
        assert_missing(file, "crate::cli::args");
        assert_missing(file, "crate::output::presenter::Presenter");
        assert_missing(file, "crate::output::json::Envelope");
    }
}
