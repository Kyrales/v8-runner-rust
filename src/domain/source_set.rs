use std::path::{Path, PathBuf};

use crate::support::path::is_safe_path_segment;

/// Runtime context for one logical source-set.
#[derive(Debug, Clone)]
pub struct SourceSetContext {
    /// Logical name (matches `SourceSetConfig.name`).
    name: String,
    /// Absolute root directory of the sources.
    path: PathBuf,
    /// Key used to name the redb hash-storage file (`workPath/hash-storages/<key>.redb`).
    storage_key: String,
}

impl SourceSetContext {
    pub fn new(name: impl Into<String>, path: PathBuf, storage_key: impl Into<String>) -> Self {
        let name = name.into();
        let storage_key = storage_key.into();
        assert!(
            path.is_absolute(),
            "SourceSetContext.path must be absolute, got: {}",
            path.display()
        );
        assert!(
            is_safe_path_segment(&storage_key),
            "SourceSetContext.storage_key must be a safe single path segment, got: {storage_key}"
        );

        Self {
            name,
            path,
            storage_key,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Absolute path to the redb hash-storage file for this context.
    pub fn storage_path(&self, work_path: &Path) -> PathBuf {
        work_path
            .join("hash-storages")
            .join(format!("{}.redb", self.storage_key))
    }
}

#[cfg(test)]
mod tests {
    use super::SourceSetContext;
    use std::path::PathBuf;

    #[test]
    fn accepts_absolute_path() {
        let context =
            SourceSetContext::new("main", PathBuf::from("/tmp/src-main"), "designer-main");
        assert_eq!(context.name(), "main");
        assert_eq!(context.path(), PathBuf::from("/tmp/src-main").as_path());
        assert_eq!(
            context.storage_path(PathBuf::from("/tmp/work").as_path()),
            PathBuf::from("/tmp/work/hash-storages/designer-main.redb")
        );
    }

    #[test]
    #[should_panic(expected = "must be absolute")]
    fn rejects_relative_path() {
        let _ = SourceSetContext::new("main", PathBuf::from("relative/path"), "designer-main");
    }

    #[test]
    fn accepts_safe_storage_key() {
        let context =
            SourceSetContext::new("main", PathBuf::from("/tmp/src-main"), "main-config_01");
        assert_eq!(
            context.storage_path(PathBuf::from("/tmp/work").as_path()),
            PathBuf::from("/tmp/work/hash-storages/main-config_01.redb")
        );
    }

    #[test]
    #[should_panic(expected = "safe single path segment")]
    fn rejects_storage_key_with_parent_traversal() {
        let _ = SourceSetContext::new("main", PathBuf::from("/tmp/src-main"), "../outside");
    }

    #[test]
    #[should_panic(expected = "safe single path segment")]
    fn rejects_storage_key_with_separator() {
        let _ = SourceSetContext::new("main", PathBuf::from("/tmp/src-main"), "bad/name");
    }

    #[test]
    #[should_panic(expected = "safe single path segment")]
    fn rejects_storage_key_with_backslash_separator() {
        let _ = SourceSetContext::new("main", PathBuf::from("/tmp/src-main"), "bad\\name");
    }
}
