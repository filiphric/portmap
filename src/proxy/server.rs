use crate::app::Mapping;
use crate::proxy::handler::handle_request;
use anyhow::Result;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::watch;

/// Start the reverse proxy server on port 80.
/// Runs until the shutdown signal is received.
pub async fn run_proxy(
    mappings_rx: watch::Receiver<Vec<Mapping>>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to bind to port 80: {}. Are you running with sudo?",
            e
        )
    })?;

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _addr) = result?;
                let rx = mappings_rx.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = service_fn(move |req| {
                        let rx = rx.clone();
                        handle_request(req, rx)
                    });
                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, service)
                        .await
                    {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }

    Ok(())
}
