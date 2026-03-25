#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::ffi::{OsStr, OsString};
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStringExt;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
pub fn bootstrap_shell_environment() {
    eprintln!("[anduin] bootstrapping shell environment");

    let shell = resolve_shell();
    eprintln!("[anduin] shell env using {}", shell.display());

    match capture_shell_environment(&shell) {
        Ok(vars) => {
            let imported = apply_environment(vars);
            eprintln!("[anduin] shell env imported {imported} variables");

            match Command::new("git").arg("--version").output() {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    eprintln!("[anduin] git available after shell import: {version}");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                    if stderr.is_empty() {
                        eprintln!(
                            "[anduin] warning: git still unavailable after shell import (status {})",
                            output.status
                        );
                    } else {
                        eprintln!(
                            "[anduin] warning: git still unavailable after shell import: {stderr}"
                        );
                    }
                }
                Err(error) => {
                    eprintln!("[anduin] warning: failed to probe git after shell import: {error}");
                }
            }
        }
        Err(error) => {
            eprintln!("[anduin] warning: failed to import shell environment: {error}");
        }
    }
}

#[cfg(target_os = "macos")]
fn resolve_shell() -> PathBuf {
    env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.exists())
        .or_else(|| {
            ["/bin/zsh", "/bin/bash", "/usr/bin/zsh", "/usr/bin/bash"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.exists())
        })
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

#[cfg(target_os = "macos")]
fn capture_shell_environment(shell: &Path) -> Result<Vec<(OsString, OsString)>, String> {
    let args = command_args_for_shell(shell);
    let output = Command::new(shell)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", shell.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let details = if stderr.is_empty() {
            format!("{} exited with status {}", shell.display(), output.status)
        } else {
            stderr
        };
        return Err(details);
    }

    Ok(parse_env_output(&output.stdout))
}

#[cfg(target_os = "macos")]
fn command_args_for_shell(_shell: &Path) -> [&'static str; 3] {
    ["-l", "-c", "env -0"]
}

#[cfg(target_os = "macos")]
fn parse_env_output(bytes: &[u8]) -> Vec<(OsString, OsString)> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let equals = entry.iter().position(|byte| *byte == b'=')?;
            let (name, value) = entry.split_at(equals);
            let value = &value[1..];
            Some((
                OsString::from_vec(name.to_vec()),
                OsString::from_vec(value.to_vec()),
            ))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn apply_environment(vars: Vec<(OsString, OsString)>) -> usize {
    let mut imported = 0;

    for (name, value) in vars {
        if !should_import_var(&name) {
            continue;
        }

        // SAFETY: This runs once during startup on macOS before the app creates
        // background tasks or worker threads, so mutating process environment is
        // confined to single-threaded initialization.
        unsafe {
            env::set_var(&name, &value);
        }
        imported += 1;
    }

    imported
}

#[cfg(target_os = "macos")]
fn should_import_var(name: &OsStr) -> bool {
    !matches!(name.to_str(), Some("PWD" | "OLDPWD" | "SHLVL" | "_"))
}

#[cfg(not(target_os = "macos"))]
pub fn bootstrap_shell_environment() {}
