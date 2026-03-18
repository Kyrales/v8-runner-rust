use std::path::{Path, PathBuf};

use crate::change_detection::analyzer::{ChangeKind, FileChange};

/// Maximum number of changed files before forcing a full load.
pub const PARTIAL_LOAD_THRESHOLD: usize = 20;

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
///
/// Rules (per spec):
/// - If `Configuration.xml` is among the changed files → Full.
/// - If any file was deleted → Full (partial load cannot safely replay removals).
/// - If the number of expanded files exceeds [`PARTIAL_LOAD_THRESHOLD`] → Full.
/// - Otherwise → Partial with the expanded file list.
pub fn decide(changes: &[FileChange], source_root: &Path) -> LoadDecision {
    // Configuration.xml touched → must do full load.
    let has_config_xml = changes.iter().any(|c| {
        c.path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == CONFIGURATION_XML)
            .unwrap_or(false)
    });

    if has_config_xml {
        return LoadDecision::Full;
    }

    if changes.iter().any(|c| c.kind == ChangeKind::Deleted) {
        return LoadDecision::Full;
    }

    let expanded = expand_files(changes, source_root);

    if expanded.len() > PARTIAL_LOAD_THRESHOLD {
        LoadDecision::Full
    } else {
        LoadDecision::Partial(expanded)
    }
}

/// Expand the raw changed-file list into the set of paths that Designer needs
/// for a partial load.
///
/// Rules (per spec / Designer `-partial` semantics):
/// - Every changed file is included as-is.
/// - For `.bsl` files: also include the sibling XML descriptor and the
///   parent object directory (Designer needs the full object context).
fn expand_files(changes: &[FileChange], _source_root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for change in changes {
        paths.push(change.path.clone());

        if is_bsl(&change.path) {
            // Sibling XML descriptor (same stem, .xml extension).
            if let Some(xml) = sibling_xml(&change.path) {
                if xml.exists() && !paths.contains(&xml) {
                    paths.push(xml);
                }
            }
            // Parent object directory.
            if let Some(obj_dir) = object_dir(&change.path) {
                if obj_dir.exists() && !paths.contains(&obj_dir) {
                    paths.push(obj_dir);
                }
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
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
///
/// Designer object layout: `ObjectType.ObjectName/Forms/FormName/Module.bsl`
/// or `ObjectType.ObjectName/ObjectModule.bsl`.
/// The "object directory" is the parent for top-level object modules, or the
/// ancestor above the nested kind/name folders for paths like `Forms/Foo/Module.bsl`.
fn object_dir(bsl: &Path) -> Option<PathBuf> {
    let parent = bsl.parent()?;
    let is_nested_module = bsl
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.eq_ignore_ascii_case("Module.bsl"))
        .unwrap_or(false);

    if is_nested_module {
        return parent.parent()?.parent().map(Path::to_path_buf);
    }

    Some(parent.to_path_buf())
}

/// Write a partial-load list file (UTF-8, one path per line, no empty lines).
///
/// Paths are written relative to `source_root` as required by Designer's
/// `-listFile` parameter when running in agent mode.
pub fn write_list_file(paths: &[PathBuf], source_root: &Path, dest: &Path) -> std::io::Result<()> {
    let mut content = String::new();
    for path in paths {
        let rel = path.strip_prefix(source_root).unwrap_or(path);
        if rel.as_os_str().is_empty() {
            continue;
        }
        content.push_str(&rel.display().to_string());
        content.push_str("\r\n");
    }
    std::fs::write(dest, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{object_dir, write_list_file};

    #[test]
    fn object_dir_uses_parent_for_top_level_modules() {
        let bsl = Path::new("/tmp/src/Catalogs.Items/ObjectModule.bsl");

        assert_eq!(
            object_dir(bsl),
            Some(PathBuf::from("/tmp/src/Catalogs.Items"))
        );
    }

    #[test]
    fn object_dir_uses_owning_object_for_nested_modules() {
        let bsl = Path::new("/tmp/src/Catalogs.Items/Forms/Form1/Module.bsl");

        assert_eq!(
            object_dir(bsl),
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
}
