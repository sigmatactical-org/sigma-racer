//! Small I/O building blocks shared by the relay and its producers.

mod backlog_writer;

pub use backlog_writer::{BacklogWriter, MAX_BACKLOG};
