//! Prometheus metrics exporter.
//!
//! Uses the `metrics` + `metrics-exporter-prometheus` crates to serve
//! `/metrics` via an HTTP listener.  A background thread periodically
//! refreshes gauge values from `sysinfo`.
//!
//! Configuration:
//!   SERVERSTAT_METRICS_BIND  — listen address (default `0.0.0.0:9101`)

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::sysinfo;

static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start() {
	if RUNNING.swap(true, Ordering::SeqCst) {
		return; // already running
	}

	let bind_addr = std::env::var("SERVERSTAT_METRICS_BIND")
		.unwrap_or_else(|_| "0.0.0.0:9101".to_string());

	let addr: std::net::SocketAddr = match bind_addr.parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("[serverstat] Invalid metrics bind address '{bind_addr}': {e}");
			RUNNING.store(false, Ordering::SeqCst);
			return;
		}
	};

	if let Err(e) = metrics_exporter_prometheus::PrometheusBuilder::new()
		.with_http_listener(addr)
		.install()
	{
		eprintln!("[serverstat] Failed to install Prometheus exporter: {e}");
		RUNNING.store(false, Ordering::SeqCst);
		return;
	}

	eprintln!("[serverstat] Prometheus metrics on http://{addr}/metrics");

	// Descriptions (become HELP lines in Prometheus output)
	metrics::describe_gauge!(
		"srcds_process_cpu_usage",
		"SRCDS process CPU usage (0.0-1.0 normalized by core count)"
	);
	metrics::describe_gauge!(
		"srcds_process_memory_mib",
		"SRCDS process resident memory in MiB"
	);
	metrics::describe_gauge!(
		"srcds_system_cpu_usage",
		"System-wide CPU usage (0.0-100.0)"
	);
	metrics::describe_gauge!(
		"srcds_system_memory_used_mib",
		"System used memory in MiB"
	);
	metrics::describe_gauge!(
		"srcds_system_memory_available_mib",
		"System available memory in MiB"
	);
	metrics::describe_gauge!(
		"srcds_system_memory_total_mib",
		"System total memory in MiB"
	);
	metrics::describe_gauge!("srcds_logical_cpus", "Number of logical CPU cores");
	metrics::describe_gauge!("srcds_physical_cpus", "Number of physical CPU cores");

	// Static values — set once
	metrics::gauge!("srcds_system_memory_total_mib").set(sysinfo::system_total_memory());
	metrics::gauge!("srcds_logical_cpus").set(sysinfo::logical_cpus() as f64);
	metrics::gauge!("srcds_physical_cpus").set(sysinfo::physical_cpus() as f64);

	std::thread::Builder::new()
		.name("serverstat-metrics".into())
		.spawn(refresh_loop)
		.expect("Failed to spawn metrics refresh thread");
}

pub fn stop() {
	RUNNING.store(false, Ordering::SeqCst);
}

fn refresh_loop() {
	while RUNNING.load(Ordering::Relaxed) {
		metrics::gauge!("srcds_process_cpu_usage").set(sysinfo::process_cpu_usage());
		metrics::gauge!("srcds_process_memory_mib").set(sysinfo::process_memory_usage());
		metrics::gauge!("srcds_system_cpu_usage").set(sysinfo::system_cpu_usage());
		metrics::gauge!("srcds_system_memory_used_mib").set(sysinfo::system_memory_usage());
		metrics::gauge!("srcds_system_memory_available_mib")
			.set(sysinfo::system_available_memory());

		std::thread::sleep(Duration::from_secs(2));
	}
}
