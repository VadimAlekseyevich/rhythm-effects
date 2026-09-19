#![forbid(unsafe_code)]

#[must_use]
pub const fn status() -> &'static str {
    "engine scaffold ready"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_status_is_available() {
        assert_eq!(status(), "engine scaffold ready");
    }
}
