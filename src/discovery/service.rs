use std::{fmt::Write as _, sync::Arc, time::Instant};

use anyhow::Result;
use graphile_worker::{IntoTaskHandlerResult, TaskHandler, WorkerContext};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    filter::check,
    model::{Verdict, Want},
    repo,
    sources::{Source, harvest},
};
use crate::{
    app::App,
    jobs::{self, Stage},
    matching::service::Score,
    profile,
};

pub async fn sweep(app: &Arc<App>) -> Result<()> {
    let settings = app.settings().await?;
    let profile = profile::repo::profile(&app.db).await?.unwrap_or_default();
    let (Some(reach), false) = (settings.reach, profile.roles.is_empty()) else {
        let body = "Skipped the job search: finish setup so hunt knows where you can work and what you do.";
        return jobs::record(&app.db, None, "sweep", body, json!({})).await;
    };

    let started = Instant::now();
    let first_run = repo::count(&app.db).await? == 0;
    let want = Want {
        terms: profile.roles,
        reach: reach.clone(),
        seniority: profile.seniority,
        days: if first_run { 30 } else { 4 },
    };
    let harvest = harvest(&app.http, &want, &Source::all(settings.watched)).await;

    let fresh = jobs::get_many(&app.db, &jobs::add(&app.db, &harvest.postings).await?).await?;
    let ids: Vec<i64> = fresh.iter().map(|job| job.id).collect();
    let texts: Vec<String> = fresh.iter().map(jobs::Job::embedding_text).collect();
    let shared = Arc::clone(app);
    let embeddings = tokio::task::spawn_blocking(move || shared.embedder.embed_all(&texts)).await?;
    repo::save_embeddings(&app.db, &ids, &embeddings).await?;
    let companies: Vec<&str> = fresh.iter().map(|job| job.company.as_str()).collect();
    let applied = repo::applied_recently(&app.db, &companies).await?;

    let mut flags = vec![];
    let mut dropped = vec![];
    let mut scoring = vec![];
    for job in &fresh {
        if applied.contains(&job.company.to_lowercase()) {
            flags.push((job.id, "applied_recently"));
        }
        match check(job, &reach) {
            Verdict::Drop(reason) => {
                dropped.push((job.id, reason));
                continue;
            }
            Verdict::Manual(_) => flags.push((job.id, "video")),
            Verdict::Keep { flags: kept } => flags.extend(kept.into_iter().map(|flag| (job.id, flag))),
        }
        scoring.push((Score { job: job.id }, format!("score:{}", job.id)));
    }
    jobs::add_flags(&app.db, &flags).await?;
    let moves: Vec<(i64, &str)> = dropped.iter().map(|(id, reason)| (*id, reason.as_str())).collect();
    jobs::move_to(&app.db, &moves, Stage::Filtered).await?;
    let dropped = dropped.len();
    let queued = scoring.len();
    app.queue_all(scoring, None).await?;

    let failed: Vec<&str> = harvest.failures.iter().map(|(name, _)| name.as_str()).collect();
    let mut body = format!(
        "Checked {} sources in {} s. Saw {} jobs, {} new. {dropped} cannot work for where you are. {queued} go to scoring.",
        harvest.sources,
        started.elapsed().as_secs(),
        harvest.postings.len(),
        fresh.len(),
    );
    if !failed.is_empty() {
        let _ = write!(body, " Could not reach: {}.", failed.join(", "));
    }
    let failures: Vec<_> =
        harvest.failures.iter().map(|(name, error)| json!({ "source": name, "error": error })).collect();
    jobs::record(&app.db, None, "sweep", &body, json!({ "new": fresh.len(), "failures": failures })).await
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sweep;

impl TaskHandler for Sweep {
    const IDENTIFIER: &'static str = "sweep";

    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        sweep(&App::of(&ctx)).await
    }
}
