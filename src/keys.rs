pub(crate) fn alias(key: &str) -> &str {
    match key {
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "Insert" => "Ins",
        "PrintScreen" => "PrtSc",
        "ScrollLock" => "ScrLk",
        "NumLock" => "Num",
        "Escape" => "Esc",
        "Delete" => "Del",
        _ => key,
    }
}

pub(crate) fn is_alias(key: &str) -> bool {
    alias(key) != key
}

pub(crate) fn is_function_key(key: &str) -> bool {
    matches!(
        key,
        "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11" | "F12"
    )
}

pub(crate) fn display_text(key: &str, caps_lock: bool) -> String {
    let key = alias(key);
    if key.len() == 1 && key.as_bytes()[0].is_ascii_uppercase() && !caps_lock {
        key.to_ascii_lowercase()
    } else {
        key.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_lock_changes_letters_without_shift() {
        assert_eq!(display_text("A", false), "a");
        assert_eq!(display_text("A", true), "A");
    }

    #[test]
    fn identifies_function_keys() {
        assert!(is_function_key("F1"));
        assert!(is_function_key("F12"));
        assert!(!is_function_key("F13"));
        assert!(!is_function_key("1"));
    }
}
