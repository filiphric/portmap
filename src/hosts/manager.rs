use crate::hosts::parser::HostsFile;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Manages the /etc/hosts file with portmap sentinel blocks.
pub struct HostsManager {
    path: PathBuf,
}

impl HostsManager {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from("/etc/hosts"),
        }
    }

    /// Create a manager with a custom path (for testing).
    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    fn read(&self) -> Result<String> {
        std::fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read {}", self.path.display()))
    }

    fn write(&self, content: &str) -> Result<()> {
        std::fs::write(&self.path, content)
            .with_context(|| format!("Failed to write {}", self.path.display()))
    }

    /// Add a domain → localhost mapping to /etc/hosts.
    pub fn add_entry(&self, domain: &str) -> Result<bool> {
        let content = self.read()?;
        let mut hosts = HostsFile::parse(&content);
        if !hosts.add_entry(domain, "127.0.0.1") {
            return Ok(false);
        }
        self.write(&hosts.serialize())?;
        Ok(true)
    }

    /// Remove a domain mapping from /etc/hosts.
    pub fn remove_entry(&self, domain: &str) -> Result<bool> {
        let content = self.read()?;
        let mut hosts = HostsFile::parse(&content);
        if !hosts.remove_entry(domain) {
            return Ok(false);
        }
        self.write(&hosts.serialize())?;
        Ok(true)
    }

    /// Remove all portmap-managed entries from /etc/hosts.
    pub fn restore_all(&self) -> Result<()> {
        let content = self.read()?;
        let mut hosts = HostsFile::parse(&content);
        hosts.remove_all();
        self.write(&hosts.serialize())?;
        Ok(())
    }

    #[cfg(test)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Synchronous cleanup function for use in panic hooks and signal handlers.
/// Reads /etc/hosts and removes the sentinel block.
pub fn sync_cleanup(path: &Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let mut hosts = HostsFile::parse(&content);
    hosts.remove_all();
    let _ = std::fs::write(path, hosts.serialize());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_hosts(content: &str) -> (NamedTempFile, HostsManager) {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        let path = file.path().to_path_buf();
        let manager = HostsManager::with_path(path);
        (file, manager)
    }

    #[test]
    fn test_add_and_remove() {
        let (_file, manager) = temp_hosts("127.0.0.1\tlocalhost\n");

        // Add entry
        assert!(manager.add_entry("test.localhost").unwrap());
        let content = std::fs::read_to_string(manager.path()).unwrap();
        assert!(content.contains("test.localhost"));
        assert!(content.contains("portmap-start"));

        // Duplicate returns false
        assert!(!manager.add_entry("test.localhost").unwrap());

        // Remove entry
        assert!(manager.remove_entry("test.localhost").unwrap());
        let content = std::fs::read_to_string(manager.path()).unwrap();
        assert!(!content.contains("test.localhost"));
        assert!(!content.contains("portmap-start"));

        // Original content preserved
        assert!(content.contains("127.0.0.1\tlocalhost"));
    }

    #[test]
    fn test_restore_all() {
        let (_file, manager) = temp_hosts("127.0.0.1\tlocalhost\n");
        manager.add_entry("a.localhost").unwrap();
        manager.add_entry("b.localhost").unwrap();
        manager.restore_all().unwrap();
        let content = std::fs::read_to_string(manager.path()).unwrap();
        assert!(!content.contains("portmap-start"));
        assert!(!content.contains("a.localhost"));
        assert!(content.contains("127.0.0.1\tlocalhost"));
    }
}
