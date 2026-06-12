#[cfg(test)]
mod tests {
    use crate::networking::utils::{get_bar, format_bytes};
    use crate::networking::service::{SystemData, ProcessData};
    use crate::networking::view::{prepare_monitor_view, prepare_wtn_view};

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B/s");
        assert_eq!(format_bytes(1500), "1.5KB/s");
        assert_eq!(format_bytes(1_500_000), "1.5MB/s");
        assert_eq!(format_bytes(1_500_000_000), "1.5GB/s");
    }

    #[test]
    fn test_get_bar_idle() {
        let bar = get_bar(0.0);
        assert!(bar.contains("\u{2581}"));
    }

    #[test]
    fn test_get_bar_full() {
        let bar = get_bar(100.0);
        assert!(bar.contains("\u{2588}"));
    }

    #[test]
    fn test_prepare_monitor_view_system_fixed() {
        let data = SystemData {
            global_cpu: 10.0,
            used_memory: 1024 * 1024,
            total_memory: 1024 * 1024 * 1024,
            net_rx: 0,
            net_tx: 0,
            gpu_status: vec![],
            processes: vec![
                ProcessData {
                    pid: "123".to_string(),
                    name: "test_proc".to_string(),
                    cpu_usage: 5.0,
                    memory: 1024,
                }
            ],
        };

        let buf = prepare_monitor_view(&data, &["*".to_string()], &["cpu", "mem"], 1.0, false);
        let output = String::from_utf8_lossy(&buf);
        
        assert!(output.contains("SYSTEM"));
        assert!(output.contains("test_proc"), "Process list should NOT be empty when targets=['*']");
    }

    #[test]
    fn test_prepare_wtn_view() {
        let data = SystemData {
            global_cpu: 10.0,
            used_memory: 1024 * 1024,
            total_memory: 1024 * 1024 * 1024,
            net_rx: 500,
            net_tx: 300,
            gpu_status: vec![],
            processes: vec![
                ProcessData {
                    pid: "123".to_string(),
                    name: "test_proc".to_string(),
                    cpu_usage: 5.0,
                    memory: 1024,
                }
            ],
        };

        let buf = prepare_wtn_view(&data, &["123".to_string()], false);
        let output = String::from_utf8_lossy(&buf);
        
        assert!(output.contains("NETWORK"));
        assert!(output.contains("123"));
        assert!(output.contains("test_proc"));
    }
}
