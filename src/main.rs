mod app;
mod cleanup;
mod error;
mod hosts;
mod proxy;
mod tui;

use crate::app::Mapping;
use crate::cleanup::{install_panic_hook, run_cleanup, spawn_signal_handler};
use crate::hosts::manager::HostsManager;
use crate::proxy::server::run_proxy;
use crate::tui::terminal::run_tui;
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::watch;

fn escalate_if_needed() -> Result<()> {
    if unsafe { libc::geteuid() == 0 } {
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = std::process::Command::new("sudo")
        .arg(exe)
        .args(args)
        .status()?;
    std::process::exit(status.code().unwrap_or(1));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Handle --cleanup flag
    if args.iter().any(|a| a == "--cleanup") {
        escalate_if_needed()?;
        return run_cleanup();
    }

    escalate_if_needed()?;

    let hosts_path = PathBuf::from("/etc/hosts");

    // Install panic hook for crash cleanup
    install_panic_hook(hosts_path.clone());

    // Shutdown signal channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn signal handlers (Ctrl+C, SIGTERM)
    spawn_signal_handler(hosts_path.clone(), shutdown_tx.clone());

    // Shared mappings channel (TUI writes, proxy reads)
    let (mappings_tx, mappings_rx) = watch::channel::<Vec<Mapping>>(Vec::new());

    let hosts_manager = HostsManager::new();

    // Run proxy and TUI concurrently
    let proxy_shutdown_rx = shutdown_rx.clone();
    let proxy_mappings_rx = mappings_rx.clone();

    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = run_proxy(proxy_mappings_rx, proxy_shutdown_rx).await {
            eprintln!("Proxy error: {}", e);
        }
    });

    // Run TUI on the main task (it needs terminal access)
    let tui_result = run_tui(mappings_tx, hosts_manager, shutdown_rx).await;

    // TUI exited — signal shutdown to proxy
    let _ = shutdown_tx.send(true);

    // Clean up /etc/hosts
    let manager = HostsManager::new();
    if let Err(e) = manager.restore_all() {
        eprintln!("Warning: failed to clean up /etc/hosts: {}", e);
    }

    // Wait for proxy to finish
    let _ = proxy_handle.await;

    tui_result
}
