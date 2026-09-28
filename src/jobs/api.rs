use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Job, Stage};
use crate::{insights::model::Event, ui::error::Error};

pub const JOBS_PAGE: i64 = 30;

/// The list filters, in the order the page shows them.
pub const SHOWS: [(&str, &str); 7] = [
    ("", "Worth a look"),
    ("manual", "Do it yourself"),
    ("applied", "Applied"),
    ("hidden", "Hidden"),
    ("filtered", "Not a fit"),
    ("skipped", "Skipped"),
    ("all", "Everything"),
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JobList {
    pub jobs: Vec<Job>,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDetail {
    pub job: Job,
    /// The description as sanitized HTML.
    pub html: String,
    pub record: Vec<Event>,
    /// Session videos and screenshots from the automated application.
    pub recording: Vec<String>,
}

#[server]
pub async fn get_jobs(show: String, q: String, page: i64) -> Result<JobList, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let stages: &[&str] = match show.as_str() {
        "manual" => &["manual"],
        "applied" => &["sending", "applied", "screen", "interview", "offer", "rejected", "ghosted", "withdrawn"],
        "hidden" => &["passed"],
        "filtered" => &["filtered"],
        "skipped" => &["skipped"],
        "all" => &[],
        _ => &["new", "shortlist", "ready", "approved", "manual"],
    };
    let app = expect_context::<Arc<App>>();
    let total: i64 = sqlx::query_scalar(
        "select count(*) from jobs
         where (cardinality($1::text[]) = 0 or stage = any($1)) and ($2 = '' or search @@ websearch_to_tsquery('english', $2))",
    )
    .bind(stages)
    .bind(&q)
    .fetch_one(&app.db)
    .await?;
    let jobs = sqlx::query_as(crate::select_jobs!(
        "where (cardinality($1::text[]) = 0 or stage = any($1)) and ($2 = '' or search @@ websearch_to_tsquery('english', $2))
         order by score desc nulls last, first_seen desc limit $3 offset $4"
    ))
    .bind(stages)
    .bind(&q)
    .bind(JOBS_PAGE)
    .bind((page.max(1) - 1) * JOBS_PAGE)
    .fetch_all(&app.db)
    .await?;
    Ok(JobList { jobs, total })
}

#[server]
pub async fn get_job(id: i64) -> Result<JobDetail, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    let job = super::get(&app.db, id).await?;
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&job.description));
    let record = sqlx::query_as("select at, kind, body, job_id from events where job_id = $1 order by at desc")
        .bind(id)
        .fetch_all(&app.db)
        .await?;
    let mut recording = vec![];
    if let Ok(mut entries) = tokio::fs::read_dir(app.config.documents().join(id.to_string()).join("recording")).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ext = std::path::Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or_default();
            if ext.eq_ignore_ascii_case("webm") || ext.eq_ignore_ascii_case("png") {
                recording.push(name);
            }
        }
    }
    recording.sort();
    Ok(JobDetail { job, html: ammonia::clean(&html), record, recording })
}

#[server]
pub async fn decide(id: i64, stage: Stage, why: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::{app::App, matching::service::Learn, review::service::Draft};

    let app = expect_context::<Arc<App>>();
    super::move_to(&app.db, &[(id, why.as_str())], stage).await?;
    match stage {
        Stage::Shortlist => app.queue(Draft { job: id }, format!("draft:{id}")).await?,
        Stage::Approved | Stage::Skipped => app.queue(Learn, "learn").await?,
        _ => {}
    }
    Ok(())
}
