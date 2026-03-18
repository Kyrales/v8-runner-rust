use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::change_detection::file_state::FileState;
use crate::change_detection::hash_storage::HashStorage;
use crate::change_detection::scanner::{self, ScanError};
use crate::config::model::SourceSetConfig;

/// A single detected file change.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub new_hash: Option<String>,
}

/// How a file changed relative to the stored state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    /// File is new (not in storage).
    Added,
    /// File content changed (hash differs).
    Modified,
    /// File existed in storage but is now absent on disk.
    Deleted,
}

/// Result of analyzing one source-set for changes.
#[derive(Debug)]
pub struct SourceSetChanges {
    pub source_set_name: String,
    pub changes: Vec<FileChange>,
    /// True when the scanner failed and we fell back to "all changed".
    pub is_fallback: bool,
}

impl SourceSetChanges {
    /// Returns true if there are any changes (including fallback).
    pub fn has_changes(&self) -> bool {
        self.is_fallback || !self.changes.is_empty()
    }
}

/// Analyse changes for a single source-set directory against its `HashStorage`.
///
/// On scan error, falls back to reporting all current files as changed
/// (safe fallback per spec).
pub fn analyze(
    source_set: &SourceSetConfig,
    root: &Path,
    storage: &HashStorage,
) -> SourceSetChanges {
    match scanner::scan(root) {
        Ok(current_files) => {
            let changes = detect_changes(&current_files, storage);
            SourceSetChanges {
                source_set_name: source_set.name.clone(),
                changes,
                is_fallback: false,
            }
        }
        Err(e) => {
            tracing::warn!(
                source_set = %source_set.name,
                error = %e,
                "scan failed, falling back to full rebuild"
            );
            SourceSetChanges {
                source_set_name: source_set.name.clone(),
                changes: vec![],
                is_fallback: true,
            }
        }
    }
}

/// Compare scanned files against stored hashes, returning only changed entries.
fn detect_changes(current: &[FileState], storage: &HashStorage) -> Vec<FileChange> {
    let current_paths: HashSet<PathBuf> = current.iter().map(|file| file.path.clone()).collect();

    let mut changes: Vec<FileChange> = current
        .iter()
        .filter_map(|file| {
            let kind = match storage.get(&file.path) {
                None => ChangeKind::Added,
                Some(stored) if stored.hash != file.hash => ChangeKind::Modified,
                Some(_) => return None,
            };
            Some(FileChange {
                path: file.path.clone(),
                kind,
                new_hash: Some(file.hash.clone()),
            })
        })
        .collect();

    changes.extend(
        storage
            .iter()
            .filter(|stored| !current_paths.contains(&stored.path))
            .map(|stored| FileChange {
                path: stored.path.clone(),
                kind: ChangeKind::Deleted,
                new_hash: None,
            }),
    );

    changes
}

/// Update storage with the current scan results after a successful build.
pub fn commit_changes(current_files: &[FileState], storage: &mut HashStorage) {
    let current_paths: HashSet<PathBuf> =
        current_files.iter().map(|file| file.path.clone()).collect();
    let stale_paths: Vec<PathBuf> = storage
        .iter()
        .filter(|stored| !current_paths.contains(&stored.path))
        .map(|stored| stored.path.clone())
        .collect();

    for path in stale_paths {
        storage.remove(&path);
    }

    for file in current_files {
        storage.insert(file.clone());
    }
}

/// Re-scan `root` and commit all results into `storage`.
/// Returns `Err` only if the scan itself fails.
pub fn rescan_and_commit(root: &Path, storage: &mut HashStorage) -> Result<(), ScanError> {
    let files = scanner::scan(root)?;
    commit_changes(&files, storage);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{commit_changes, detect_changes, ChangeKind};
    use crate::change_detection::file_state::FileState;
    use crate::change_detection::hash_storage::HashStorage;

    #[test]
    fn detect_changes_reports_deleted_files() {
        let deleted = PathBuf::from("/tmp/deleted.bsl");
        let mut storage = HashStorage::default();
        storage.insert(FileState::new(deleted.clone(), 1, "old".to_owned()));

        let changes = detect_changes(&[], &storage);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, deleted);
        assert_eq!(changes[0].kind, ChangeKind::Deleted);
        assert_eq!(changes[0].new_hash, None);
    }

    #[test]
    fn commit_changes_removes_stale_entries() {
        let kept = FileState::new(PathBuf::from("/tmp/kept.bsl"), 2, "new".to_owned());
        let deleted = PathBuf::from("/tmp/deleted.bsl");
        let mut storage = HashStorage::default();
        storage.insert(FileState::new(kept.path.clone(), 1, "old".to_owned()));
        storage.insert(FileState::new(deleted.clone(), 1, "old".to_owned()));

        commit_changes(std::slice::from_ref(&kept), &mut storage);

        assert!(storage.get(&kept.path).is_some());
        assert!(storage.get(&deleted).is_none());
    }
}
