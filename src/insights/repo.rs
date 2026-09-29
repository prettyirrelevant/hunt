use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Json};

use super::model::{Band, Event, Learned, Log, Reach, SourceYield, Today, Week};
use crate::{jobs::Job, matching::learn::Model};

pub async fn today(db: &PgPool) -> Result<Today> {
    #[allow(clippy::type_complexity)]
    let counts: (i64, i64, i64, i64, i64, i64, i64, i64, i64, bool) = sqlx::query_as(
        "select
             (select count(*) from jobs where stage = 'ready'),
             (select count(*) from jobs where stage = 'manual'),
             (select count(*) from messages where status = 'needs_you'),
             (select count(*) from jobs where last_seen > now() - interval '24 hours'),
             (select count(*) from jobs where first_seen > now() - interval '24 hours'),
             (select count(*) from events where kind = 'stage' and data ->> 'to' = 'shortlist' and at > now() - interval '24 hours'),
             (select count(*) from events where kind = 'stage' and data ->> 'to' = 'applied' and at > now() - interval '7 days'),
             (select count(*) from events where kind = 'stage'
                  and data ->> 'to' in ('screen', 'interview', 'offer', 'rejected') and at > now() - interval '7 days'),
             (select count(*) from jobs),
             exists (select 1 from graphile_worker.jobs where task_identifier = 'sweep' and locked_at is not null)",
    )
    .fetch_one(db)
    .await?;
    let feed =
        sqlx::query_as("select at, kind, body, job_id from events where kind <> 'score' order by at desc limit 12")
            .fetch_all(db)
            .await?;
    let (ready, manual, replies, seen_24h, new_24h, shortlisted_24h, applied_7d, heard_back_7d, jobs, searching) =
        counts;
    Ok(Today {
        ready,
        manual,
        replies,
        seen_24h,
        new_24h,
        shortlisted_24h,
        applied_7d,
        heard_back_7d,
        jobs,
        searching,
        feed,
        ..Default::default()
    })
}

pub async fn log(db: &PgPool, kind: &str, page: i64, per: i64) -> Result<Log> {
    let total =
        sqlx::query_scalar("select count(*) from events where $1 = '' or kind = $1").bind(kind).fetch_one(db).await?;
    let events: Vec<Event> = sqlx::query_as(
        "select at, kind, body, job_id from events where $1 = '' or kind = $1 order by at desc limit $2 offset $3",
    )
    .bind(kind)
    .bind(per)
    .bind((page - 1) * per)
    .fetch_all(db)
    .await?;
    Ok(Log { events, total })
}

pub async fn reached(db: &PgPool) -> Result<Vec<Reach>> {
    Ok(sqlx::query_as(
        "select j.stage,
                coalesce(bool_or(e.data ->> 'to' in ('shortlist', 'ready', 'approved', 'sending', 'applied', 'screen', 'interview', 'offer')), false)
                    or j.stage in ('shortlist', 'ready', 'approved', 'sending', 'applied', 'screen', 'interview', 'offer') as shortlisted,
                coalesce(bool_or(e.data ->> 'to' in ('sending', 'applied')), false)
                    or j.stage in ('sending', 'applied', 'screen', 'interview', 'offer') as applied,
                coalesce(bool_or(e.data ->> 'to' in ('screen', 'interview', 'offer')), false) as screened,
                coalesce(bool_or(e.data ->> 'to' in ('interview', 'offer')), false) as interviewed
         from jobs j
         left join events e on e.job_id = j.id and e.kind = 'stage'
         group by j.id",
    )
    .fetch_all(db)
    .await?)
}

pub async fn active(db: &PgPool) -> Result<Vec<Job>> {
    Ok(sqlx::query_as(crate::select_jobs!(
        "where stage in ('sending', 'applied', 'screen', 'interview', 'offer') order by stage_at desc limit 50"
    ))
    .fetch_all(db)
    .await?)
}

pub async fn weeks(db: &PgPool) -> Result<Vec<Week>> {
    Ok(sqlx::query_as(
        "select to_char(w, 'DD Mon') as week,
                count(e.id) filter (where e.data ->> 'to' = 'applied') as applied,
                count(e.id) filter (where e.data ->> 'to' in ('screen', 'interview', 'offer', 'rejected')) as heard_back
         from generate_series(date_trunc('week', now()) - interval '11 weeks', date_trunc('week', now()), interval '1 week') w
         left join events e on e.kind = 'stage' and date_trunc('week', e.at) = w
         group by w order by w",
    )
    .fetch_all(db)
    .await?)
}

pub async fn bands(db: &PgPool) -> Result<Vec<Band>> {
    Ok(sqlx::query_as(
        "select case when score >= 90 then '90+' when score >= 80 then '80–89' when score >= 70 then '70–79' else 'under 70' end as band,
                count(*) as decided,
                count(*) filter (where stage in ('screen', 'interview', 'offer')) as moved_on
         from jobs
         where score is not null
           and (stage in ('screen', 'interview', 'offer', 'rejected', 'ghosted')
                or (stage = 'applied' and stage_at < now() - interval '21 days'))
         group by 1 order by 1 desc",
    )
    .fetch_all(db)
    .await?)
}

pub async fn sources(db: &PgPool) -> Result<Vec<SourceYield>> {
    Ok(sqlx::query_as(
        "select source, count(*) as found,
                count(*) filter (where stage not in ('new', 'filtered', 'passed', 'closed')) as shortlisted,
                count(*) filter (where stage in ('sending', 'applied', 'screen', 'interview', 'offer', 'rejected', 'ghosted', 'withdrawn')) as applied
         from jobs group by source order by shortlisted desc, found desc limit 12",
    )
    .fetch_all(db)
    .await?)
}

pub async fn drops(db: &PgPool) -> Result<Vec<(String, i64)>> {
    Ok(sqlx::query_as(
        "select coalesce(nullif(data ->> 'why', ''), 'No reason recorded'), count(*) from events
         where kind = 'stage' and data ->> 'to' in ('filtered', 'passed')
         group by 1 order by 2 desc limit 8",
    )
    .fetch_all(db)
    .await?)
}

pub async fn gaps(db: &PgPool, known: &[String]) -> Result<Vec<(String, i64)>> {
    let known: Vec<String> = known.iter().map(|s| s.to_lowercase()).collect();
    Ok(sqlx::query_as(
        "select s, count(*) from jobs, unnest(skills) s
         where stage not in ('new', 'filtered', 'passed', 'closed') and not (lower(s) = any($1))
         group by s order by 2 desc limit 10",
    )
    .bind(&known)
    .fetch_all(db)
    .await?)
}

pub async fn median_reply_days(db: &PgPool) -> Result<Option<f64>> {
    Ok(sqlx::query_scalar(
        "select percentile_cont(0.5) within group (order by extract(epoch from reply.at - sent.at) / 86400)
         from events sent
         join lateral (
             select at from events r
             where r.job_id = sent.job_id and r.kind = 'stage' and r.at > sent.at
               and r.data ->> 'to' in ('screen', 'interview', 'offer', 'rejected')
             order by r.at limit 1
         ) reply on true
         where sent.kind = 'stage' and sent.data ->> 'to' = 'applied'",
    )
    .fetch_one(db)
    .await?)
}

pub async fn learned(db: &PgPool, name: &str) -> Result<Option<Learned>> {
    let row: Option<(Json<Model>, DateTime<Utc>)> =
        sqlx::query_as("select data, trained_at from models where name = $1").bind(name).fetch_optional(db).await?;
    Ok(row.map(|(Json(m), trained_at)| Learned { examples: m.examples, accuracy: m.accuracy, trained_at }))
}

pub async fn decisions(db: &PgPool) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "select count(*) from events where kind = 'stage' and data ->> 'to' in ('approved', 'skipped')",
    )
    .fetch_one(db)
    .await?)
}
