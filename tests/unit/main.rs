//! Pure logic, with no database. The PII tests download their model into
//! `target/` once.

mod ats;
mod documents;
mod feeds;
mod filter;
mod learn;
mod never_read;
mod pii;
mod pipeline;
mod recordings;
mod text;

pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).expect("fixture exists")
}
