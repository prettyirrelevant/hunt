use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::error::Error;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Nav {
    pub ready: i64,
    pub manual: i64,
    pub replies: i64,
    pub waiting: i64,
    pub ai: String,
    pub ai_ready: bool,
    pub last_sweep: Option<DateTime<Utc>>,
    pub last_backup: Option<DateTime<Utc>>,
}

#[server]
pub async fn get_nav() -> Result<Nav, Error> {
    use std::sync::Arc;

    use crate::{app::App, common::ai::Status};

    let app = expect_context::<Arc<App>>();
    let (ready, manual, replies, waiting, last_sweep, last_backup): (i64, i64, i64, i64, _, _) = sqlx::query_as(
        "select
             (select count(*) from jobs where stage = 'ready'),
             (select count(*) from jobs where stage = 'manual'),
             (select count(*) from messages where status = 'needs_you'),
             (select count(*) from graphile_worker.jobs where locked_at is null),
             (select max(at) from events where kind = 'sweep'),
             (select max(at) from events where kind = 'system')",
    )
    .fetch_one(&app.db)
    .await?;
    let providers = app.settings().await?.providers;
    let ready_ai = providers.iter().find(|p| matches!(app.ai.status(**p), Status::Ready));
    let ai = match ready_ai {
        Some(p) => p.name().to_string(),
        None if providers.is_empty() => "none installed".into(),
        None => "all resting".into(),
    };
    Ok(Nav { ready, manual, replies, waiting, ai, ai_ready: ready_ai.is_some(), last_sweep, last_backup })
}
