use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::jobs::{Job, Stage};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Today {
    pub setup: Setup,
    pub ready: i64,
    pub manual: i64,
    pub replies: i64,
    pub seen_24h: i64,
    pub new_24h: i64,
    pub shortlisted_24h: i64,
    pub applied_7d: i64,
    pub heard_back_7d: i64,
    pub jobs: i64,
    pub searching: bool,
    pub feed: Vec<Event>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Setup {
    pub reach: bool,
    pub cv: bool,
    pub profile: bool,
    pub ai: bool,
    pub email: bool,
}

impl Setup {
    pub fn done(&self) -> bool {
        self.reach && self.profile && self.ai
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Event {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub body: String,
    pub job_id: Option<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Log {
    pub events: Vec<Event>,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Reach {
    #[cfg_attr(feature = "ssr", sqlx(try_from = "String"))]
    pub stage: Stage,
    pub shortlisted: bool,
    pub applied: bool,
    pub screened: bool,
    pub interviewed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub flows: Vec<Flow>,
    pub active: Vec<Job>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub from: String,
    pub to: String,
    pub jobs: i64,
}

pub fn flows(reached: &[Reach]) -> Vec<Flow> {
    let mut flows: Vec<Flow> = vec![];
    let mut add = |from: &str, to: &str| match flows.iter_mut().find(|f| f.from == from && f.to == to) {
        Some(flow) => flow.jobs += 1,
        None => flows.push(Flow { from: from.into(), to: to.into(), jobs: 1 }),
    };
    for r in reached {
        let end = match r.stage {
            Stage::Rejected => "Rejected",
            Stage::Ghosted => "No reply",
            Stage::Offer => "Offer",
            Stage::Withdrawn => "Withdrawn",
            _ => "Waiting",
        };
        if !r.shortlisted {
            add(
                "Found",
                match r.stage {
                    Stage::Filtered => "Not a fit",
                    Stage::Passed => "Hidden",
                    Stage::Closed => "Closed",
                    _ => "Waiting to score",
                },
            );
            continue;
        }
        add("Found", "Shortlisted");
        if !r.applied {
            add("Shortlisted", if r.stage == Stage::Skipped { "Skipped" } else { "In review" });
            continue;
        }
        add("Shortlisted", "Applied");
        if !r.screened {
            add("Applied", end);
            continue;
        }
        add("Applied", "Screen");
        if !r.interviewed {
            add("Screen", end);
            continue;
        }
        add("Screen", "Interview");
        add("Interview", end);
    }
    flows
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Insights {
    pub weeks: Vec<Week>,
    pub bands: Vec<Band>,
    pub sources: Vec<SourceYield>,
    pub drops: Vec<(String, i64)>,
    pub gaps: Vec<(String, i64)>,
    pub median_reply_days: Option<f64>,
    pub like: Option<Learned>,
    pub odds: Option<Learned>,
    pub decisions: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Week {
    pub week: String,
    pub applied: i64,
    pub heard_back: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Band {
    pub band: String,
    pub decided: i64,
    pub moved_on: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SourceYield {
    pub source: String,
    pub found: i64,
    pub shortlisted: i64,
    pub applied: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Learned {
    pub examples: usize,
    pub accuracy: Option<f64>,
    pub trained_at: DateTime<Utc>,
}
