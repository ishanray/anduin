use crate::actions::load_changed_files;
use crate::config;
use crate::git::diff::{ChangedFile, FileDiff};
use crate::search::{ProjectSearchResult, SEARCH_DEBOUNCE_MS};
use crate::theme;
use crate::tree::{SidebarRow, TreeDir, expand_parent_dirs};
use crate::watch;
use iced::keyboard;
use iced::widget::Id;
use iced::{Task, Theme};
use iced_code_editor::Message as EditorMessage;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusTone {
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct StatusMessage {
    pub(crate) text: String,
    pub(crate) tone: StatusTone,
}

#[derive(Debug, Clone)]
pub(crate) struct CommitComposer {
    pub(crate) summary: String,
    pub(crate) input_id: Id,
    pub(crate) submitting: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BranchPicker {
    pub(crate) branches: Vec<String>,
    pub(crate) current: String,
    pub(crate) filter: String,
    pub(crate) selected_index: usize,
    pub(crate) error: Option<String>,
    pub(crate) input_id: Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SidebarTarget {
    Root,
    Dir(String),
    File(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivePane {
    Sidebar,
    Diff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTab {
    Changes,
    #[allow(dead_code)] // will be used in Task 3
    History,
}

#[allow(dead_code)] // will be used in Task 3+
#[derive(Debug, Clone)]
pub(crate) struct Commit {
    pub(crate) hash: String,
    pub(crate) short_hash: String,
    pub(crate) author: String,
    pub(crate) date: String,
    pub(crate) message: String,
}

pub(crate) struct State {
    pub(crate) repo_path: PathBuf,
    pub(crate) files: Vec<ChangedFile>,
    pub(crate) selected_file: Option<usize>,
    pub(crate) selected_path: Option<String>,
    pub(crate) focused_sidebar_target: Option<SidebarTarget>,
    pub(crate) selected_sidebar_targets: HashSet<SidebarTarget>,
    pub(crate) selection_anchor_sidebar_target: Option<SidebarTarget>,
    pub(crate) current_diff: Option<FileDiff>,
    pub(crate) diff_editor: iced_code_editor::CodeEditor,
    pub(crate) theme_mode: ThemeMode,
    pub(crate) error: Option<String>,
    pub(crate) status_message: Option<StatusMessage>,
    pub(crate) commit_composer: Option<CommitComposer>,
    pub(crate) expanded_dirs: HashSet<String>,
    pub(crate) tree_root_expanded: bool,
    pub(crate) alt_pressed: bool,
    pub(crate) initialized_tree: bool,
    pub(crate) cached_rows: Vec<SidebarRow>,
    pub(crate) tree_dirty: bool,
    pub(crate) refresh_in_flight: bool,
    pub(crate) refresh_queued: bool,
    pub(crate) active_diff_request: u64,
    pub(crate) diff_search_cache: HashMap<String, DiffSearchCacheEntry>,
    pub(crate) scroll_positions: HashMap<String, f32>,
    pub(crate) project_search: Option<ProjectSearch>,
    pub(crate) pending_diff_jump: Option<PendingDiffJump>,
    pub(crate) sidebar_scroll_id: Id,
    pub(crate) sidebar_scroll_offset: f32,
    pub(crate) sidebar_viewport_height: f32,
    pub(crate) active_pane: ActivePane,
    pub(crate) cached_theme: Theme,
    pub(crate) show_shortcuts_help: bool,
    pub(crate) current_branch: Option<String>,
    pub(crate) branch_picker: Option<BranchPicker>,
    pub(crate) project_picker: Option<ProjectPicker>,
    pub(crate) recent_repos: Vec<String>,
    #[allow(dead_code)] // will be used in Task 3
    pub(crate) sidebar_tab: SidebarTab,
    #[allow(dead_code)] // will be used in Task 4
    pub(crate) commits: Vec<Commit>,
    #[allow(dead_code)] // will be used in Task 4
    pub(crate) selected_commit: Option<usize>,
    #[allow(dead_code)] // will be used in Task 5
    pub(crate) commit_files: Vec<ChangedFile>,
    #[allow(dead_code)] // will be used in Task 4
    pub(crate) commits_loading: bool,
    #[allow(dead_code)] // will be used in Task 4
    pub(crate) commits_exhausted: bool,
    #[allow(dead_code)] // will be used in Task 5
    pub(crate) history_selected_file: Option<usize>,
    #[allow(dead_code)] // will be used in Task 5
    pub(crate) history_selected_path: Option<String>,
    #[allow(dead_code)] // will be used in Task 5
    pub(crate) history_diff: Option<FileDiff>,
    #[allow(dead_code)] // will be used in Task 5
    pub(crate) history_commit_header: Option<Commit>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffSearchCacheEntry {
    pub(crate) raw_diff: Arc<str>,
    pub(crate) raw_diff_lower: Option<Arc<str>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectSearchResponse {
    pub(crate) results: Vec<ProjectSearchResult>,
    pub(crate) cache: HashMap<String, DiffSearchCacheEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectSearch {
    pub(crate) query: String,
    pub(crate) case_sensitive: bool,
    pub(crate) query_lower: String,
    pub(crate) results: Vec<ProjectSearchResult>,
    pub(crate) matching_paths: HashSet<String>,
    pub(crate) result_index_by_path: HashMap<String, usize>,
    pub(crate) searching: bool,
    pub(crate) request_id: u64,
    pub(crate) pending_run_at: Option<Instant>,
    pub(crate) is_open: bool,
    pub(crate) input_focused: bool,
    pub(crate) input_id: Id,
    pub(crate) cached_summary: String,
    pub(crate) cached_file_summary: String,
    pub(crate) cached_result_summary: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDiffJump {
    pub(crate) path: String,
    pub(crate) line_number: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    FilesLoaded(Result<Vec<ChangedFile>, String>),
    SelectFile(usize),
    ToggleRoot(bool),
    ToggleDir(String, bool),
    DiffLoaded(u64, Result<FileDiff, String>),
    DiffEditor(EditorMessage),
    ToggleTheme,
    OpenRepo,
    RepoOpened(Option<PathBuf>),
    WatchEvent(watch::Event),
    KeyboardEvent(keyboard::Event),
    SidebarScrolled(f32, f32),
    OpenProjectSearch,
    CloseProjectSearch,
    OpenCommitComposer,
    CloseCommitComposer,
    CommitSummaryChanged(String),
    SubmitCommit,
    GitOperationFinished(Result<String, String>),
    CommitFinished(Result<String, String>),
    ProjectSearchQueryChanged(String),
    ProjectSearchToggleCase,
    ProjectSearchTick,
    ProjectSearchResults(u64, Result<ProjectSearchResponse, String>),
    ProjectSearchJumpTo(String, usize),
    ToggleShortcutsHelp,
    OpenBranchPicker,
    BranchesFetched(Result<(Vec<String>, String), String>),
    BranchPickerFilterChanged(String),
    SwitchBranch(String),
    BranchSwitched(Result<(), String>),
    CurrentBranchFetched(Result<String, String>),
    OpenProjectPicker,
    #[allow(dead_code)]
    CloseProjectPicker,
    ProjectPickerFilterChanged(String),
    SwitchProject(String),
    #[allow(dead_code)]
    SwitchSidebarTab(SidebarTab),
    #[allow(dead_code)]
    CommitsLoaded(Result<Vec<Commit>, String>),
    #[allow(dead_code)]
    SelectCommit(usize),
    #[allow(dead_code)]
    CommitFilesLoaded(Result<Vec<ChangedFile>, String>),
    #[allow(dead_code)]
    SelectHistoryFile(usize),
    #[allow(dead_code)]
    HistoryDiffLoaded(u64, Result<FileDiff, String>),
    #[allow(dead_code)]
    LoadMoreCommits,
    #[allow(dead_code)]
    CopyCommitHash(String),
}

impl ThemeMode {
    pub(crate) fn from_preference(preference: config::ThemePreference) -> Self {
        match preference {
            config::ThemePreference::Dark => Self::Dark,
            config::ThemePreference::Light => Self::Light,
            config::ThemePreference::System => Self::Dark,
        }
    }

    pub(crate) fn preference(self) -> config::ThemePreference {
        match self {
            Self::Dark => config::ThemePreference::Dark,
            Self::Light => config::ThemePreference::Light,
        }
    }

    pub(crate) fn app_theme(self) -> Theme {
        match self {
            Self::Dark => theme::github_dark(),
            Self::Light => theme::github_light(),
        }
    }

    pub(crate) fn toggle(&mut self) {
        *self = match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        };
    }

    pub(crate) fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

impl CommitComposer {
    pub(crate) fn new() -> Self {
        Self {
            summary: String::new(),
            input_id: Id::unique(),
            submitting: false,
            error: None,
        }
    }

    pub(crate) fn can_submit(&self, staged_count: usize) -> bool {
        !self.submitting && staged_count > 0 && !self.summary.trim().is_empty()
    }
}

impl BranchPicker {
    pub(crate) fn new(branches: Vec<String>, current: String) -> Self {
        Self {
            branches,
            current,
            filter: String::new(),
            selected_index: 0,
            error: None,
            input_id: Id::unique(),
        }
    }

    pub(crate) fn filtered_branches(&self) -> Vec<&str> {
        if self.filter.is_empty() {
            return self.branches.iter().map(String::as_str).collect();
        }

        let filter_lower = self.filter.to_lowercase();

        // Partition into prefix matches and other substring matches,
        // preserving recency order within each group.
        let mut prefix = Vec::new();
        let mut rest = Vec::new();

        for b in &self.branches {
            let lower = b.to_lowercase();
            if lower.starts_with(&filter_lower) {
                prefix.push(b.as_str());
            } else if lower.contains(&filter_lower) {
                rest.push(b.as_str());
            }
        }

        prefix.extend(rest);
        prefix
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectPicker {
    pub(crate) repos: Vec<String>,
    pub(crate) current: String,
    pub(crate) filter: String,
    pub(crate) selected_index: usize,
    pub(crate) input_id: Id,
}

impl ProjectPicker {
    pub(crate) fn new(repos: Vec<String>, current: String) -> Self {
        Self {
            repos,
            current,
            filter: String::new(),
            selected_index: 0,
            input_id: Id::unique(),
        }
    }

    pub(crate) fn filtered_repos(&self) -> Vec<&str> {
        let non_current: Vec<&str> = self
            .repos
            .iter()
            .filter(|r| *r != &self.current)
            .map(String::as_str)
            .collect();

        if self.filter.is_empty() {
            return non_current;
        }

        let filter_lower = self.filter.to_lowercase();
        let mut prefix = Vec::new();
        let mut rest = Vec::new();

        for repo in non_current {
            let name = repo.rsplit('/').next().unwrap_or(repo);
            let name_lower = name.to_lowercase();
            let path_lower = repo.to_lowercase();

            if name_lower.starts_with(&filter_lower) {
                prefix.push(repo);
            } else if path_lower.contains(&filter_lower) {
                rest.push(repo);
            }
        }

        prefix.extend(rest);
        prefix
    }
}

impl ProjectSearch {
    pub(crate) fn new() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            query_lower: String::new(),
            results: Vec::new(),
            matching_paths: HashSet::new(),
            result_index_by_path: HashMap::new(),
            searching: false,
            request_id: 0,
            pending_run_at: None,
            is_open: true,
            input_focused: true,
            input_id: Id::unique(),
            cached_summary: String::new(),
            cached_file_summary: "0 files".to_owned(),
            cached_result_summary: String::new(),
        }
    }

    pub(crate) fn set_results(&mut self, results: Vec<ProjectSearchResult>) {
        self.matching_paths = results
            .iter()
            .map(|result| result.file_path.clone())
            .collect();
        self.result_index_by_path = results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.file_path.clone(), i))
            .collect();
        self.results = results;
        self.searching = false;
        self.rebuild_cached_summaries();
    }

    pub(crate) fn clear_results(&mut self) {
        self.results.clear();
        self.matching_paths.clear();
        self.result_index_by_path.clear();
        self.searching = false;
        self.rebuild_cached_summaries();
    }

    pub(crate) fn update_query_lower(&mut self) {
        self.query_lower = if self.case_sensitive {
            String::new()
        } else {
            self.query.to_lowercase()
        };
    }

    pub(crate) fn rebuild_cached_summaries(&mut self) {
        if self.query.is_empty() {
            self.cached_summary = "Search all diffs".to_owned();
            self.cached_file_summary = "0 files".to_owned();
            self.cached_result_summary = String::new();
            return;
        }

        if self.searching {
            self.cached_summary = format!("Searching for {:?}…", self.query);
            self.cached_file_summary = "0 files".to_owned();
            self.cached_result_summary = String::new();
            return;
        }

        let file_count = self.results.len();
        let match_count: usize = self.results.iter().map(|r| r.total_matches).sum();
        self.cached_summary = format!(
            "{:?} • {} file{} • {} match{}",
            self.query,
            file_count,
            if file_count == 1 { "" } else { "s" },
            match_count,
            if match_count == 1 { "" } else { "es" }
        );

        self.cached_file_summary = format!(
            "{} file{}",
            file_count,
            if file_count == 1 { "" } else { "s" }
        );

        let context_count: usize = self.results.iter().map(|r| r.matches.len()).sum();
        self.cached_result_summary = format!(
            "{} context block{} • {} total match{}",
            context_count,
            if context_count == 1 { "" } else { "s" },
            match_count,
            if match_count == 1 { "" } else { "es" }
        );
    }
}

impl State {
    pub(crate) fn is_search_open(&self) -> bool {
        self.project_search
            .as_ref()
            .is_some_and(|s| s.is_open)
    }

    pub(crate) fn is_branch_picker_open(&self) -> bool {
        self.branch_picker.is_some()
    }

    pub(crate) fn is_project_picker_open(&self) -> bool {
        self.project_picker.is_some()
    }

    pub(crate) fn app_theme(&self) -> Theme {
        self.cached_theme.clone()
    }

    pub(crate) fn persist_settings(&self) {
        let settings = config::Settings {
            theme: self.theme_mode.preference(),
            repo_path: Some(self.repo_path.to_string_lossy().into_owned()),
            recent_repos: self.recent_repos.clone(),
        };

        if let Err(error) = config::save_settings(&settings) {
            eprintln!("[anduin] warning: failed to save settings: {error}");
        }
    }

    pub(crate) fn staged_file_count(&self) -> usize {
        self.files.iter().filter(|file| file.is_staged()).count()
    }

    pub(crate) fn unstaged_file_count(&self) -> usize {
        self.files.iter().filter(|file| file.is_unstaged()).count()
    }

    pub(crate) fn selected_file_count(&self) -> usize {
        self.selected_sidebar_targets.len()
    }

    pub(crate) fn has_explicit_selection(&self) -> bool {
        !self.selected_sidebar_targets.is_empty()
    }

    pub(crate) fn is_sidebar_target_selected(&self, target: &SidebarTarget) -> bool {
        self.selected_sidebar_targets.contains(target)
    }

    pub(crate) fn clear_explicit_selection(&mut self) {
        self.selected_sidebar_targets.clear();
        self.selection_anchor_sidebar_target = None;
    }

    pub(crate) fn sidebar_target_for_row(&self, row: &SidebarRow) -> SidebarTarget {
        match row {
            SidebarRow::Root { .. } => SidebarTarget::Root,
            SidebarRow::Dir { path, .. } => SidebarTarget::Dir(path.clone()),
            SidebarRow::File { index, .. } => self
                .files
                .get(*index)
                .map(|file| SidebarTarget::File(file.path.clone()))
                .unwrap_or(SidebarTarget::Root),
        }
    }

    /// Returns the cached rows visible in the sidebar, filtering by search
    /// results when project search is active.
    pub(crate) fn visible_cached_rows(&self) -> Vec<&SidebarRow> {
        let search_filter = self
            .project_search
            .as_ref()
            .filter(|s| s.is_open && !s.query.is_empty() && !s.matching_paths.is_empty());

        if let Some(search) = search_filter {
            // Compute matching dirs (ancestors of matching file paths)
            let mut matching_dirs = HashSet::<String>::new();
            for path in &search.matching_paths {
                let mut current = path.as_str();
                while let Some(pos) = current.rfind('/') {
                    current = &current[..pos];
                    if !matching_dirs.insert(current.to_owned()) {
                        break;
                    }
                }
            }

            self.cached_rows
                .iter()
                .filter(|row| match row {
                    SidebarRow::Root { .. } => true,
                    SidebarRow::Dir { path, .. } => matching_dirs.contains(path),
                    SidebarRow::File { index, .. } => self
                        .files
                        .get(*index)
                        .is_some_and(|f| search.matching_paths.contains(&f.path)),
                })
                .collect()
        } else {
            self.cached_rows.iter().collect()
        }
    }

    pub(crate) fn visible_sidebar_targets(&self) -> Vec<SidebarTarget> {
        self.visible_cached_rows()
            .iter()
            .map(|row| self.sidebar_target_for_row(row))
            .collect()
    }

    pub(crate) fn focused_sidebar_row_index(&self) -> Option<usize> {
        let target = self.focused_sidebar_target.as_ref()?;
        self.visible_cached_rows()
            .iter()
            .position(|row| self.sidebar_target_for_row(row) == *target)
    }

    pub(crate) fn retain_sidebar_selection(&mut self) {
        let visible_targets: HashSet<SidebarTarget> =
            self.visible_sidebar_targets().into_iter().collect();
        self.selected_sidebar_targets
            .retain(|target| visible_targets.contains(target));

        if self
            .selection_anchor_sidebar_target
            .as_ref()
            .is_some_and(|target| !visible_targets.contains(target))
        {
            self.selection_anchor_sidebar_target = None;
        }

        if self
            .focused_sidebar_target
            .as_ref()
            .is_some_and(|target| !visible_targets.contains(target))
        {
            self.focused_sidebar_target = None;
        }
    }

    pub(crate) fn ensure_sidebar_focus(&mut self) {
        if self.focused_sidebar_target.is_some() {
            return;
        }

        self.focused_sidebar_target = self
            .selected_path
            .as_ref()
            .map(|path| SidebarTarget::File(path.clone()))
            .or_else(|| (!self.cached_rows.is_empty()).then_some(SidebarTarget::Root));
    }

    pub(crate) fn targeted_sidebar_targets(&self) -> Vec<SidebarTarget> {
        if self.has_explicit_selection() {
            self.visible_sidebar_targets()
                .into_iter()
                .filter(|target| self.selected_sidebar_targets.contains(target))
                .collect()
        } else {
            self.focused_sidebar_target.clone().into_iter().collect()
        }
    }

    pub(crate) fn file_paths_for_sidebar_target(&self, target: &SidebarTarget) -> Vec<String> {
        match target {
            SidebarTarget::Root => self.files.iter().map(|file| file.path.clone()).collect(),
            SidebarTarget::Dir(path) => self
                .files
                .iter()
                .filter(|file| is_path_within_dir(&file.path, path))
                .map(|file| file.path.clone())
                .collect(),
            SidebarTarget::File(path) => self
                .files
                .iter()
                .any(|file| file.path == *path)
                .then_some(path.clone())
                .into_iter()
                .collect(),
        }
    }

    pub(crate) fn targeted_file_paths(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut paths = Vec::new();

        for target in self.targeted_sidebar_targets() {
            for path in self.file_paths_for_sidebar_target(&target) {
                if seen.insert(path.clone()) {
                    paths.push(path);
                }
            }
        }

        paths
    }

    pub(crate) fn are_all_paths_staged(&self, paths: &[String]) -> bool {
        !paths.is_empty()
            && paths.iter().all(|path| {
                self.files
                    .iter()
                    .find(|file| file.path == *path)
                    .is_some_and(ChangedFile::is_staged)
            })
    }

    pub(crate) fn sidebar_target_is_fully_staged(&self, target: &SidebarTarget) -> bool {
        let paths = self.file_paths_for_sidebar_target(target);
        self.are_all_paths_staged(&paths)
    }

    pub(crate) fn set_status_message(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            tone,
        });
    }

    fn tree_root_name(&self) -> String {
        self.repo_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.repo_path.to_string_lossy().into_owned())
    }

    fn build_tree(&self) -> TreeDir {
        let mut root = TreeDir::root();
        for (index, file) in self.files.iter().enumerate() {
            root.insert_file(index, file);
        }
        root.sort_files_recursive();
        root.collapse_single_child_dirs();
        root
    }

    fn rebuild_cached_rows(&mut self) {
        let tree = self.build_tree();
        self.cached_rows.clear();
        self.cached_rows.push(SidebarRow::Root {
            name: self.tree_root_name(),
            expanded: self.tree_root_expanded,
        });

        if self.tree_root_expanded {
            tree.collect_visible_rows(&self.expanded_dirs, 1, &mut self.cached_rows);
        }

        self.tree_dirty = false;
    }

    pub(crate) fn all_dir_paths(&self) -> Vec<String> {
        let tree = self.build_tree();
        let mut all_dirs = Vec::new();
        tree.collect_dir_paths(&mut all_dirs);
        all_dirs
    }

    /// Find the first file index under a given directory path.
    #[allow(dead_code)] // will be called in Task 5
    pub(crate) fn first_file_index_for_dir(&self, dir_path: &str) -> Option<usize> {
        let tree = self.build_tree();
        let dir = tree.find_dir(dir_path)?;
        dir.first_file_index()
    }

    /// Find the first file index in the entire tree.
    #[allow(dead_code)] // will be called in Task 5
    pub(crate) fn first_file_index_for_root(&self) -> Option<usize> {
        let tree = self.build_tree();
        tree.first_file_index()
    }

    pub(crate) fn descendant_dir_paths(&self, path: &str) -> Vec<String> {
        self.all_dir_paths()
            .into_iter()
            .filter(|dir| is_path_within_dir(dir, path))
            .collect()
    }

    pub(crate) fn ensure_rows_cached(&mut self) {
        if self.tree_dirty {
            self.rebuild_cached_rows();
        }
    }

    pub(crate) fn sync_tree_state(&mut self) {
        let tree = self.build_tree();
        let mut all_dirs = Vec::new();
        tree.collect_dir_paths(&mut all_dirs);

        if !self.initialized_tree {
            self.expanded_dirs = all_dirs.iter().cloned().collect();
            self.tree_root_expanded = true;
            self.initialized_tree = true;
        } else {
            let known: HashSet<String> = all_dirs.into_iter().collect();
            self.expanded_dirs.retain(|path| known.contains(path));
        }

        if let Some(selected_path) = self.selected_path.as_ref() {
            self.tree_root_expanded = true;
            expand_parent_dirs(&mut self.expanded_dirs, selected_path);
        } else if let Some(selected) = self.selected_file
            && let Some(file) = self.files.get(selected)
        {
            self.tree_root_expanded = true;
            expand_parent_dirs(&mut self.expanded_dirs, &file.path);
        }

        self.tree_dirty = true;
    }

    pub(crate) fn toggle_root(&mut self, recursive: bool) {
        if self.tree_root_expanded {
            self.tree_root_expanded = false;

            if recursive {
                self.expanded_dirs.clear();
            }
        } else {
            self.tree_root_expanded = true;

            if recursive {
                self.expanded_dirs = self.all_dir_paths().into_iter().collect();
            }
        }

        self.tree_dirty = true;
    }

    pub(crate) fn toggle_dir(&mut self, path: &str, recursive: bool) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);

            if recursive {
                let prefix = format!("{path}/");
                self.expanded_dirs.retain(|dir| !dir.starts_with(&prefix));
            }
        } else {
            self.expanded_dirs.insert(path.to_owned());

            if recursive {
                self.expanded_dirs.extend(self.descendant_dir_paths(path));
            }
        }
        self.tree_dirty = true;
    }

    pub(crate) fn queue_refresh(&mut self) -> Task<Message> {
        if self.refresh_in_flight {
            self.refresh_queued = true;
            Task::none()
        } else {
            self.refresh_in_flight = true;
            self.refresh_queued = false;
            let repo = self.repo_path.clone();
            Task::perform(
                async move { load_changed_files(repo) },
                Message::FilesLoaded,
            )
        }
    }

    pub(crate) fn finish_refresh(&mut self) -> Task<Message> {
        self.refresh_in_flight = false;

        if self.refresh_queued {
            self.refresh_queued = false;
            self.queue_refresh()
        } else {
            Task::none()
        }
    }

    pub(crate) fn next_diff_request(&mut self) -> u64 {
        self.active_diff_request = self.active_diff_request.wrapping_add(1);
        self.active_diff_request
    }

    pub(crate) fn next_project_search_request(&mut self) -> u64 {
        let Some(search) = self.project_search.as_mut() else {
            return 0;
        };

        search.request_id = search.request_id.wrapping_add(1);
        search.request_id
    }

    pub(crate) fn queue_project_search(&mut self) {
        if let Some(search) = self.project_search.as_mut() {
            search.pending_run_at =
                Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
            search.searching = false;
        }
    }
}

fn is_path_within_dir(path: &str, dir: &str) -> bool {
    path.strip_prefix(dir)
        .is_some_and(|suffix| suffix.starts_with('/'))
}
