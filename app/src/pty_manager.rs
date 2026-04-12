use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct PtyOutputEvent {
    pub id: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, Serialize)]
pub struct PtyExitEvent {
    pub id: u64,
    pub exit_code: Option<u32>,
}

struct PtySession {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

pub struct PtyManager {
    sessions: Mutex<HashMap<u64, Arc<PtySession>>>,
    next_id: AtomicU64,
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

fn build_command(
    shell: &Option<String>,
    command: &Option<String>,
    cwd: &Option<String>,
) -> CommandBuilder {
    let mut cmd = if let Some(command) = command {
        let shell_path = shell.clone().unwrap_or_else(|| {
            std::env::var("SHELL").unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    "cmd".to_string()
                } else {
                    "/bin/sh".to_string()
                }
            })
        });
        let mut c = CommandBuilder::new(&shell_path);
        if cfg!(target_os = "windows") {
            c.arg("/C");
        } else {
            c.arg("-c");
        }
        c.arg(command);
        c
    } else if let Some(sh) = shell {
        CommandBuilder::new(sh)
    } else {
        CommandBuilder::new_default_prog()
    };

    if let Some(dir) = cwd {
        let path = std::path::PathBuf::from(dir);
        if path.is_dir() {
            cmd.cwd(path);
        }
    }

    cmd.env("TERM", "xterm-256color");
    cmd
}

fn open_pty(cols: u16, rows: u16) -> Result<portable_pty::PtyPair, String> {
    let pty_system = native_pty_system();
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    pty_system
        .openpty(size)
        .map_err(|e| format!("Failed to open PTY: {e}"))
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn spawn(
        &self,
        app_handle: tauri::AppHandle,
        shell: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<u64, String> {
        let pair = open_pty(cols, rows)?;
        let cmd = build_command(&shell, &command, &cwd);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {e}"))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {e}"))?;

        let killer = child.clone_killer();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let session = Arc::new(PtySession {
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            killer: Mutex::new(killer),
            child: Mutex::new(child),
        });

        self.sessions.lock().unwrap().insert(id, session.clone());
        self.start_reader(id, reader, app_handle.clone(), &session);
        self.start_waiter(id, app_handle, &session);

        tracing::info!("PTY spawned: id={id}");
        Ok(id)
    }

    fn start_reader(
        &self,
        id: u64,
        mut reader: Box<dyn Read + Send>,
        app: tauri::AppHandle,
        _session: &Arc<PtySession>,
    ) {
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = app.emit(
                            "pty_output",
                            PtyOutputEvent {
                                id,
                                data: buf[..n].to_vec(),
                            },
                        );
                    }
                    Err(e) => {
                        tracing::debug!("PTY {id} read error: {e}");
                        break;
                    }
                }
            }
            tracing::debug!("PTY {id} reader loop ended");
        });
    }

    fn start_waiter(&self, id: u64, app: tauri::AppHandle, session: &Arc<PtySession>) {
        let session = Arc::downgrade(session);
        std::thread::spawn(move || {
            let exit_code = if let Some(sess) = session.upgrade() {
                match sess.child.lock().unwrap().wait() {
                    Ok(status) => {
                        if status.success() {
                            Some(0u32)
                        } else {
                            Some(1u32)
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            let _ = app.emit("pty_exit", PtyExitEvent { id, exit_code });
            tracing::info!("PTY {id} child exited: {exit_code:?}");
        });
    }

    pub fn write(&self, id: u64, data: &[u8]) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.lock().unwrap();
            sessions
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("PTY session {id} not found"))?
        };
        let mut writer = session.writer.lock().unwrap();
        writer
            .write_all(data)
            .map_err(|e| format!("PTY write error: {e}"))
    }

    pub fn resize(&self, id: u64, cols: u16, rows: u16) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.lock().unwrap();
            sessions
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("PTY session {id} not found"))?
        };
        let master = session.master.lock().unwrap();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("PTY resize error: {e}"))
    }

    pub fn kill(&self, id: u64) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.remove(&id) {
            let _ = session.killer.lock().unwrap().kill();
            tracing::info!("PTY {id} killed");
            Ok(())
        } else {
            Err(format!("PTY session {id} not found"))
        }
    }

    pub fn shutdown_all(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        for (id, session) in sessions.drain() {
            let _ = session.killer.lock().unwrap().kill();
            tracing::info!("PTY {id} shutdown");
        }
    }
}
