//! Unix socket permissions and listener binding.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

/// Socket mode: owner + group read/write, no world access.
pub const SOCKET_MODE: u32 = 0o660;
/// Runtime directory mode: owner + group traverse, no world access.
pub const RUNTIME_DIR_MODE: u32 = 0o750;

/// Ensure the parent directory exists and remove any stale socket file.
pub fn prepare_socket_path(path: &Path) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(RUNTIME_DIR_MODE))?;
    }
    let _ = fs::remove_file(path);
    Ok(())
}

/// Bind a non-blocking listener and restrict access to the telemetry group.
pub fn bind_listener(path: &Path) -> io::Result<UnixListener> {
    prepare_socket_path(path)?;
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE))?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}
