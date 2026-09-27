use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn command_output<S: AsRef<OsStr>>(
    program: &str,
    args: &[S],
    cwd: Option<&Path>,
) -> Result<String> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .with_context(|| format!("start {program}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        bail!("{program} failed ({}): {detail}", output.status);
    }
    Ok(String::from_utf8(output.stdout).context("command output was not UTF-8")?)
}

pub fn sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn json_string(value: &str) -> Result<String> {
    Ok(serde_json::to_string(value).context("encode CUE string")?)
}
