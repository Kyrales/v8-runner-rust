use std::path::PathBuf;

use crate::config::model::{AppConfig, SourceFormat};
use crate::domain::source_set::SourceSetContext;

/// Builds the list of [`SourceSetContext`] instances for the given config.
///
/// - `DESIGNER` format: one context per source-set, rooted at `basePath/ss.path`.
/// - `EDT` format (Wave 2): two contexts per source-set — the original EDT path
///   and a generated Designer copy under `workPath/<name>/`.
pub struct SourceSetsService<'a> {
    config: &'a AppConfig,
}

impl<'a> SourceSetsService<'a> {
    pub fn new(config: &'a AppConfig) -> Self {
        Self { config }
    }

    /// Return all Designer-format contexts that should be scanned and built.
    ///
    /// In `DESIGNER` mode this is simply each source-set resolved against `basePath`.
    /// In `EDT` mode (Wave 2) this returns the generated Designer copies in `workPath`.
    pub fn designer_contexts(&self) -> Vec<SourceSetContext> {
        match self.config.format {
            SourceFormat::Designer => self
                .config
                .source_sets
                .iter()
                .map(|ss| {
                    let path = if ss.path.is_absolute() {
                        ss.path.clone()
                    } else {
                        self.config.base_path.join(&ss.path)
                    };
                    SourceSetContext::new(&ss.name, path, format!("designer-{}", ss.name))
                })
                .collect(),

            SourceFormat::Edt => self
                .config
                .source_sets
                .iter()
                .map(|ss| {
                    // Generated Designer copy lives at workPath/<name>/
                    let path = self.config.work_path.join(&ss.name);
                    SourceSetContext::new(&ss.name, path, format!("designer-{}", ss.name))
                })
                .collect(),
        }
    }

    /// Return EDT source-set contexts (only meaningful in `EDT` format).
    pub fn edt_contexts(&self) -> Vec<SourceSetContext> {
        if self.config.format != SourceFormat::Edt {
            return vec![];
        }
        self.config
            .source_sets
            .iter()
            .map(|ss| {
                let path = if ss.path.is_absolute() {
                    ss.path.clone()
                } else {
                    self.config.base_path.join(&ss.path)
                };
                SourceSetContext::new(&ss.name, path, format!("edt-{}", ss.name))
            })
            .collect()
    }

    /// Absolute path to the hash-storages directory.
    pub fn storage_dir(&self) -> PathBuf {
        self.config.work_path.join("hash-storages")
    }
}
