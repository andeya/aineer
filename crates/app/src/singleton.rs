use std::io::{self, Read, Write};

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
const SOCKET_NAME: &str = "aineer.sock";

#[cfg(unix)]
fn socket_path() -> PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join(SOCKET_NAME)
}

#[allow(dead_code)]
pub enum SingletonResult {
    Primary,
    Secondary { response: String },
}

/// Try to become the singleton instance. If another instance is already
/// running, send it a message and return its response.
///
/// Uses `bind` as the atomic ownership test to avoid TOCTOU races.
pub fn try_acquire(message: &str) -> io::Result<SingletonResult> {
    #[cfg(unix)]
    {
        use std::os::unix::net::{UnixListener, UnixStream};

        let path = socket_path();

        // Try to connect to an existing instance first
        if let Ok(mut stream) = UnixStream::connect(&path) {
            stream.write_all(message.as_bytes())?;
            stream.shutdown(std::net::Shutdown::Write)?;
            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            return Ok(SingletonResult::Secondary { response });
        }

        // No existing instance — remove stale socket and claim ownership
        let _ = std::fs::remove_file(&path);
        match UnixListener::bind(&path) {
            Ok(_listener) => Ok(SingletonResult::Primary),
            Err(e) => Err(e),
        }
    }

    #[cfg(not(unix))]
    {
        use std::net::{TcpListener, TcpStream};
        let addr = "127.0.0.1:18090";

        if let Ok(mut stream) = TcpStream::connect(addr) {
            stream.write_all(message.as_bytes())?;
            stream.shutdown(std::net::Shutdown::Write)?;
            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            return Ok(SingletonResult::Secondary { response });
        }

        match TcpListener::bind(addr) {
            Ok(_listener) => Ok(SingletonResult::Primary),
            Err(e) => Err(e),
        }
    }
}

/// Start the IPC listener in a background thread so secondary instances
/// can send us messages (e.g., "open <path>").
pub fn start_listener(on_message: impl Fn(String) + Send + 'static) {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;

        let path = socket_path();
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Failed to bind singleton socket: {e}");
                return;
            }
        };

        std::thread::Builder::new()
            .name("singleton-listener".to_string())
            .spawn(move || {
                for mut stream in listener.incoming().flatten() {
                    let mut msg = String::new();
                    if stream.read_to_string(&mut msg).is_ok() {
                        let _ = stream.write_all(b"ok");
                        on_message(msg);
                    }
                }
                // Cleanup on exit
                let _ = std::fs::remove_file(&path);
            })
            .ok();
    }

    #[cfg(not(unix))]
    {
        use std::net::TcpListener;

        let listener = match TcpListener::bind("127.0.0.1:18090") {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Failed to bind singleton socket: {e}");
                return;
            }
        };

        std::thread::Builder::new()
            .name("singleton-listener".to_string())
            .spawn(move || {
                for mut stream in listener.incoming().flatten() {
                    let mut msg = String::new();
                    if stream.read_to_string(&mut msg).is_ok() {
                        let _ = stream.write_all(b"ok");
                        on_message(msg);
                    }
                }
            })
            .ok();
    }
}
