use anyhow::Result;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
    pub staged: bool,
    pub unstaged: bool,
}

impl ChangedFile {
    pub fn is_staged(&self) -> bool {
        self.staged
    }

    pub fn is_unstaged(&self) -> bool {
        self.unstaged
    }
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub raw_patch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Untracked,
    Other,
}

impl Display for FileStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Added => write!(f, "A"),
            Self::Deleted => write!(f, "D"),
            Self::Modified => write!(f, "M"),
            Self::Renamed => write!(f, "R"),
            Self::Untracked => write!(f, "?"),
            Self::Other => write!(f, "!"),
        }
    }
}

/// Discover the repo root from a path using gix.
pub fn find_repo_root(path: &Path) -> Result<PathBuf> {
    let repo = gix::discover(path)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("bare repository, no working directory"))?;
    Ok(workdir.to_path_buf())
}

/// Get the list of changed files.
/// We use `git status --porcelain` here because it exactly matches user-visible Git behavior.
pub fn get_changed_files(repo_path: &Path) -> Result<Vec<ChangedFile>> {
    let mut files = super::cli::git_changed_files(repo_path)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);
    Ok(files)
}

/// Parse unified diff output with `unidiff`, but always keep sanitized patch text for display.
pub fn parse_unified_diff(raw_diff: &str, path: &str, _status: FileStatus) -> FileDiff {
    let mut patch_set = unidiff::PatchSet::new();
    let _ = patch_set.parse(raw_diff);

    // Replace tabs with spaces for stable display in the viewer.
    let display_patch = raw_diff.replace('\t', "    ");

    FileDiff {
        path: path.to_owned(),
        raw_patch: display_patch,
    }
}
