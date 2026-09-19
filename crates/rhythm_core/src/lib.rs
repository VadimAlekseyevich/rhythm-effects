#![forbid(unsafe_code)]

pub mod domain;

pub mod ids;
pub mod time;

pub const APP_NAME: &str = "Rhythm Effects";

#[must_use]
pub const fn app_name() -> &'static str {
    APP_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_stable() {
        assert_eq!(app_name(), "Rhythm Effects");
    }
}
