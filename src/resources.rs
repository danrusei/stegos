use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Resource {
    cpuUsage(CpuUsage),
}

// CpuUsage display usage for each processor (or processor core) using `mpstat`
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CpuUsage {
    // %usr – % CPU usage at the user level
    usr: f64,
    // %nice – % CPU usage for user processes labeled “nice”
    nice: f64,
    // %sys – % CPU usage at the system (Linux kernel) level
    sys: f64,
    // %iowait – % CPU usage idling waiting on a disk read/write
    iowait: f64,
    // %irq – % CPU usage handling hardware interrupts
    irq: f64,
    // %soft – % CPU usage handing software interrupts
    soft: f64,
    // %steal – % CPU usage being forced to wait for a hypervisor handling other virtual processors
    steal: f64,
    // %guest – % CPU usage spent running a virtual processor
    guest: f64,
    // %idle – % CPU usage on idle time (no processes, and not waiting on a disk read/write)
    idle: f64,
}
