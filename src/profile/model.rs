use serde::{Deserialize, Serialize};

/// Who you are as a candidate. hunt writes it from your CV, your repos and
/// your decisions, and you can edit it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Profile {
    /// One line, such as "Backend engineer, 6 years, Rust and Go".
    pub headline: String,
    /// Three or four sentences on strengths and the work that shows them.
    pub summary: String,
    /// Short job-search phrases, such as "backend engineer" or "rust".
    pub roles: Vec<String>,
    /// Levels to search: intern, junior, middle, senior or lead.
    pub seniority: Vec<String>,
    pub skills: Vec<String>,
    pub years: Option<u32>,
    /// Kinds of work or companies you have turned down.
    pub avoid: Vec<String>,
}

/// One piece of your work, rewritten for a CV: impact and engineering, no
/// personal data.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Work {
    /// What the project is, in a few words.
    pub title: String,
    /// Two or three sentences on the problem, what you built and the result.
    pub summary: String,
    /// Concrete results with numbers where the source gives them.
    pub highlights: Vec<String>,
    pub stack: Vec<String>,
}

/// What the candidate's own site, blog or talks show about their engineering.
#[cfg(feature = "ssr")]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SiteReading {
    /// One entry per project, article or talk about the candidate's own engineering work, most telling
    /// first. At most 8. Empty when the site shows none.
    pub work: Vec<Work>,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Note {
    pub id: i64,
    pub source: String,
    pub path: String,
    pub title: String,
    pub summary: Option<String>,
    pub facts: Option<sqlx::types::Json<Work>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
