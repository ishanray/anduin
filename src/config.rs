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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    GitHubLight,
    #[default]
    GitHubDark,
    Light,
    Dark,
    Dracula,
    Nord,
    SolarizedLight,
    SolarizedDark,
    GruvboxLight,
    GruvboxDark,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
    KanagawaWave,
    KanagawaDragon,
    KanagawaLotus,
    Moonfly,
    Nightfly,
    Oxocarbon,
    Ferra,
}

impl AppTheme {
    pub fn is_dark(self) -> bool {
        !matches!(
            self,
            Self::GitHubLight
                | Self::Light
                | Self::SolarizedLight
                | Self::GruvboxLight
                | Self::CatppuccinLatte
                | Self::TokyoNightLight
                | Self::KanagawaLotus
        )
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::GitHubLight => "GitHub Light",
            Self::GitHubDark => "GitHub Dark",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::SolarizedLight => "Solarized Light",
            Self::SolarizedDark => "Solarized Dark",
            Self::GruvboxLight => "Gruvbox Light",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::CatppuccinFrappe => "Catppuccin Frappé",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::TokyoNight => "Tokyo Night",
            Self::TokyoNightStorm => "Tokyo Night Storm",
            Self::TokyoNightLight => "Tokyo Night Light",
            Self::KanagawaWave => "Kanagawa Wave",
            Self::KanagawaDragon => "Kanagawa Dragon",
            Self::KanagawaLotus => "Kanagawa Lotus",
            Self::Moonfly => "Moonfly",
            Self::Nightfly => "Nightfly",
            Self::Oxocarbon => "Oxocarbon",
            Self::Ferra => "Ferra",
        }
    }

    pub fn all_light() -> &'static [Self] {
        &[
            Self::GitHubLight,
            Self::Light,
            Self::SolarizedLight,
            Self::GruvboxLight,
            Self::CatppuccinLatte,
            Self::TokyoNightLight,
            Self::KanagawaLotus,
        ]
    }

    pub fn all_dark() -> &'static [Self] {
        &[
            Self::GitHubDark,
            Self::Dark,
            Self::Dracula,
            Self::Nord,
            Self::SolarizedDark,
            Self::GruvboxDark,
            Self::CatppuccinFrappe,
            Self::CatppuccinMacchiato,
            Self::CatppuccinMocha,
            Self::TokyoNight,
            Self::TokyoNightStorm,
            Self::KanagawaWave,
            Self::KanagawaDragon,
            Self::Moonfly,
            Self::Nightfly,
            Self::Oxocarbon,
            Self::Ferra,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub current_theme: AppTheme,
    #[serde(default = "default_light_theme")]
    pub last_light_theme: AppTheme,
    #[serde(default = "default_dark_theme")]
    pub last_dark_theme: AppTheme,
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

fn default_light_theme() -> AppTheme {
    AppTheme::GitHubLight
}

fn default_dark_theme() -> AppTheme {
    AppTheme::GitHubDark
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
