//! Git worktree management tools.
//!
//! `EnterWorktree` creates a new git worktree for isolated development on a
//! branch, changes the process working directory into it, and records state so
//! `ExitWorktree` can restore the original workspace.  Only one worktree can
//! be active at a time.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use git2::{Repository, WorktreeAddOptions};

use crate::types::{EnterWorktreeInput, ExitWorktreeInput};

// ── Active worktree state ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ActiveWorktree {
    /// Absolute path to the newly created worktree.
    path: PathBuf,
    /// Name used when creating the worktree (branch name).
    name: String,
    /// Original working directory before entering the worktree.
    original_cwd: PathBuf,
}

static ACTIVE_WORKTREE: OnceLock<Mutex<Option<ActiveWorktree>>> = OnceLock::new();

fn worktree_state() -> &'static Mutex<Option<ActiveWorktree>> {
    ACTIVE_WORKTREE.get_or_init(|| Mutex::new(None))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_repo_root(start: &Path) -> Result<PathBuf, String> {
    Repository::discover(start)
        .map(|repo| repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf())
        .map_err(|e| format!("not a git repository: {e}"))
}

// ── Tool implementations ──────────────────────────────────────────────────────

pub(crate) fn execute_enter_worktree(input: EnterWorktreeInput) -> Result<String, String> {
    let mut state = worktree_state()
        .lock()
        .map_err(|e| format!("worktree state lock poisoned: {e}"))?;

    if state.is_some() {
        return Err(
            "A worktree is already active. Call ExitWorktree first before entering another."
                .to_string(),
        );
    }

    let branch = input.branch.trim().to_string();
    if branch.is_empty() {
        return Err("branch name must not be empty".to_string());
    }

    let original_cwd =
        std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;

    let repo_root = find_repo_root(&original_cwd)?;
    let repo =
        Repository::open(&repo_root).map_err(|e| format!("cannot open git repository: {e}"))?;

    let worktree_path = if let Some(p) = &input.path {
        PathBuf::from(p)
    } else {
        repo_root.join(".worktrees").join(&branch)
    };

    if worktree_path.exists() {
        return Err(format!(
            "worktree path already exists: {}",
            worktree_path.display()
        ));
    }

    std::fs::create_dir_all(worktree_path.parent().unwrap_or(&worktree_path))
        .map_err(|e| format!("cannot create parent directory: {e}"))?;

    let mut opts = WorktreeAddOptions::new();

    // If the branch already exists, check it out; otherwise create it from HEAD.
    let branch_ref = format!("refs/heads/{branch}");
    let reference = repo.find_reference(&branch_ref).ok();
    if let Some(ref r) = reference {
        opts.reference(Some(r));
    }

    repo.worktree(&branch, &worktree_path, Some(&opts))
        .map_err(|e| format!("git worktree add failed: {e}"))?;

    std::env::set_current_dir(&worktree_path)
        .map_err(|e| format!("cannot chdir to worktree: {e}"))?;

    let path_str = worktree_path.display().to_string();
    *state = Some(ActiveWorktree {
        path: worktree_path,
        name: branch.clone(),
        original_cwd,
    });

    Ok(format!(
        "Entered worktree '{branch}' at {path_str}. \
         All file operations now target this worktree. \
         Call ExitWorktree when done."
    ))
}

pub(crate) fn execute_exit_worktree(input: ExitWorktreeInput) -> Result<String, String> {
    let mut state = worktree_state()
        .lock()
        .map_err(|e| format!("worktree state lock poisoned: {e}"))?;

    let active = state
        .take()
        .ok_or_else(|| "No worktree is currently active.".to_string())?;

    std::env::set_current_dir(&active.original_cwd)
        .map_err(|e| format!("cannot restore original directory: {e}"))?;

    let path_str = active.path.display().to_string();

    if input.cleanup.unwrap_or(false) {
        let repo_root = find_repo_root(&active.original_cwd)?;
        let repo =
            Repository::open(&repo_root).map_err(|e| format!("cannot open git repository: {e}"))?;
        if let Ok(wt) = repo.find_worktree(&active.name) {
            wt.prune(None)
                .map_err(|e| format!("worktree prune failed: {e}"))?;
        }
        if active.path.exists() {
            std::fs::remove_dir_all(&active.path)
                .map_err(|e| format!("cannot remove worktree directory: {e}"))?;
        }
        return Ok(format!(
            "Exited and cleaned up worktree '{}' ({}). \
             Restored original directory: {}",
            active.name,
            path_str,
            active.original_cwd.display()
        ));
    }

    Ok(format!(
        "Exited worktree '{}' at {}. \
         Restored original directory: {}. \
         The worktree directory is preserved; use `cleanup: true` to remove it.",
        active.name,
        path_str,
        active.original_cwd.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, OnceLock, PoisonError};

    fn worktree_test_lock() -> &'static StdMutex<()> {
        static L: OnceLock<StdMutex<()>> = OnceLock::new();
        L.get_or_init(|| StdMutex::new(()))
    }

    #[test]
    fn empty_branch_name_rejected() {
        let _g = worktree_test_lock()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let err = execute_enter_worktree(EnterWorktreeInput {
            branch: "  ".to_string(),
            path: None,
        })
        .unwrap_err();
        assert!(err.contains("must not be empty"), "got: {err}");
    }

    #[test]
    fn exit_without_enter_returns_error() {
        let _g = worktree_test_lock()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        {
            let mut state = worktree_state()
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            *state = None;
        }

        let err = execute_exit_worktree(ExitWorktreeInput { cleanup: None }).unwrap_err();
        assert!(err.contains("No worktree"), "got: {err}");
    }

    #[test]
    fn double_enter_without_exit_rejected() {
        let _g = worktree_test_lock()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        {
            let mut state = worktree_state()
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            *state = Some(ActiveWorktree {
                path: PathBuf::from("/tmp/fake"),
                name: "fake-branch".to_string(),
                original_cwd: PathBuf::from("/tmp"),
            });
        }

        let err = execute_enter_worktree(EnterWorktreeInput {
            branch: "new-branch".to_string(),
            path: None,
        })
        .unwrap_err();
        assert!(err.contains("already active"), "got: {err}");

        {
            let mut state = worktree_state()
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            *state = None;
        }
    }

    #[test]
    fn find_repo_root_in_non_git_dir_returns_error() {
        let dir = std::env::temp_dir().join(format!("codineer-wt-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let err = find_repo_root(&dir).unwrap_err();
        assert!(err.contains("not a git repository"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
