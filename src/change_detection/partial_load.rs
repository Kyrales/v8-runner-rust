use std::path::{Path, PathBuf};

use crate::change_detection::analyzer::{ChangeKind, FileChange};

/// Default maximum number of changed files before forcing a full load.
#[cfg(test)]
pub const DEFAULT_PARTIAL_LOAD_THRESHOLD: usize = 20;

/// The name of the root configuration descriptor — if changed, partial load is forbidden.
const CONFIGURATION_XML: &str = "Configuration.xml";

/// Decision made by [`decide`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadDecision {
    /// Load only the listed files.
    Partial(Vec<PathBuf>),
    /// Load the entire source-set directory.
    Full,
}

/// Decide whether a partial or full load is appropriate for `changes`.
pub fn decide(changes: &[FileChange], source_root: &Path, threshold: usize) -> LoadDecision {
    if threshold == 0 {
        return LoadDecision::Full;
    }

    if changes
        .iter()
        .any(|change| is_configuration_xml(&change.path))
    {
        return LoadDecision::Full;
    }

    if changes
        .iter()
        .any(|change| change.kind == ChangeKind::Deleted)
    {
        return LoadDecision::Full;
    }

    let Some(expanded) = expand_files(changes, source_root) else {
        return LoadDecision::Full;
    };

    if expanded.is_empty() || expanded.len() > threshold {
        LoadDecision::Full
    } else {
        LoadDecision::Partial(expanded)
    }
}

/// Write a partial-load list file (UTF-8, one path per line, no empty lines).
///
/// Paths are written relative to `source_root` as required by Designer's
/// `-listFile` parameter when running in agent mode.
pub fn write_list_file(paths: &[PathBuf], source_root: &Path, dest: &Path) -> std::io::Result<()> {
    let rel_paths = relative_paths(paths, source_root)?;
    let lines = rel_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    std::fs::write(dest, lines.join("\r\n"))
}

/// Convert safe absolute paths into relative paths under `source_root`.
pub fn relative_paths(paths: &[PathBuf], source_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let root_real = canonicalize_existing(source_root).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "source root does not exist or is not canonicalizable: {}",
                source_root.display()
            ),
        )
    })?;
    let mut rel_paths = Vec::new();

    for path in paths {
        let rel = safe_relative_path(path, source_root, &root_real).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "path cannot be safely represented in partial list: {}",
                    path.display()
                ),
            )
        })?;
        if !rel.as_os_str().is_empty() {
            rel_paths.push(rel);
        }
    }

    Ok(rel_paths)
}

fn expand_files(changes: &[FileChange], source_root: &Path) -> Option<Vec<PathBuf>> {
    let root_real = canonicalize_existing(source_root)?;
    let mut paths = Vec::new();

    for change in changes {
        push_if_safe(&mut paths, &change.path, source_root, &root_real)?;

        if is_bsl(&change.path) {
            if let Some(xml) = sibling_xml(&change.path) {
                push_if_safe_if_exists(&mut paths, &xml, source_root, &root_real)?;
            }

            if let Some(object_dir) = object_dir(&change.path, source_root) {
                push_if_safe_if_exists(&mut paths, &object_dir, source_root, &root_real)?;
            }
        }
    }

    paths.sort();
    paths.dedup();
    Some(paths)
}

fn push_if_safe(
    paths: &mut Vec<PathBuf>,
    candidate: &Path,
    source_root: &Path,
    root_real: &Path,
) -> Option<()> {
    let relative = safe_relative_path(candidate, source_root, root_real)?;
    paths.push(source_root.join(relative));
    Some(())
}

fn push_if_safe_if_exists(
    paths: &mut Vec<PathBuf>,
    candidate: &Path,
    source_root: &Path,
    root_real: &Path,
) -> Option<()> {
    if !candidate.exists() {
        return Some(());
    }

    push_if_safe(paths, candidate, source_root, root_real)
}

fn safe_relative_path(path: &Path, source_root: &Path, root_real: &Path) -> Option<PathBuf> {
    let candidate_real = canonicalize_existing(path)?;
    if !candidate_real.starts_with(root_real) {
        return None;
    }

    if let Ok(relative) = path.strip_prefix(source_root) {
        return Some(relative.to_path_buf());
    }

    candidate_real
        .strip_prefix(root_real)
        .ok()
        .map(Path::to_path_buf)
}

fn canonicalize_existing(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

fn is_configuration_xml(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name == CONFIGURATION_XML)
        .unwrap_or(false)
}

fn is_bsl(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bsl"))
        .unwrap_or(false)
}

/// Return the XML descriptor alongside a `.bsl` file (same name, `.xml` ext).
fn sibling_xml(bsl: &Path) -> Option<PathBuf> {
    let parent = bsl.parent()?;
    let stem = bsl.file_stem()?.to_str()?;
    Some(parent.join(format!("{stem}.xml")))
}

/// Return the object directory that owns a `.bsl` module.
fn object_dir(bsl: &Path, source_root: &Path) -> Option<PathBuf> {
    let parent = bsl.parent()?;
    let relative = bsl.strip_prefix(source_root).ok();
    let is_nested_module = bsl
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.eq_ignore_ascii_case("Module.bsl"))
        .unwrap_or(false)
        && relative
            .map(|path| path.components().count() >= 4)
            .unwrap_or(false);

    if is_nested_module {
        return parent.parent()?.parent().map(Path::to_path_buf);
    }

    Some(parent.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{
        decide, object_dir, relative_paths, write_list_file, LoadDecision,
        DEFAULT_PARTIAL_LOAD_THRESHOLD,
    };
    use crate::change_detection::analyzer::{ChangeKind, FileChange};

    #[test]
    fn object_dir_uses_parent_for_top_level_modules() {
        let bsl = Path::new("/tmp/src/Catalogs.Items/ObjectModule.bsl");

        assert_eq!(
            object_dir(bsl, Path::new("/tmp/src")),
            Some(PathBuf::from("/tmp/src/Catalogs.Items"))
        );
    }

    #[test]
    fn object_dir_uses_owning_object_for_nested_modules() {
        let bsl = Path::new("/tmp/src/Catalogs.Items/Forms/Form1/Module.bsl");

        assert_eq!(
            object_dir(bsl, Path::new("/tmp/src")),
            Some(PathBuf::from("/tmp/src/Catalogs.Items"))
        );
    }

    #[test]
    fn write_list_file_skips_empty_relative_paths() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let list_file = root.join("partial.lst");

        write_list_file(&[root.to_path_buf()], root, &list_file).expect("write list");

        assert_eq!(std::fs::read_to_string(list_file).expect("read list"), "");
    }

    #[test]
    fn relative_paths_returns_relative_entries_for_safe_paths() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let nested = root.join("Catalogs.Items");
        std::fs::create_dir_all(&nested).expect("mkdir");
        let file = nested.join("ObjectModule.bsl");
        std::fs::write(&file, "module").expect("write");

        let rels = relative_paths(&[file.clone()], root).expect("relative paths");

        assert_eq!(rels, vec![PathBuf::from("Catalogs.Items/ObjectModule.bsl")]);
    }

    #[cfg(unix)]
    #[test]
    fn relative_paths_rejects_path_outside_root() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("src");
        let outside = temp.path().join("outside");
        let link = root.join("Catalogs.Items");
        let escaped = outside.join("ObjectModule.bsl");

        std::fs::create_dir_all(&root).expect("root");
        std::fs::create_dir_all(&outside).expect("outside");
        std::fs::write(&escaped, "module").expect("escaped");
        symlink(&outside, &link).expect("link");

        let err =
            relative_paths(&[link.join("ObjectModule.bsl")], &root).expect_err("expected error");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[cfg(unix)]
    #[test]
    fn write_list_file_fails_for_paths_outside_root() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("src");
        let outside = temp.path().join("outside");
        let link = root.join("Catalogs.Items");
        let escaped = outside.join("ObjectModule.bsl");
        let list_file = temp.path().join("partial.lst");

        std::fs::create_dir_all(&root).expect("root");
        std::fs::create_dir_all(&outside).expect("outside");
        std::fs::write(&escaped, "module").expect("escaped");
        symlink(&outside, &link).expect("link");

        let err = write_list_file(&[link.join("ObjectModule.bsl")], &root, &list_file)
            .expect_err("expected invalid path");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn decide_expands_bsl_to_xml_and_object_dir() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let object_dir = root.join("Catalogs.Items");
        let module = object_dir.join("ObjectModule.bsl");
        let xml = object_dir.join("ObjectModule.xml");

        std::fs::create_dir_all(&object_dir).expect("create object dir");
        std::fs::write(&module, "module").expect("write module");
        std::fs::write(&xml, "<xml />").expect("write xml");

        let decision = decide(
            &[FileChange {
                path: module.clone(),
                kind: ChangeKind::Modified,
            }],
            root,
            DEFAULT_PARTIAL_LOAD_THRESHOLD,
        );

        assert_eq!(
            decision,
            LoadDecision::Partial(vec![object_dir, module, xml])
        );
    }

    #[test]
    fn decide_forces_full_when_configuration_xml_changed() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let config_xml = root.join("Configuration.xml");
        std::fs::write(&config_xml, "<xml />").expect("write config");

        let decision = decide(
            &[FileChange {
                path: config_xml,
                kind: ChangeKind::Modified,
            }],
            root,
            DEFAULT_PARTIAL_LOAD_THRESHOLD,
        );

        assert_eq!(decision, LoadDecision::Full);
    }

    #[test]
    fn decide_forces_full_when_deleted_files_exist() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let removed = root.join("Catalogs.Items").join("ObjectModule.bsl");

        let decision = decide(
            &[FileChange {
                path: removed,
                kind: ChangeKind::Deleted,
            }],
            root,
            DEFAULT_PARTIAL_LOAD_THRESHOLD,
        );

        assert_eq!(decision, LoadDecision::Full);
    }

    #[test]
    fn decide_forces_full_when_threshold_is_exceeded() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        let mut changes = Vec::new();

        for index in 0..=DEFAULT_PARTIAL_LOAD_THRESHOLD {
            let path = root.join(format!("CommonModules/Module{index}.bsl"));
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, "module").expect("write");
            changes.push(FileChange {
                path,
                kind: ChangeKind::Modified,
            });
        }

        let decision = decide(&changes, root, DEFAULT_PARTIAL_LOAD_THRESHOLD);
        assert_eq!(decision, LoadDecision::Full);
    }

    #[cfg(unix)]
    #[test]
    fn traversal_or_symlink_escape_forces_full() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("src");
        let outside_dir = temp.path().join("outside");
        let link_dir = root.join("Catalogs.Items");
        let escaped = outside_dir.join("ObjectModule.bsl");

        std::fs::create_dir_all(&outside_dir).expect("outside");
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(&escaped, "module").expect("write escaped");
        symlink(&outside_dir, &link_dir).expect("create symlink");

        let decision = decide(
            &[FileChange {
                path: link_dir.join("ObjectModule.bsl"),
                kind: ChangeKind::Modified,
            }],
            &root,
            DEFAULT_PARTIAL_LOAD_THRESHOLD,
        );

        assert_eq!(decision, LoadDecision::Full);
    }
}
