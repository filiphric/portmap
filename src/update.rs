use anyhow::{anyhow, Result};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn check_for_update() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // Fetch latest release info from GitHub
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "https://api.github.com/repos/filiphric/portmap/releases/latest",
        ])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("failed to fetch latest release"));
    }

    let body = String::from_utf8_lossy(&output.stdout);

    // Extract tag_name with simple string search
    let tag = body
        .lines()
        .find(|line| line.contains("\"tag_name\""))
        .and_then(|line| {
            let after_key = &line[line.find("\"tag_name\"")? + "\"tag_name\"".len()..];
            // Skip colon and whitespace, find opening quote of value
            let after_colon = after_key.find('"')?;
            let value_start = &after_key[after_colon + 1..];
            let end = value_start.find('"')?;
            Some(value_start[..end].to_string())
        })
        .ok_or_else(|| anyhow!("tag_name not found in response"))?;

    let latest = tag.strip_prefix('v').unwrap_or(&tag);

    if latest == current {
        return Ok(());
    }

    println!("Updating portmap {current} → {latest}...");

    // Detect architecture
    let uname_output = Command::new("uname").arg("-m").output()?;
    let arch_raw = String::from_utf8_lossy(&uname_output.stdout).trim().to_string();
    let arch = match arch_raw.as_str() {
        "arm64" => "aarch64-apple-darwin",
        "x86_64" => "x86_64-apple-darwin",
        other => return Err(anyhow!("unsupported architecture: {other}")),
    };

    let tarball_url = format!(
        "https://github.com/filiphric/portmap/releases/download/{tag}/portmap-{arch}.tar.gz"
    );

    let tmp_dir = std::env::temp_dir().join("portmap-update");
    std::fs::create_dir_all(&tmp_dir)?;

    let tarball_path = tmp_dir.join("portmap.tar.gz");

    // Download tarball
    let status = Command::new("curl")
        .args([
            "-fsSL",
            &tarball_url,
            "-o",
            tarball_path.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!("failed to download release tarball"));
    }

    // Extract tarball
    let status = Command::new("tar")
        .args([
            "xzf",
            tarball_path.to_str().unwrap(),
            "-C",
            tmp_dir.to_str().unwrap(),
        ])
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(anyhow!("failed to extract tarball"));
    }

    // Replace current binary
    let exe_path = std::env::current_exe()?;
    let new_binary = tmp_dir.join("portmap");
    std::fs::rename(&new_binary, &exe_path)?;

    // Clean up
    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("Updated to {latest}. Restarting...");

    // Re-exec with same args
    let err = Command::new(&exe_path)
        .args(std::env::args().skip(1))
        .exec();

    // exec() only returns on error
    Err(anyhow!("failed to re-exec: {err}"))
}
