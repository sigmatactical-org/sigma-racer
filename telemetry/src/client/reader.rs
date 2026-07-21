//! Background reader thread for the telemetry Unix socket.

use super::line_pump::{READ_TIMEOUT, RECONNECT_DELAY, read_stream};
use super::outcome::Outcome;
use crate::protocol::Message;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread;

/// Apply the read timeout that lets the thread observe shutdown requests.
pub(super) fn configure(stream: &UnixStream) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
}

/// Spawn the auto-reconnecting Unix-socket reader thread.
pub(super) fn spawn_reader(
    path: PathBuf,
    initial: Option<UnixStream>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || run(path, initial, tx, alive))
}

/// Connect (or adopt `initial`), pump frames, and reconnect until dropped.
fn run(
    path: PathBuf,
    initial: Option<UnixStream>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) {
    let mut stream = initial;
    while alive.load(Ordering::Relaxed) {
        let connection = match stream.take() {
            Some(s) => s,
            None => match UnixStream::connect(&path) {
                Ok(s) => {
                    configure(&s);
                    s
                }
                Err(_) => {
                    thread::sleep(RECONNECT_DELAY);
                    continue;
                }
            },
        };

        match read_stream(connection, &tx, &alive) {
            Outcome::Stop => return,
            Outcome::Reconnect => {
                if alive.load(Ordering::Relaxed) {
                    thread::sleep(RECONNECT_DELAY);
                }
            }
        }
    }
}
