//! Pure window-message policy, separate from shortcut activation/generation production.
pub fn blocks_message(
    open: bool,
    alt_f4_binding: bool,
    modifiers: u8,
    message: u32,
    key: usize,
) -> bool {
    if open && (matches!(message, 0x100..=0x109 | 0x200..=0x20e) || message == 0xff) {
        return true;
    }
    // WM_SYSKEYDOWN/UP, VK_F4. Suppress native SC_CLOSE generation for this saved chord only.
    alt_f4_binding && modifiers == 16 && key == 0x73 && matches!(message, 0x104 | 0x105)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_is_scoped_to_menu_input_and_saved_left_alt_f4() {
        assert!(blocks_message(true, false, 0, 0x100, 0));
        assert!(blocks_message(true, false, 0, 0xff, 0));
        assert!(blocks_message(true, false, 0, 0x201, 0));
        assert!(
            !blocks_message(true, false, 0, 0x10, 0),
            "close-window is not input capture"
        );
        assert!(
            !blocks_message(true, false, 0, 0x8, 0),
            "focus messages must propagate"
        );
        assert!(!blocks_message(false, false, 0, 0x100, 0));
        assert!(blocks_message(false, true, 16, 0x104, 0x73));
        assert!(!blocks_message(false, false, 16, 0x104, 0x73));
        assert!(!blocks_message(false, true, 32, 0x104, 0x73));
        assert!(!blocks_message(false, true, 17, 0x104, 0x73));
    }
}
