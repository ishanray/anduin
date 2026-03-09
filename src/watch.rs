use iced::futures::{SinkExt, Stream, StreamExt, channel::mpsc};
use iced::{Subscription, stream};
use notify::{EventKind, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Event {
    Changed,
    Error(String),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct WatchConfig {
    workdir: PathBuf,
}

pub fn subscription(repo_path: PathBuf) -> Subscription<Event> {
    let config = match watch_config(&repo_path) {
        Ok(config) => config,
        Err(_) => WatchConfig { workdir: repo_path },
    };

    Subscription::run_with(config, watch_stream)
}

#[allow(clippy::ptr_arg)]
fn watch_stream(config: &WatchConfig) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
    let config = config.clone();

    Box::pin(stream::channel(
        16,
        move |mut output: mpsc::Sender<Event>| async move {
            let (sender, mut receiver) = mpsc::unbounded();

            thread::spawn(move || run_watcher(config, sender));

            while let Some(event) = receiver.next().await {
                if output.send(event).await.is_err() {
                    break;
                }
            }
        },
    ))
}

fn run_watcher(config: WatchConfig, sender: mpsc::UnboundedSender<Event>) {
    let watch_paths = vec![(config.workdir.clone(), RecursiveMode::Recursive)];

    let (raw_tx, raw_rx) = std_mpsc::channel::<notify::Result<notify::Event>>();

    let mut watcher = match notify::recommended_watcher(move |result| {
        let _ = raw_tx.send(result);
    }) {
        Ok(watcher) => watcher,
        Err(error) => {
            let _ = sender.unbounded_send(Event::Error(error.to_string()));
            return;
        }
    };

    for (path, mode) in &watch_paths {
        if let Err(error) = watcher.watch(path, *mode) {
            let _ = sender.unbounded_send(Event::Error(format!(
                "failed to watch {}: {error}",
                path.display()
            )));
            return;
        }
    }

    let debounce = Duration::from_millis(75);

    while let Ok(result) = raw_rx.recv() {
        let mut changed = process_event(result, &config, &sender);

        while let Ok(result) = raw_rx.recv_timeout(debounce) {
            changed |= process_event(result, &config, &sender);
        }

        if changed && sender.unbounded_send(Event::Changed).is_err() {
            break;
        }
    }
}

fn process_event(
    result: notify::Result<notify::Event>,
    config: &WatchConfig,
    sender: &mpsc::UnboundedSender<Event>,
) -> bool {
    match result {
        Ok(event) => is_relevant_event(&event, config),
        Err(error) => {
            let _ = sender.unbounded_send(Event::Error(error.to_string()));
            false
        }
    }
}

fn is_relevant_event(event: &notify::Event, config: &WatchConfig) -> bool {
    is_relevant_kind(&event.kind)
        && event
            .paths
            .iter()
            .any(|path| is_relevant_path(path, &config.workdir))
}

fn is_relevant_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Other
    )
}

fn is_relevant_path(path: &Path, workdir: &Path) -> bool {
    if let Ok(relative) = path.strip_prefix(workdir) {
        let mut components = relative.components();

        if let Some(first) = components.next() {
            let first = first.as_os_str();

            if first == OsStr::new(".git") || first == OsStr::new("target") {
                return false;
            }
        }
    }

    true
}

fn watch_config(repo_path: &Path) -> Result<WatchConfig, String> {
    let repo = gix::discover(repo_path).map_err(|e| e.to_string())?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| "bare repository, no working directory".to_owned())?
        .to_path_buf();

    Ok(WatchConfig { workdir })
}
