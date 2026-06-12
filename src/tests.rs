/// Tests for utility functions and monitoring logic.
#[cfg(test)]
mod tests {
    use crate::utils::{get_bar, format_bytes};

    /// Verifies that byte counts are correctly formatted into human-readable strings.
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B/s");
        assert_eq!(format_bytes(1500), "1.5KB/s");
        assert_eq!(format_bytes(1_500_000), "1.5MB/s");
        assert_eq!(format_bytes(1_500_000_000), "1.5GB/s");
    }

    /// Verifies that 0% usage results in the idle state bar character.
    #[test]
    fn test_get_bar_idle() {
        // 0% should show the smallest block to indicate idle state
        let bar = get_bar(0.0);
        assert!(bar.contains("\u{2581}"));
    }

    /// Verifies that 100% usage results in the full bar character.
    #[test]
    fn test_get_bar_full() {
        // 100% should show the full block
        let bar = get_bar(100.0);
        assert!(bar.contains("\u{2588}"));
    }

    /// Verifies that the bar changes color at the expected usage thresholds.
    #[test]
    fn test_get_bar_thresholds() {
        // Force colors on for testing so we can distinguish thresholds
        colored::control::set_override(true);
        
        // < 33.0 is Green
        let green = get_bar(32.0);
        // 33.0 to < 66.0 is Yellow
        let yellow = get_bar(34.0);
        // >= 66.0 is Red
        let red = get_bar(67.0);

        assert_ne!(green, yellow, "Green and Yellow should be different (color codes)");
        assert_ne!(yellow, red, "Yellow and Red should be different (color codes)");
        assert_ne!(green, red, "Green and Red should be different (color codes)");
        
        // Reset color override to avoid affecting other things (though it's a test process)
        colored::control::unset_override();
    }

    /// Verifies that percentage values outside the 0-100 range are correctly clamped.
    #[test]
    fn test_get_bar_clamping() {
        // Negative should be treated as 0% (idle block)
        let bar_neg = get_bar(-5.0);
        assert!(bar_neg.contains("\u{2581}"));

        // > 100% should be treated as 100% (full block)
        let bar_over = get_bar(105.0);
        assert!(bar_over.contains("\u{2588}"));
    }

    /// Verifies that the correct bar character is chosen for various percentage steps.
    #[test]
    fn test_get_bar_steps() {
        // Verify character selection at different percentages
        // 0-12.5% should be index 1 (▂)
        assert!(get_bar(0.0).contains("\u{2581}"));
        assert!(get_bar(12.5).contains("\u{2581}"));
        
        // 12.6% should jump to index 2 (▃)
        assert!(get_bar(13.0).contains("\u{2582}"));
        
        // 50% should be index 4 (▅)
        assert!(get_bar(50.0).contains("\u{2584}"));
        
        // 100% should be index 8 (█)
        assert!(get_bar(100.0).contains("\u{2588}"));
    }
}
