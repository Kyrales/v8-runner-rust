/// Parse connection string components
pub struct ConnectionString(pub String);

impl ConnectionString {
    pub fn file_path(&self) -> Option<&str> {
        self.0.split(';').find_map(|part| {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower.starts_with("file=") {
                Some(&part[5..])
            } else {
                None
            }
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
