#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod actions;
mod app;
mod config;
mod git;
#[path = "../vendor/lucide/mod.rs"]
mod lucide;
mod search;
mod shortcuts;
mod theme;
mod tree;
mod update;
mod views;
mod watch;

#[cfg(test)]
mod tests;

use actions::{fetch_current_branch, load_changed_files};
use app::{ActivePane, ChangesFocus, HistoryFocus, Message, SidebarTab, State, ThemeMode};
use iced::event as iced_event;
use iced::time;
use iced::widget::{Id, Stack, column, container, row, text};
use iced::window;
use iced::{Element, Fill, Font, Subscription, Task};
use iced_code_editor::{CodeEditor, Message as EditorMessage, theme as editor_theme};
use lucide::LUCIDE_FONT_BYTES;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use update::update;
use views::actions_footer::view_actions_footer;
use views::context_menu::view_context_menu;
use views::diff::view_diff;
use views::discard_confirm::view_discard_confirm;
use views::sidebar::view_sidebar;

const MONO: Font = Font::MONOSPACE;
const TREE_INDENT: f32 = 14.0;
/// Height of a single sidebar row (content + vertical padding + spacing).
const SIDEBAR_ROW_HEIGHT: f32 = 24.0;
/// Estimated height of a single commit list row (two text lines + padding + spacing).
const COMMIT_ROW_HEIGHT: f32 = 48.0;
/// Shared height for the top header bars across panels.
const PANEL_HEADER_HEIGHT: f32 = 48.0;

fn main() -> iced::Result {
    let foreground = env::args().any(|a| a == "--foreground" || a == "-f");

    if should_detach_from_terminal() && !foreground {
        detach_from_terminal();
    }

    let mut app = iced::application(boot, update, view)
        .title("Anduin")
        .font(LUCIDE_FONT_BYTES)
        .theme(|state: &State| state.app_theme())
        .subscription(subscription)
        .exit_on_close_request(false);

    // Restore saved window size if available
    if let Ok(settings) = config::load_settings()
        && let (Some(w), Some(h)) = (settings.window_width, settings.window_height)
    {
        app = app.window_size(iced::Size::new(w, h));
    }

    app.run()
}

fn should_detach_from_terminal() -> bool {
    cfg!(target_os = "linux")
}

fn detach_from_terminal() {
    #[cfg(target_os = "linux")]
    {
        use fork::{Fork, daemon};

        match daemon(true, false) {
            Ok(Fork::Parent(_)) => {
                std::process::exit(0);
            }
            Ok(Fork::Child) => {}
            Err(_) => {
                eprintln!("[anduin] warning: fork() failed, running in foreground");
            }
        }
    }
}

fn boot() -> (State, Task<Message>) {
    let settings = match config::load_settings() {
        Ok(s) => s,
        Err(error) => {
            eprintln!("[anduin] warning: failed to load settings: {error}");
            config::Settings::default()
        }
    };

    let repo_path = settings
        .repo_path
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .and_then(|p| git::diff::find_repo_root(&p).ok())
        .unwrap_or_else(|| {
            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            git::diff::find_repo_root(&cwd).unwrap_or(cwd)
        });

    let mut recent_repos = settings.recent_repos;
    let repo_str = repo_path.to_string_lossy().into_owned();
    recent_repos.retain(|p| p != &repo_str);
    recent_repos.insert(0, repo_str);
    recent_repos.truncate(20);

    eprintln!("[anduin] boot repo_path={}", repo_path.display(),);

    let settings_window_size = match (settings.window_width, settings.window_height) {
        (Some(w), Some(h)) => Some(iced::Size::new(w, h)),
        _ => None,
    };

    let theme_mode = ThemeMode::from_preference(settings.theme);

    let mut diff_editor = CodeEditor::new("", "diff");
    diff_editor.set_theme(editor_theme::from_iced_theme(&theme_mode.app_theme()));
    diff_editor.set_font(MONO);
    diff_editor.set_font_size(13.0, true);
    diff_editor.set_smooth_scroll_enabled(true);
    diff_editor.lose_focus();

    let cached_theme = theme_mode.app_theme();
    let state = State {
        repo_path: repo_path.clone(),
        files: Vec::new(),
        selected_file: None,
        selected_path: None,
        focused_sidebar_target: None,
        selected_sidebar_targets: HashSet::new(),
        selection_anchor_sidebar_target: None,
        current_diff: None,
        diff_editor,
        theme_mode,
        error: None,
        status_message: None,
        commit_composer: None,
        expanded_dirs: HashSet::new(),
        tree_root_expanded: true,
        alt_pressed: false,
        initialized_tree: false,
        cached_rows: Vec::new(),
        tree_dirty: true,
        refresh_in_flight: true,
        refresh_queued: false,
        active_diff_request: 0,
        diff_search_cache: HashMap::new(),
        scroll_positions: HashMap::new(),
        project_search: None,
        pending_diff_jump: None,
        sidebar_scroll_id: Id::unique(),
        sidebar_scroll_offset: 0.0,
        sidebar_viewport_height: 0.0,
        active_pane: ActivePane::Sidebar,
        cached_theme,
        show_actions_panel: false,
        current_branch: None,
        branch_picker: None,
        project_picker: None,
        recent_repos,
        sidebar_tab: SidebarTab::Changes,
        commits: Vec::new(),
        selected_commit: None,
        commit_files: Vec::new(),
        commits_loading: false,
        commits_exhausted: false,
        history_selected_file: None,
        history_selected_path: None,
        history_diff: None,
        history_commit_header: None,
        history_focus: HistoryFocus::CommitList,
        changes_focus: ChangesFocus::FileList,
        commit_list_scroll_id: Id::unique(),
        commit_list_scroll_offset: 0.0,
        commit_list_viewport_height: 0.0,
        discard_confirm: None,
        sidebar_context_menu: None,
        window_size: settings_window_size,
        pending_settings_save: None,
    };

    let branch_task = {
        let repo = repo_path.clone();
        Task::perform(
            async move { fetch_current_branch(repo) },
            Message::CurrentBranchFetched,
        )
    };

    let task = Task::perform(
        async move { load_changed_files(repo_path) },
        Message::FilesLoaded,
    );
    (state, Task::batch([task, branch_task]))
}

fn subscription(state: &State) -> Subscription<Message> {
    let mut subscriptions = vec![
        iced_event::listen_with(|event, _status, _window| match event {
            iced::Event::Keyboard(kb_event) => Some(Message::KeyboardEvent(kb_event)),
            iced::Event::Window(window::Event::Resized(size)) => Some(Message::WindowResized(size)),
            iced::Event::Window(window::Event::CloseRequested) => {
                Some(Message::WindowCloseRequested)
            }
            _ => None,
        }),
        watch::subscription(state.repo_path.clone()).map(Message::WatchEvent),
    ];

    if state.current_diff.is_some()
        && let Some(interval) = state.diff_editor.tick_interval()
    {
        subscriptions.push(time::every(interval).map(|_| Message::DiffEditor(EditorMessage::Tick)));
    }

    if state
        .project_search
        .as_ref()
        .is_some_and(|search| search.pending_run_at.is_some())
    {
        subscriptions
            .push(time::every(Duration::from_millis(50)).map(|_| Message::ProjectSearchTick));
    }

    if state.pending_settings_save.is_some() {
        subscriptions
            .push(time::every(Duration::from_millis(100)).map(|_| Message::SettingsSaveTick));
    }

    Subscription::batch(subscriptions)
}

fn view(state: &State) -> Element<'_, Message> {
    if let Some(ref err) = state.error {
        return container(
            iced::widget::column![
                text("Error").size(20),
                text(err.clone()).size(14).font(MONO)
            ]
            .spacing(10),
        )
        .padding(20)
        .into();
    }

    let sidebar = view_sidebar(state);
    let diff_view = view_diff(state);

    let main_content: Element<'_, Message> = row![
        container(sidebar).width(320),
        container(diff_view).width(Fill)
    ]
    .height(Fill)
    .into();

    let content_with_footer: Element<'_, Message> =
        column![main_content, view_actions_footer(state)]
            .height(Fill)
            .into();

    if state.discard_confirm.is_some() {
        let overlay = view_discard_confirm(state);
        Stack::new()
            .push(content_with_footer)
            .push(overlay)
            .width(Fill)
            .height(Fill)
            .into()
    } else if state.sidebar_context_menu.is_some() {
        let overlay = view_context_menu(state);
        Stack::new()
            .push(content_with_footer)
            .push(overlay)
            .width(Fill)
            .height(Fill)
            .into()
    } else {
        content_with_footer
    }
}
