const SENTINEL_START: &str = "# portmap-start (DO NOT EDIT - managed by portmap)";
const SENTINEL_END: &str = "# portmap-end";

/// Represents the parsed state of /etc/hosts with portmap's managed block.
#[derive(Debug, Clone)]
pub struct HostsFile {
    /// Lines before the sentinel block (or all lines if no block exists).
    pub before: Vec<String>,
    /// Managed entries inside the sentinel block (just the mapping lines).
    pub entries: Vec<HostEntry>,
    /// Lines after the sentinel block.
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostEntry {
    pub ip: String,
    pub domain: String,
}

impl HostsFile {
    /// Parse a hosts file content string into structured form.
    pub fn parse(content: &str) -> Self {
        let lines: Vec<&str> = content.lines().collect();
        let mut before = Vec::new();
        let mut entries = Vec::new();
        let mut after = Vec::new();

        let start_idx = lines.iter().position(|l| l.trim() == SENTINEL_START);
        let end_idx = lines.iter().position(|l| l.trim() == SENTINEL_END);

        match (start_idx, end_idx) {
            (Some(start), Some(end)) if start < end => {
                // Lines before sentinel block
                for line in &lines[..start] {
                    before.push(line.to_string());
                }
                // Parse entries inside sentinel block
                for line in &lines[start + 1..end] {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        entries.push(HostEntry {
                            ip: parts[0].to_string(),
                            domain: parts[1].to_string(),
                        });
                    }
                }
                // Lines after sentinel block
                for line in &lines[end + 1..] {
                    after.push(line.to_string());
                }
            }
            _ => {
                // No valid sentinel block found — all lines are "before"
                for line in &lines {
                    before.push(line.to_string());
                }
            }
        }

        HostsFile {
            before,
            entries,
            after,
        }
    }

    /// Serialize back to a hosts file string.
    pub fn serialize(&self) -> String {
        let mut result = String::new();

        for line in &self.before {
            result.push_str(line);
            result.push('\n');
        }

        if !self.entries.is_empty() {
            result.push_str(SENTINEL_START);
            result.push('\n');
            for entry in &self.entries {
                result.push_str(&format!("{}\t{}", entry.ip, entry.domain));
                result.push('\n');
            }
            result.push_str(SENTINEL_END);
            result.push('\n');
        }

        for line in &self.after {
            result.push_str(line);
            result.push('\n');
        }

        result
    }

    /// Add an entry. Returns false if the domain already exists.
    pub fn add_entry(&mut self, domain: &str, ip: &str) -> bool {
        if self.entries.iter().any(|e| e.domain == domain) {
            return false;
        }
        self.entries.push(HostEntry {
            ip: ip.to_string(),
            domain: domain.to_string(),
        });
        true
    }

    /// Remove an entry by domain. Returns true if found and removed.
    pub fn remove_entry(&mut self, domain: &str) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.domain != domain);
        self.entries.len() < len_before
    }

    /// Remove all managed entries (for cleanup).
    pub fn remove_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_hosts() {
        let content = "127.0.0.1\tlocalhost\n::1\tlocalhost\n";
        let hosts = HostsFile::parse(content);
        assert_eq!(hosts.before.len(), 2);
        assert!(hosts.entries.is_empty());
        assert!(hosts.after.is_empty());
    }

    #[test]
    fn test_parse_with_sentinel_block() {
        let content = "\
127.0.0.1\tlocalhost
# portmap-start (DO NOT EDIT - managed by portmap)
127.0.0.1\tmy-project.localhost
127.0.0.1\tapi.localhost
# portmap-end
::1\tlocalhost
";
        let hosts = HostsFile::parse(content);
        assert_eq!(hosts.before, vec!["127.0.0.1\tlocalhost"]);
        assert_eq!(hosts.entries.len(), 2);
        assert_eq!(hosts.entries[0].domain, "my-project.localhost");
        assert_eq!(hosts.entries[1].domain, "api.localhost");
        assert_eq!(hosts.after, vec!["::1\tlocalhost"]);
    }

    #[test]
    fn test_roundtrip() {
        let original = "127.0.0.1\tlocalhost\n::1\tlocalhost\n";
        let mut hosts = HostsFile::parse(original);
        hosts.add_entry("test.localhost", "127.0.0.1");
        let serialized = hosts.serialize();
        let reparsed = HostsFile::parse(&serialized);
        assert_eq!(reparsed.entries.len(), 1);
        assert_eq!(reparsed.entries[0].domain, "test.localhost");
        assert!(reparsed.before.iter().any(|l| l.contains("localhost")));
    }

    #[test]
    fn test_add_duplicate() {
        let mut hosts = HostsFile::parse("");
        assert!(hosts.add_entry("test.localhost", "127.0.0.1"));
        assert!(!hosts.add_entry("test.localhost", "127.0.0.1"));
    }

    #[test]
    fn test_remove_entry() {
        let mut hosts = HostsFile::parse("");
        hosts.add_entry("test.localhost", "127.0.0.1");
        hosts.add_entry("api.localhost", "127.0.0.1");
        assert!(hosts.remove_entry("test.localhost"));
        assert_eq!(hosts.entries.len(), 1);
        assert_eq!(hosts.entries[0].domain, "api.localhost");
    }

    #[test]
    fn test_remove_all() {
        let mut hosts = HostsFile::parse("");
        hosts.add_entry("a.localhost", "127.0.0.1");
        hosts.add_entry("b.localhost", "127.0.0.1");
        hosts.remove_all();
        assert!(hosts.entries.is_empty());
        // Serializing with no entries should not include sentinel block
        let serialized = hosts.serialize();
        assert!(!serialized.contains("portmap-start"));
    }
}
