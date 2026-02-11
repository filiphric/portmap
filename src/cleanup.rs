use crate::hosts::manager::{sync_cleanup, HostsManager};
use std::path::PathBuf;

/// Install a panic hook that cleans up /etc/hosts before aborting.
pub fn install_panic_hook(hosts_path: PathBuf) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Attempt synchronous cleanup
        sync_cleanup(&hosts_path);
        // Call the default hook to print the panic message
        default_hook(info);
    }));
}

/// Run the --cleanup command: remove all portmap entries from /etc/hosts.
pub fn run_cleanup() -> anyhow::Result<()> {
    let manager = HostsManager::new();
    manager.restore_all()?;
    println!("Cleaned up all portmap entries from /etc/hosts");
    Ok(())
}

/// Spawn a task that listens for Ctrl+C and SIGTERM, then cleans up.
pub fn spawn_signal_handler(
    hosts_path: PathBuf,
    shutdown: tokio::sync::watch::Sender<bool>,
) {
    // Ctrl+C handler
    let path = hosts_path.clone();
    let shutdown_tx = shutdown.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        sync_cleanup(&path);
        let _ = shutdown_tx.send(true);
    });

    // SIGTERM handler (Unix only)
    #[cfg(unix)]
    {
        let path = hosts_path;
        let shutdown_tx = shutdown;
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM handler");
            sigterm.recv().await;
            sync_cleanup(&path);
            let _ = shutdown_tx.send(true);
        });
    }
}
