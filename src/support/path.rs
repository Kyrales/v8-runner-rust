use std::path::{Component, Path};

/// Returns true when `value` can be safely used as a single file/path segment.
pub fn is_safe_path_segment(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let mut components = Path::new(value).components();
    let is_single_normal_component =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    if !is_single_normal_component {
        return false;
    }

    !value.chars().any(|ch| {
        matches!(
            ch,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'
        )
    })
}
