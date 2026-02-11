/// A single domain → port mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    /// Full domain, e.g. "my-project.localhost"
    pub domain: String,
    /// Target port on localhost
    pub port: u16,
    /// Whether the port is reachable
    pub status: MappingStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MappingStatus {
    Active,
    PortUnreachable,
    /// Not yet checked
    Unknown,
}

impl std::fmt::Display for MappingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappingStatus::Active => write!(f, "Active"),
            MappingStatus::PortUnreachable => write!(f, "Port Unreachable"),
            MappingStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The current mode of the TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal mode — navigating the table
    Normal,
    /// Adding a new mapping (popup visible)
    Adding,
}

/// Which field is focused in the add-mapping popup.
#[derive(Debug, Clone, PartialEq)]
pub enum PopupField {
    Domain,
    Port,
}

/// State for the TUI (not shared with the proxy — the proxy uses the watch channel).
pub struct TuiState {
    /// Currently selected row index
    pub selected: usize,
    /// Current input mode
    pub mode: InputMode,
    /// Domain input buffer (without .localhost suffix)
    pub domain_input: String,
    /// Port input buffer
    pub port_input: String,
    /// Currently focused popup field
    pub popup_field: PopupField,
    /// Status message shown in the status bar
    pub status_message: Option<String>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            mode: InputMode::Normal,
            domain_input: String::new(),
            port_input: String::new(),
            popup_field: PopupField::Domain,
            status_message: None,
        }
    }
}
