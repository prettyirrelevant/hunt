pub mod api;
mod model;
#[cfg(feature = "ssr")]
mod repo;
pub mod views;

pub use model::{Job, Posting, Requirement, Stage, WorkMode, flag_label};
#[cfg(feature = "ssr")]
pub use repo::{add, add_flags, get, get_many, move_from, move_to, recent_decisions, record};
