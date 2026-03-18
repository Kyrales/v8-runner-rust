use std::path::PathBuf;
use tempfile::NamedTempFile;

pub fn create_temp_file(prefix: &str, suffix: &str) -> std::io::Result<NamedTempFile> {
    tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile()
}

pub fn temp_dir_for(work_path: &PathBuf, name: &str) -> PathBuf {
    work_path.join(name)
}
