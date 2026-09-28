use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::jobs::Stage;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Class {
    Screen,
    Interview,
    Offer,
    Rejection,
    /// Asks for information, such as availability or notice period.
    Info,
    Other,
}

impl Class {
    pub fn stage(self) -> Option<Stage> {
        match self {
            Class::Screen => Some(Stage::Screen),
            Class::Interview => Some(Stage::Interview),
            Class::Offer => Some(Stage::Offer),
            Class::Rejection => Some(Stage::Rejected),
            Class::Info | Class::Other => None,
        }
    }
}

/// The AI's read of one reply from a company you applied to.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Reading {
    pub class: Class,
    /// 0 to 1: how sure you are.
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Reply {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub sender: String,
    pub subject: String,
    pub snippet: String,
    pub job_id: Option<i64>,
    pub company: Option<String>,
    pub title: Option<String>,
    pub class: Option<String>,
    pub confidence: Option<f32>,
    pub status: String,
}
