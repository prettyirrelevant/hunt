use serde::{Deserialize, Serialize};

use super::filter::region_of;
use crate::jobs::Posting;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reach {
    /// ISO 3166 alpha-2, lowercase.
    pub country: String,
    pub relocate: bool,
}

impl Reach {
    pub fn region(&self) -> &'static str {
        region_of(&self.country)
    }
}

#[derive(Clone, Debug)]
pub struct Want {
    pub terms: Vec<String>,
    pub reach: Reach,
    pub seniority: Vec<String>,
    pub days: u32,
}

impl Want {
    pub fn matches(&self, title: &str, tags: &[String]) -> bool {
        let haystack = format!("{} {}", title, tags.join(" ")).to_lowercase();
        self.terms.iter().any(|term| term.to_lowercase().split_whitespace().all(|w| haystack.contains(w)))
    }
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Keep { flags: Vec<&'static str> },
    Manual(String),
    Drop(String),
}

#[derive(Default)]
pub struct Harvest {
    pub postings: Vec<Posting>,
    pub failures: Vec<(String, String)>,
    pub sources: usize,
}
