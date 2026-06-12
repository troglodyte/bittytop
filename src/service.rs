use sysinfo::{System, Networks, ProcessesToUpdate};
use machine_info::{Machine, GraphicsUsage};

/// Holds metrics for a single process.
pub struct ProcessData {
    pub pid: String,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

/// Holds a snapshot of system-wide and per-process metrics.
pub struct SystemData {
    pub global_cpu: f32,
    pub used_memory: u64,
    pub total_memory: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub gpu_status: Vec<GraphicsUsage>,
    pub processes: Vec<ProcessData>,
}

/// A service that handles the collection of system and process data.
pub struct MonitorService {
    sys: System,
    machine: Machine,
    networks: Networks,
    num_cpus: f32,
}

impl MonitorService {
    /// Creates a new `MonitorService` and performs an initial full refresh.
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let num_cpus = sys.cpus().len().max(1) as f32;
        let machine = Machine::new();
        let networks = Networks::new_with_refreshed_list();
        
        Self {
            sys,
            machine,
            networks,
            num_cpus,
        }
    }

    /// Returns the number of logical CPUs on the system.
    pub fn get_num_cpus(&self) -> f32 {
        self.num_cpus
    }

    /// Refreshes system metrics and returns a new `SystemData` snapshot.
    pub fn tick(&mut self) -> SystemData {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.sys.refresh_memory();
        self.networks.refresh(false);

        let gpu = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.machine.graphics_status())).unwrap_or_default();
        let (net_rx, net_tx): (u64, u64) = self.networks.list().values()
            .map(|n| (n.received(), n.transmitted()))
            .fold((0, 0), |acc, (r, t)| (acc.0 + r, acc.1 + t));

        let processes = self.sys.processes().iter()
            .map(|(pid, proc)| ProcessData {
                pid: pid.to_string(),
                name: proc.name().to_string_lossy().to_string(),
                cpu_usage: proc.cpu_usage(),
                memory: proc.memory(),
            })
            .collect();

        SystemData {
            global_cpu: self.sys.global_cpu_usage(),
            used_memory: self.sys.used_memory(),
            total_memory: self.sys.total_memory(),
            net_rx,
            net_tx,
            gpu_status: gpu,
            processes,
        }
    }
}
