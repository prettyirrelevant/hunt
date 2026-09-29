use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    New,
    Filtered,
    Passed,
    Shortlist,
    Ready,
    Approved,
    Sending,
    Applied,
    Screen,
    Interview,
    Offer,
    Rejected,
    Ghosted,
    Skipped,
    Closed,
    Manual,
    Withdrawn,
}

impl Stage {
    pub const ALL: [Stage; 17] = [
        Stage::New,
        Stage::Filtered,
        Stage::Passed,
        Stage::Shortlist,
        Stage::Ready,
        Stage::Approved,
        Stage::Sending,
        Stage::Applied,
        Stage::Screen,
        Stage::Interview,
        Stage::Offer,
        Stage::Rejected,
        Stage::Ghosted,
        Stage::Skipped,
        Stage::Closed,
        Stage::Manual,
        Stage::Withdrawn,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::New => "new",
            Stage::Filtered => "filtered",
            Stage::Passed => "passed",
            Stage::Shortlist => "shortlist",
            Stage::Ready => "ready",
            Stage::Approved => "approved",
            Stage::Sending => "sending",
            Stage::Applied => "applied",
            Stage::Screen => "screen",
            Stage::Interview => "interview",
            Stage::Offer => "offer",
            Stage::Rejected => "rejected",
            Stage::Ghosted => "ghosted",
            Stage::Skipped => "skipped",
            Stage::Closed => "closed",
            Stage::Manual => "manual",
            Stage::Withdrawn => "withdrawn",
        }
    }

    pub fn parse(s: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|stage| stage.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Stage::New => "New",
            Stage::Filtered => "Not a fit",
            Stage::Passed => "Hidden",
            Stage::Shortlist => "Shortlisted",
            Stage::Ready => "Ready",
            Stage::Approved => "Approved",
            Stage::Sending => "Sending",
            Stage::Applied => "Applied",
            Stage::Screen => "Screen",
            Stage::Interview => "Interview",
            Stage::Offer => "Offer",
            Stage::Rejected => "Rejected",
            Stage::Ghosted => "No reply",
            Stage::Skipped => "Skipped",
            Stage::Closed => "Closed",
            Stage::Manual => "Do it yourself",
            Stage::Withdrawn => "Withdrawn",
        }
    }

    pub fn tone(self) -> &'static str {
        match self {
            Stage::Offer => "ok",
            Stage::Screen | Stage::Interview | Stage::Ready => "acc",
            Stage::Rejected => "crit",
            Stage::Ghosted | Stage::Manual => "warn",
            _ => "",
        }
    }

    pub fn is_applied(self) -> bool {
        matches!(
            self,
            Stage::Sending
                | Stage::Applied
                | Stage::Screen
                | Stage::Interview
                | Stage::Offer
                | Stage::Rejected
                | Stage::Ghosted
                | Stage::Withdrawn
        )
    }
}

impl TryFrom<String> for Stage {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        Stage::parse(&s).ok_or(s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkMode {
    Remote,
    Hybrid,
    Onsite,
    #[default]
    Unknown,
}

impl WorkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkMode::Remote => "remote",
            WorkMode::Hybrid => "hybrid",
            WorkMode::Onsite => "onsite",
            WorkMode::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> WorkMode {
        match s.to_lowercase().replace(['-', ' '], "_").as_str() {
            "remote" => WorkMode::Remote,
            "hybrid" => WorkMode::Hybrid,
            "onsite" | "office" | "on_site" | "in_office" => WorkMode::Onsite,
            _ => WorkMode::Unknown,
        }
    }
}

impl From<String> for WorkMode {
    fn from(s: String) -> Self {
        WorkMode::parse(&s)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
    pub text: String,
    pub required: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Posting {
    /// Stable per source, such as `greenhouse:vercel:123`.
    pub key: String,
    pub source: String,
    pub url: String,
    pub apply_url: String,
    pub company: String,
    pub title: String,
    pub location: String,
    pub work_mode: WorkMode,
    pub regions: Vec<String>,
    pub countries: Vec<String>,
    pub visa: Option<bool>,
    pub relocation: Option<bool>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub currency: Option<String>,
    pub seniority: Option<String>,
    pub skills: Vec<String>,
    pub requirements: Vec<Requirement>,
    pub description: String,
    pub posted_at: Option<DateTime<Utc>>,
}

impl Posting {
    pub fn new(source: &str, id: impl std::fmt::Display, company: &str, title: &str, url: &str) -> Posting {
        Posting {
            key: format!("{source}:{id}"),
            source: source.into(),
            url: url.into(),
            apply_url: url.into(),
            company: company.trim().into(),
            title: title.trim().into(),
            ..Default::default()
        }
    }

    /// Links reach a page and the form agent, so a `javascript:` link must never pass.
    pub fn with_web_links(mut self) -> Option<Posting> {
        let web = |link: &str| link.starts_with("https://") || link.starts_with("http://");
        if !web(&self.apply_url) {
            self.apply_url.clone_from(&self.url);
        }
        web(&self.url).then_some(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Job {
    pub id: i64,
    pub source: String,
    pub url: String,
    pub apply_url: String,
    pub company: String,
    pub title: String,
    pub location: String,
    #[cfg_attr(feature = "ssr", sqlx(try_from = "String"))]
    pub work_mode: WorkMode,
    pub regions: Vec<String>,
    pub countries: Vec<String>,
    pub visa: Option<bool>,
    pub relocation: Option<bool>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub currency: Option<String>,
    pub seniority: Option<String>,
    pub skills: Vec<String>,
    #[cfg_attr(feature = "ssr", sqlx(json))]
    pub requirements: Vec<Requirement>,
    pub description: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub first_seen: DateTime<Utc>,
    #[cfg_attr(feature = "ssr", sqlx(try_from = "String"))]
    pub stage: Stage,
    pub stage_at: DateTime<Utc>,
    pub fit: Option<i32>,
    pub like_score: Option<f32>,
    pub odds: Option<f32>,
    pub score: Option<i32>,
    pub assessment: Option<Value>,
    pub flags: Vec<String>,
    pub apply_via: Option<String>,
}

#[macro_export]
macro_rules! select_jobs {
    ($rest:literal) => {
        concat!(
            "select id, source, url, apply_url, company, title, location, work_mode, regions, countries, ",
            "visa, relocation, salary_min, salary_max, currency, seniority, skills, requirements, description, ",
            "posted_at, first_seen, stage, stage_at, fit, like_score, odds, score, assessment, flags, apply_via ",
            "from jobs ",
            $rest
        )
    };
}

impl Job {
    pub fn embedding_text(&self) -> String {
        let mut text = format!("{} at {}. {}. ", self.title, self.company, self.skills.join(", "));
        text.extend(self.description.chars().take(1500));
        text
    }

    pub fn salary(&self) -> Option<String> {
        let k = |n: i64| {
            if n >= 1000 { format!("{}k", n / 1000) } else { n.to_string() }
        };
        let range = match (self.salary_min, self.salary_max) {
            (Some(a), Some(b)) if a != b => format!("{}–{}", k(a), k(b)),
            (Some(a), _) | (None, Some(a)) => k(a),
            _ => return None,
        };
        Some(format!("{} {range}", self.currency.as_deref().unwrap_or("")).trim().to_string())
    }

    pub fn where_label(&self) -> String {
        match self.work_mode {
            WorkMode::Remote if self.regions.iter().any(|r| r == "global") => "Remote, worldwide".into(),
            WorkMode::Remote if !self.location.is_empty() => format!("Remote · {}", self.location),
            WorkMode::Remote => "Remote".into(),
            _ if self.location.is_empty() => "Location not stated".into(),
            WorkMode::Hybrid => format!("Hybrid · {}", self.location),
            _ => self.location.clone(),
        }
    }
}

pub fn flag_label(flag: &str) -> &str {
    match flag {
        "reposted" => "Reposted",
        "video" => "Asks for a video",
        "stale" => "Posted over 30 days ago",
        "no_salary" => "No salary listed",
        "applied_recently" => "Applied here recently",
        other => other,
    }
}
