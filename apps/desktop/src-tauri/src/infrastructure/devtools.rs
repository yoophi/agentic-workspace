pub fn should_open_devtools() -> bool {
    std::env::var("ACP_OPEN_DEVTOOLS")
        .ok()
        .as_deref()
        .is_some_and(devtools_flag_enabled)
}

fn devtools_flag_enabled(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
}

#[cfg(test)]
mod tests {
    use super::devtools_flag_enabled;

    #[test]
    fn recognizes_enabled_devtools_flags() {
        for value in ["1", "true", "TRUE", "yes", "YES", "on", "ON"] {
            assert!(devtools_flag_enabled(value));
        }
    }

    #[test]
    fn rejects_disabled_or_empty_devtools_flags() {
        for value in ["", "0", "false", "FALSE", "no", "off", "anything"] {
            assert!(!devtools_flag_enabled(value));
        }
    }
}
