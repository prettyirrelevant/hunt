//! Pure logic, with no database.

mod ats;
mod documents;
mod feeds;
mod filter;
mod learn;
mod never_read;
mod pay;
mod pii;
mod pipeline;
mod postings;
mod recordings;
mod text;

pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).expect("fixture exists")
}
