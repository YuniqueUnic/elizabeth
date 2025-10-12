pub use merge::Merge;

/// Merge strategy for strings: overwrite if left is empty
pub fn overwrite_not_empty_string<S: Into<String>>(left: &mut String, right: S) {
    let right = right.into();
    if !right.is_empty() {
        *left = right;
    }
}

/// Merge strategy for strings: overwrite if left is empty
pub fn overwrite<S>(left: &mut S, right: S) {
    *left = right;
}
