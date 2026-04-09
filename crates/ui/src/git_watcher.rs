use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use crate::git_diff::{compute_status, GitStatus};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct GitWatcher {
    receiver: mpsc::Receiver<Arc<GitStatus>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    watched_path: PathBuf,
}

impl GitWatcher {
    #[must_use]
    pub fn start(repo_path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let watched_path = repo_path.clone();

        let thread = thread::Builder::new()
            .name(format!(
                "git-watcher-{}",
                repo_path
                    .file_name()
                    .map_or("unknown".into(), |n| n.to_string_lossy().to_string())
            ))
            .spawn(move || watcher_loop(&repo_path, &sender, &shutdown_flag))
            .ok();

        Self {
            receiver,
            shutdown,
            thread,
            watched_path,
        }
    }

    pub fn watched_path(&self) -> &Path {
        &self.watched_path
    }

    /// Switch to watching a different directory. Stops the old watcher thread
    /// and starts a new one for `new_path`.
    pub fn switch_directory(&mut self, new_path: PathBuf) {
        if self.watched_path == new_path {
            return;
        }
        self.stop();
        let new_watcher = Self::start(new_path);
        *self = new_watcher;
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<Arc<GitStatus>> {
        let mut latest = None;
        while let Ok(status) = self.receiver.try_recv() {
            latest = Some(status);
        }
        latest
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for GitWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

fn watcher_loop(repo_path: &Path, sender: &mpsc::Sender<Arc<GitStatus>>, shutdown: &AtomicBool) {
    let index_path = resolve_git_index_path(repo_path);
    let mut last_mtime: Option<SystemTime> = None;

    if let Ok(status) = compute_status(repo_path) {
        last_mtime = index_path.as_deref().and_then(file_mtime);
        let _ = sender.send(Arc::new(status));
    }

    loop {
        thread::sleep(POLL_INTERVAL);

        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        let current_mtime = index_path.as_deref().and_then(file_mtime);

        let changed = match (last_mtime, current_mtime) {
            (Some(prev), Some(curr)) => prev != curr,
            (None, Some(_)) => true,
            _ => false,
        };

        if !changed {
            continue;
        }

        if let Ok(status) = compute_status(repo_path) {
            last_mtime = current_mtime;
            if sender.send(Arc::new(status)).is_err() {
                break;
            }
        }
    }
}

fn resolve_git_index_path(repo_path: &Path) -> Option<PathBuf> {
    if let Ok(repo) = git2::Repository::discover(repo_path) {
        Some(repo.path().join("index"))
    } else {
        let candidate = repo_path.join(".git/index");
        candidate.exists().then_some(candidate)
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}
