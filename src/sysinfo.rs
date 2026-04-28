use std::cell::RefCell;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessesToUpdate, System};

const CPU_REFRESH_INTERVAL: Duration = Duration::from_millis(200);
const BYTES_PER_MIB: f64 = 1024.0 * 1024.0;
static PID: LazyLock<Pid> = LazyLock::new(|| Pid::from_u32(std::process::id()));

static LOGICAL_CPUS: LazyLock<u16> = LazyLock::new(|| {
    with_system(|sys| {
        sys.refresh_cpu_all();
        sys.cpus().len() as u16
    })
});

thread_local! {
    static SYSTEM: RefCell<System> = RefCell::new(System::new());
}

fn with_system<R>(f: impl FnOnce(&mut System) -> R) -> R {
    SYSTEM.with(|sys| f(&mut sys.borrow_mut()))
}

pub fn system_cpu_usage() -> f64 {
    fn read(sys: &mut System) -> f64 {
        sys.refresh_cpu_all();
        sys.global_cpu_usage() as f64
    }

    thread_local! {
        static CACHED: RefCell<(f64, Instant)> = RefCell::new((
            with_system(read),
            Instant::now(),
        ));
    }
    CACHED.with(|cell| {
        let (value, ts) = &mut *cell.borrow_mut();
        if ts.elapsed() > CPU_REFRESH_INTERVAL {
            *value = with_system(read);
            *ts = Instant::now();
        }
        *value
    })
}

#[cfg(feature = "lua-api")]
pub fn process_cpu_usage() -> f64 {
    fn read(sys: &mut System) -> f64 {
        sys.refresh_processes(ProcessesToUpdate::Some(&[*PID]), true);
        sys.process(*PID)
            .map_or(0.0, |p| p.cpu_usage() as f64 / *LOGICAL_CPUS as f64)
    }

    thread_local! {
        static CACHED: RefCell<(f64, Instant)> = RefCell::new(({
            let _ = *LOGICAL_CPUS; // force init before first read
            with_system(read)
        }, Instant::now()));
    }
    CACHED.with(|cell| {
        let (value, ts) = &mut *cell.borrow_mut();
        if ts.elapsed() > CPU_REFRESH_INTERVAL {
            *value = with_system(read);
            *ts = Instant::now();
        }
        *value
    })
}

#[cfg(feature = "lua-api")]
pub fn system_memory_usage() -> f64 {
    with_system(|sys| {
        sys.refresh_memory();
        sys.used_memory() as f64 / BYTES_PER_MIB
    })
}

pub fn system_total_memory() -> f64 {
    static VALUE: LazyLock<f64> = LazyLock::new(|| {
        with_system(|sys| {
            sys.refresh_memory();
            sys.total_memory() as f64 / BYTES_PER_MIB
        })
    });
    *VALUE
}

#[cfg(feature = "lua-api")]
pub fn system_available_memory() -> f64 {
    with_system(|sys| {
        sys.refresh_memory();
        sys.available_memory() as f64 / BYTES_PER_MIB
    })
}

#[cfg(feature = "lua-api")]
pub fn process_memory_usage() -> f64 {
    with_system(|sys| {
        sys.refresh_processes(ProcessesToUpdate::Some(&[*PID]), true);
        sys.process(*PID)
            .expect("Failed to get process information")
            .memory() as f64
            / BYTES_PER_MIB
    })
}

pub fn logical_cpus() -> u16 {
    *LOGICAL_CPUS
}

pub fn physical_cpus() -> u16 {
    static VALUE: LazyLock<u16> =
        LazyLock::new(|| System::physical_core_count().unwrap_or(0) as u16);
    *VALUE
}

pub fn system_swap_total() -> f64 {
    static VALUE: LazyLock<f64> = LazyLock::new(|| {
        with_system(|sys| {
            sys.refresh_memory();
            sys.total_swap() as f64 / BYTES_PER_MIB
        })
    });
    *VALUE
}

/// All process metrics from a single `refresh_processes` call.
pub struct ProcessMetrics {
    pub cpu_usage: f64,
    pub memory_mib: f64,
    pub disk_read_bytes: f64,
    pub disk_write_bytes: f64,
    pub thread_count: u16,
    pub uptime_seconds: f64,
}

/// Refresh process info once and return all metrics.
pub fn process_metrics() -> ProcessMetrics {
    with_system(|sys| {
        sys.refresh_processes(ProcessesToUpdate::Some(&[*PID]), true);
        match sys.process(*PID) {
            Some(p) => {
                let disk = p.disk_usage();
                ProcessMetrics {
                    cpu_usage: p.cpu_usage() as f64 / *LOGICAL_CPUS as f64,
                    memory_mib: p.memory() as f64 / BYTES_PER_MIB,
                    disk_read_bytes: disk.total_read_bytes as f64,
                    disk_write_bytes: disk.total_written_bytes as f64,
                    thread_count: p.tasks().map_or(0, |t| t.len()) as u16,
                    uptime_seconds: p.run_time() as f64,
                }
            }
            None => ProcessMetrics {
                cpu_usage: 0.0,
                memory_mib: 0.0,
                disk_read_bytes: 0.0,
                disk_write_bytes: 0.0,
                thread_count: 0,
                uptime_seconds: 0.0,
            },
        }
    })
}

/// All system memory/swap metrics from a single `refresh_memory` call.
pub struct SystemMemoryMetrics {
    pub used_mib: f64,
    pub available_mib: f64,
    pub swap_used_mib: f64,
}

/// Refresh memory once and return all metrics.
pub fn system_memory_metrics() -> SystemMemoryMetrics {
    with_system(|sys| {
        sys.refresh_memory();
        SystemMemoryMetrics {
            used_mib: sys.used_memory() as f64 / BYTES_PER_MIB,
            available_mib: sys.available_memory() as f64 / BYTES_PER_MIB,
            swap_used_mib: sys.used_swap() as f64 / BYTES_PER_MIB,
        }
    })
}

#[cfg(feature = "lua-api")]
#[derive(Copy, Clone, Debug)]
pub struct AllSystem {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub total_memory: f64,
    pub available_memory: f64,
    pub logical_cpus: u16,
    pub physical_cpus: u16,
}

#[cfg(feature = "lua-api")]
pub fn all_system() -> AllSystem {
    AllSystem {
        cpu_usage: system_cpu_usage(),
        memory_usage: system_memory_usage(),
        total_memory: system_total_memory(),
        available_memory: system_available_memory(),
        logical_cpus: logical_cpus(),
        physical_cpus: physical_cpus(),
    }
}

#[cfg(feature = "lua-api")]
#[derive(Copy, Clone, Debug)]
pub struct AllProcess {
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

#[cfg(feature = "lua-api")]
pub fn all_process() -> AllProcess {
    AllProcess {
        cpu_usage: process_cpu_usage(),
        memory_usage: process_memory_usage(),
    }
}

#[cfg(feature = "lua-api")]
pub fn all() -> (AllSystem, AllProcess) {
    (all_system(), all_process())
}

#[cfg(feature = "lua-api")]
#[derive(Copy, Clone, Debug)]
pub struct RealtimeData {
    pub system_cpu_usage: f64,
    pub system_memory_usage: f64,
    pub system_available_memory: f64,
    pub process_cpu_usage: f64,
    pub process_memory_usage: f64,
}

#[cfg(feature = "lua-api")]
pub fn realtime() -> RealtimeData {
    RealtimeData {
        system_cpu_usage: system_cpu_usage(),
        system_memory_usage: system_memory_usage(),
        system_available_memory: system_available_memory(),
        process_cpu_usage: process_cpu_usage(),
        process_memory_usage: process_memory_usage(),
    }
}
