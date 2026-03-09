use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::{Command, Output};

use super::diff::{ChangedFile, FileStatus};

pub fn git_changed_files(repo_path: &Path) -> Result<Vec<ChangedFile>> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .current_dir(repo_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }

        let x = line.as_bytes()[0] as char;
        let y = line.as_bytes()[1] as char;
        let path_part = &line[3..];
        let path = if let Some((_, new_path)) = path_part.split_once(" -> ") {
            new_path
        } else {
            path_part
        };

        let status = parse_porcelain_status(x, y);
        files.push(ChangedFile {
            path: path.to_owned(),
            status,
            staged: is_index_changed(x, y),
            unstaged: is_worktree_changed(x, y),
        });
    }

    Ok(files)
}

fn parse_porcelain_status(x: char, y: char) -> FileStatus {
    if x == '?' && y == '?' {
        return FileStatus::Untracked;
    }
    if x == 'R' || y == 'R' {
        return FileStatus::Renamed;
    }
    if x == 'A' || y == 'A' {
        return FileStatus::Added;
    }
    if x == 'D' || y == 'D' {
        return FileStatus::Deleted;
    }
    if x == 'M' || y == 'M' || x == 'T' || y == 'T' {
        return FileStatus::Modified;
    }
    FileStatus::Other
}

fn is_index_changed(x: char, y: char) -> bool {
    if x == '?' && y == '?' {
        return false;
    }

    x != ' ' && x != '?'
}

fn is_worktree_changed(x: char, y: char) -> bool {
    if x == '?' && y == '?' {
        return true;
    }

    y != ' ' && y != '?'
}

/// Run `git diff` for a specific file and return the unified diff output.
///
/// Uses `git diff HEAD` as a single call combining staged+unstaged changes.
/// Falls back to `--cached` for new repos without HEAD, and `--no-index` for untracked files.
pub fn git_diff_file(repo_path: &Path, file_path: &str) -> Result<String> {
    // Try combined staged+unstaged diff against HEAD
    let output = Command::new("git")
        .args(["diff", "HEAD", "--", file_path])
        .current_dir(repo_path)
        .output()?;

    if output.status.success() {
        let combined = String::from_utf8_lossy(&output.stdout).into_owned();
        if !combined.is_empty() {
            return Ok(combined);
        }
    }

    // HEAD may not exist (new repo) — fall back to staged-only diff
    let output = Command::new("git")
        .args(["diff", "--cached", "--", file_path])
        .current_dir(repo_path)
        .output()?;

    let staged = String::from_utf8_lossy(&output.stdout).into_owned();
    if !staged.is_empty() {
        return Ok(staged);
    }

    // Untracked file — show full content as added
    let null_path = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let output = Command::new("git")
        .args(["diff", "--no-index", "--", null_path, file_path])
        .current_dir(repo_path)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn git_stage_paths(repo_path: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git_command(repo_path, &args)
}

pub fn git_stage_all(repo_path: &Path) -> Result<()> {
    run_git_command(repo_path, &["add", "--all", "--", "."])
}

pub fn git_unstage_paths(repo_path: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    if git_has_head(repo_path)? {
        let mut args = vec!["restore", "--staged", "--"];
        args.extend(paths.iter().map(String::as_str));
        run_git_command(repo_path, &args)
    } else {
        let mut args = vec!["rm", "--cached", "--"];
        args.extend(paths.iter().map(String::as_str));
        run_git_command(repo_path, &args)
    }
}

pub fn git_unstage_all(repo_path: &Path) -> Result<()> {
    if git_has_head(repo_path)? {
        run_git_command(repo_path, &["restore", "--staged", "--", "."])
    } else {
        run_git_command(repo_path, &["rm", "--cached", "-r", "--", "."])
    }
}

pub fn git_commit(repo_path: &Path, summary: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["commit", "-m", summary])
        .current_dir(repo_path)
        .output()
        .context("failed to run git commit")?;

    ensure_success(&output, "git commit")?;

    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("failed to resolve HEAD after commit")?;

    ensure_success(&output, "git rev-parse --short HEAD")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_has_head(repo_path: &Path) -> Result<bool> {
    let status = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo_path)
        .status()
        .context("failed to query HEAD")?;
    Ok(status.success())
}

fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    ensure_success(&output, &format!("git {}", args.join(" ")))
}

fn ensure_success(output: &Output, command: &str) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("{command} exited with status {}", output.status)
    };

    Err(anyhow!(details))
}

/// List local branch names and identify the current branch.
/// Returns `(branches, current_branch)`.
pub fn git_list_branches(repo_path: &Path) -> Result<(Vec<String>, String)> {
    let output = Command::new("git")
        .args(["branch", "--sort=-committerdate", "--format=%(refname:short)"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git branch")?;

    ensure_success(&output, "git branch")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect();

    let current = git_current_branch(repo_path)?;
    Ok((branches, current))
}

/// Get the current branch name.
pub fn git_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("failed to get current branch")?;

    ensure_success(&output, "git rev-parse --abbrev-ref HEAD")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Switch to the given branch.
pub fn git_switch_branch(repo_path: &Path, branch: &str) -> Result<()> {
    run_git_command(repo_path, &["switch", branch])
}

/// Fetch commit log entries as `(hash, author, relative_date, subject)` tuples.
pub fn git_log(
    repo_path: &Path,
    count: usize,
    skip: usize,
) -> Result<Vec<(String, String, String, String)>> {
    let output = Command::new("git")
        .args([
            "log",
            "--format=%H%x00%an%x00%ar%x00%s",
            &format!("-n{count}"),
            &format!("--skip={skip}"),
        ])
        .current_dir(repo_path)
        .output()
        .context("failed to run git log")?;

    ensure_success(&output, "git log")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(4, '\0').collect();
        if parts.len() == 4 {
            entries.push((
                parts[0].to_owned(),
                parts[1].to_owned(),
                parts[2].to_owned(),
                parts[3].to_owned(),
            ));
        }
    }
    Ok(entries)
}

/// List files changed in a given commit as `(status_char, path)` tuples.
pub fn git_commit_files(repo_path: &Path, sha: &str) -> Result<Vec<(char, String)>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "-r", "--name-status", sha])
        .current_dir(repo_path)
        .output()
        .context("failed to run git diff-tree")?;

    ensure_success(&output, "git diff-tree")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "M\tpath" or "A\tpath"
        let status_char = line.as_bytes()[0] as char;
        let path = line.get(2..).unwrap_or("").to_owned();
        files.push((status_char, path));
    }
    Ok(files)
}

/// Get the diff for a single file within a specific commit.
///
/// Falls back to `git show` for the initial commit where `sha^` does not exist.
pub fn git_diff_commit_file(repo_path: &Path, sha: &str, file_path: &str) -> Result<String> {
    // Try normal parent..commit diff first
    let output = Command::new("git")
        .args([
            "diff",
            &format!("{sha}^..{sha}"),
            "--",
            file_path,
        ])
        .current_dir(repo_path)
        .output()
        .context("failed to run git diff for commit file")?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    // Likely the initial commit (no parent) — fall back to git show
    let output = Command::new("git")
        .args(["show", "--format=", sha, "--", file_path])
        .current_dir(repo_path)
        .env("GIT_PAGER", "cat")
        .output()
        .context("failed to run git show for initial commit file")?;

    ensure_success(&output, "git show (initial commit fallback)")?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
