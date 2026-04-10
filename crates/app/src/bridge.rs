//! Tokio ↔ GPUI bridge (stub until wired into `application`).
#![allow(dead_code)]

use std::sync::Arc;

/// Bridge between tokio (backend) and GPUI (UI) async runtimes.
pub struct TokioBridge {
    rt: Arc<tokio::runtime::Runtime>,
}

impl TokioBridge {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");
        Self { rt: Arc::new(rt) }
    }

    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.rt
    }

    /// Spawn a tokio future and get a handle to await it.
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(future)
    }

    /// Block on a tokio future (use sparingly, prefer spawn + notify pattern).
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }
}

impl Default for TokioBridge {
    fn default() -> Self {
        Self::new()
    }
}
