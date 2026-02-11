use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PortmapError {
    #[error("Must run as root (sudo portmap)")]
    NotRoot,

    #[error("Failed to read /etc/hosts: {0}")]
    HostsRead(#[from] std::io::Error),

    #[error("Failed to write /etc/hosts: {0}")]
    HostsWrite(String),

    #[error("Port must be between 1 and 65535")]
    InvalidPort,

    #[error("Domain must be non-empty and contain only alphanumeric characters or hyphens")]
    InvalidDomain,

    #[error("Mapping already exists for {0}")]
    DuplicateMapping(String),

    #[error("Failed to bind to port 80: {0}")]
    ProxyBind(String),

    #[error("Proxy error: {0}")]
    ProxyError(String),
}
