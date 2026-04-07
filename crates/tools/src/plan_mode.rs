//! Plan-mode toggle tools.
//!
//! `EnterPlanMode` / `ExitPlanMode` switch a process-wide flag that
//! signals the assistant is in a read-only planning phase.  Other
//! subsystems can check [`is_plan_mode`] to gate write operations.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::types::{EnterPlanModeInput, ExitPlanModeInput};

/// Global plan-mode flag.  True while the model is constrained to read-only
/// planning; false (default) during normal execution.
static PLAN_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Returns `true` when plan mode is currently active.
#[must_use]
pub fn is_plan_mode() -> bool {
    PLAN_MODE_ACTIVE.load(Ordering::Relaxed)
}

pub(crate) fn execute_enter_plan_mode(_input: EnterPlanModeInput) -> Result<String, String> {
    if PLAN_MODE_ACTIVE.swap(true, Ordering::SeqCst) {
        return Ok("Plan mode was already active. No change.".to_string());
    }
    Ok(
        "Plan mode activated. All write/execute tools are now blocked until ExitPlanMode is \
         called. Use this mode to reason and draft a plan without making any changes."
            .to_string(),
    )
}

pub(crate) fn execute_exit_plan_mode(_input: ExitPlanModeInput) -> Result<String, String> {
    if !PLAN_MODE_ACTIVE.swap(false, Ordering::SeqCst) {
        return Ok("Plan mode was not active. No change.".to_string());
    }
    Ok("Plan mode deactivated. Normal execution mode restored.".to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Serializes tests that touch the process-wide [`PLAN_MODE_ACTIVE`] flag so
    /// parallel test threads cannot reset or flip it between assertions.
    static PLAN_MODE_TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn reset() {
        PLAN_MODE_ACTIVE.store(false, Ordering::SeqCst);
    }

    #[test]
    fn enter_activates_and_exit_deactivates() {
        let _guard = PLAN_MODE_TEST_MUTEX
            .lock()
            .expect("plan-mode test mutex poisoned");
        reset();
        assert!(!is_plan_mode());
        let msg = execute_enter_plan_mode(EnterPlanModeInput {}).unwrap();
        assert!(msg.contains("activated"));
        assert!(is_plan_mode());
        let msg = execute_exit_plan_mode(ExitPlanModeInput {}).unwrap();
        assert!(msg.contains("deactivated"));
        assert!(!is_plan_mode());
    }

    #[test]
    fn double_enter_is_idempotent() {
        let _guard = PLAN_MODE_TEST_MUTEX
            .lock()
            .expect("plan-mode test mutex poisoned");
        reset();
        execute_enter_plan_mode(EnterPlanModeInput {}).unwrap();
        let msg = execute_enter_plan_mode(EnterPlanModeInput {}).unwrap();
        assert!(msg.contains("already active"));
        execute_exit_plan_mode(ExitPlanModeInput {}).unwrap();
    }

    #[test]
    fn exit_without_enter_is_noop() {
        let _guard = PLAN_MODE_TEST_MUTEX
            .lock()
            .expect("plan-mode test mutex poisoned");
        reset();
        let msg = execute_exit_plan_mode(ExitPlanModeInput {}).unwrap();
        assert!(msg.contains("not active"));
    }
}
