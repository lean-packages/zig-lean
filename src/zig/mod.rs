use anyhow::Result;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ZigToolchain {
    pub exe_path: String,
    pub version: String,
}

impl ZigToolchain {
    pub fn detect(custom_exe: Option<&str>) -> Option<Self> {
        let exe = custom_exe.unwrap_or("zig");
        let output = Command::new(exe).arg("version").output().ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(Self {
                exe_path: exe.to_string(),
                version,
            })
        } else {
            None
        }
    }

    pub fn check_fmt(&self, manifest_path: &Path) -> Result<bool> {
        let status = Command::new(&self.exe_path)
            .arg("fmt")
            .arg("--check")
            .arg(manifest_path)
            .status()?;

        Ok(status.success())
    }
}
