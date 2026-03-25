use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const QUALIFIER: &str = "dev";
const ORGANIZATION: &str = "anduin";
const APPLICATION: &str = "anduin";

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .context("failed to determine platform config directories")
}

pub fn settings_path() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("settings.toml"))
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path.parent().context("path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub recent_repos: Vec<String>,
    #[serde(default)]
    pub window_width: Option<f32>,
    #[serde(default)]
    pub window_height: Option<f32>,
    #[serde(default)]
    pub zoom_level: Option<f32>,
}

pub fn load_settings() -> Result<Settings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(Settings::default());
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let settings =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(settings)
}

#[allow(dead_code)]
const MAX_RECENT_REPOS: usize = 20;

#[allow(dead_code)]
pub fn push_recent_repo(settings: &mut Settings, repo_path: &str) {
    settings.recent_repos.retain(|p| p != repo_path);
    settings.recent_repos.insert(0, repo_path.to_owned());
    settings.recent_repos.truncate(MAX_RECENT_REPOS);
}

pub fn save_settings(settings: &Settings) -> Result<()> {
    let path = settings_path()?;
    ensure_parent_dir(&path)?;
    let text = toml::to_string_pretty(settings).context("failed to serialize settings")?;
    fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
