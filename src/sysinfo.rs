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

// ── Scalar metrics ───────────────────────────────────────────────────────

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

pub fn system_available_memory() -> f64 {
	with_system(|sys| {
		sys.refresh_memory();
		sys.available_memory() as f64 / BYTES_PER_MIB
	})
}

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

// ── Bundle types (Lua API only) ──────────────────────────────────────────

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
