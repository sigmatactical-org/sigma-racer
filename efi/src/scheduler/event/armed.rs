//! [`Armed`].

#[allow(unused_imports)]
use super::*;

/// An armed event: fire `id` at absolute time `t_us`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Armed {
    pub id: EventId,
    pub t_us: u64,
}
